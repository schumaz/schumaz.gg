import { useEffect, useMemo, useRef, useState } from "react";
import type { BootstrapStatus } from "../types";
import { isTauriWindowAvailable, windowIpc } from "../services/ipc/window.ipc";

export type StartupStageKey =
  | "initializing"
  | "settings"
  | "cache"
  | "client"
  | "assets"
  | "syncing"
  | "ready";

export const MIN_SPLASH_DURATION_MS = 650;
export const NETWORK_WAIT_AFTER_CORE_MS = 2200;
export const ASSETS_PRELOAD_TIMEOUT_MS = 1500;
export const FONTS_READY_TIMEOUT_MS = 800;
export const MAX_SPLASH_TIMEOUT_MS = 4500;
export const EXIT_COMPLETE_BEAT_MS = 150;
export const EXIT_ANIMATION_MS = 280;

export interface UseAppBootstrapOptions {
  enabled: boolean;
  preferencesReady: boolean;
  lcuInitialChecked: boolean;
  profileInitialReady: boolean;
  criticalImageUrls: string[];
  reducedMotion?: boolean;
}

export interface UseAppBootstrapResult {
  isBooting: boolean;
  isExiting: boolean;
  progress: number;
  stageKey: StartupStageKey;
}

function isValidBootstrapStatus(value: unknown): value is BootstrapStatus {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<BootstrapStatus>;
  return (
    typeof candidate.stage === "string" &&
    typeof candidate.progress === "number" &&
    typeof candidate.diskReady === "boolean" &&
    typeof candidate.networkReady === "boolean"
  );
}

function preloadImage(url: string): Promise<void> {
  if (!url || typeof Image === "undefined") {
    return Promise.resolve();
  }
  return new Promise<void>((resolve) => {
    const img = new Image();
    let settled = false;
    const finish = () => {
      if (settled) return;
      settled = true;
      img.onload = null;
      img.onerror = null;
      resolve();
    };
    img.onload = finish;
    img.onerror = finish;
    img.src = url;
    if (img.complete) {
      finish();
    }
  });
}

export function useAppBootstrap({
  enabled,
  preferencesReady,
  lcuInitialChecked,
  profileInitialReady,
  criticalImageUrls,
  reducedMotion = false,
}: UseAppBootstrapOptions): UseAppBootstrapResult {
  const [isBooting, setIsBooting] = useState<boolean>(enabled);
  const [isExiting, setIsExiting] = useState<boolean>(false);
  const [backendStatus, setBackendStatus] = useState<BootstrapStatus>(() =>
    isTauriWindowAvailable()
      ? { stage: "initializing", progress: 0, diskReady: false, networkReady: false }
      : { stage: "ready", progress: 100, diskReady: true, networkReady: true }
  );
  const [fontsReady, setFontsReady] = useState<boolean>(!enabled);
  const [assetsReady, setAssetsReady] = useState<boolean>(!enabled);
  const [minDurationElapsed, setMinDurationElapsed] = useState<boolean>(!enabled);
  const [networkGraceElapsed, setNetworkGraceElapsed] = useState<boolean>(!enabled);
  const [hardTimedOut, setHardTimedOut] = useState<boolean>(false);
  const [maxProgress, setMaxProgress] = useState<number>(enabled ? 8 : 100);

  const completionStartedRef = useRef<boolean>(!enabled);

  // 1. Subscribe to backend bootstrap progress & fetch current snapshot
  useEffect(() => {
    if (!enabled) return;

    if (!isTauriWindowAvailable()) {
      setBackendStatus({ stage: "ready", progress: 100, diskReady: true, networkReady: true });
      return;
    }

    let active = true;
    let unlisten: (() => void) | undefined;

    const applyStatus = (next: BootstrapStatus) => {
      if (!active) return;
      setBackendStatus((prev) => ({
        stage: next.progress >= prev.progress ? next.stage : prev.stage,
        progress: Math.max(prev.progress, Math.min(100, next.progress)),
        diskReady: prev.diskReady || next.diskReady,
        networkReady: prev.networkReady || next.networkReady,
      }));
    };

    void windowIpc.onBootstrapProgress((status) => {
      if (isValidBootstrapStatus(status)) {
        applyStatus(status);
      }
    }).then((fn) => {
      if (!active) {
        fn();
      } else {
        unlisten = fn;
      }
    });

    void windowIpc
      .getBootstrapStatus()
      .then((status) => {
        if (!active) return;
        if (isValidBootstrapStatus(status)) {
          applyStatus(status);
        } else {
          // Graceful fallback if command returns null/unmocked payload
          applyStatus({ stage: "ready", progress: 100, diskReady: true, networkReady: true });
        }
      })
      .catch(() => {
        if (active) {
          applyStatus({ stage: "ready", progress: 100, diskReady: true, networkReady: true });
        }
      });

    return () => {
      active = false;
      unlisten?.();
    };
  }, [enabled]);

  // 2. Minimum splash duration + hard global safety timeout
  useEffect(() => {
    if (!enabled) return;
    const minTimer = window.setTimeout(() => {
      setMinDurationElapsed(true);
    }, MIN_SPLASH_DURATION_MS);
    const maxTimer = window.setTimeout(() => {
      setHardTimedOut(true);
    }, MAX_SPLASH_TIMEOUT_MS);

    return () => {
      window.clearTimeout(minTimer);
      window.clearTimeout(maxTimer);
    };
  }, [enabled]);

  // 3. Wait for document fonts (bounded by short timeout)
  useEffect(() => {
    if (!enabled) return;
    let active = true;
    const finish = () => {
      if (active) setFontsReady(true);
    };
    const timeout = window.setTimeout(finish, FONTS_READY_TIMEOUT_MS);
    const fontSet = typeof document !== "undefined" ? document.fonts : undefined;
    if (fontSet && typeof fontSet.ready?.then === "function") {
      void fontSet.ready.then(finish, finish);
    } else {
      finish();
    }
    return () => {
      active = false;
      window.clearTimeout(timeout);
    };
  }, [enabled]);

  // 4. Preload critical UI images (re-evaluated once preferences/profile hydrate splash URLs)
  const urlsKey = useMemo(
    () =>
      Array.from(new Set(criticalImageUrls.filter((u) => typeof u === "string" && u.length > 0))).join(
        "|"
      ),
    [criticalImageUrls]
  );

  useEffect(() => {
    if (!enabled || assetsReady) return;
    // Wait until initial profile cache check resolves so any cached splash URL is included
    if (!profileInitialReady && !hardTimedOut) return;

    let active = true;
    const finish = () => {
      if (active) setAssetsReady(true);
    };
    const timeout = window.setTimeout(finish, ASSETS_PRELOAD_TIMEOUT_MS);
    const urls = urlsKey ? urlsKey.split("|") : [];

    if (urls.length === 0) {
      finish();
    } else {
      void Promise.all(urls.map(preloadImage)).then(finish, finish);
    }

    return () => {
      active = false;
      window.clearTimeout(timeout);
    };
  }, [enabled, assetsReady, profileInitialReady, hardTimedOut, urlsKey]);

  const coreReady =
    backendStatus.diskReady &&
    preferencesReady &&
    lcuInitialChecked &&
    profileInitialReady &&
    fontsReady &&
    assetsReady;

  // 5. Once core disk/frontend state is ready, cap network wait so offline users never hang
  useEffect(() => {
    if (!enabled || !coreReady || backendStatus.networkReady || networkGraceElapsed) return;
    const timer = window.setTimeout(() => {
      setNetworkGraceElapsed(true);
    }, NETWORK_WAIT_AFTER_CORE_MS);
    return () => window.clearTimeout(timer);
  }, [enabled, coreReady, backendStatus.networkReady, networkGraceElapsed]);

  const allReady =
    !enabled ||
    hardTimedOut ||
    (coreReady && minDurationElapsed && (backendStatus.networkReady || networkGraceElapsed));

  // Compute monotonic progress percentage
  const computedProgress = useMemo(() => {
    if (!enabled || allReady) return 100;
    const backendPart = backendStatus.networkReady
      ? 45
      : backendStatus.diskReady
      ? 36
      : Math.round((Math.min(backendStatus.progress, 80) / 80) * 36);
    const prefsPart = preferencesReady ? 15 : 0;
    const lcuPart = lcuInitialChecked ? 10 : 0;
    const profilePart = profileInitialReady ? 10 : 0;
    const fontsPart = fontsReady ? 5 : 0;
    const assetsPart = assetsReady ? 15 : 0;
    const raw = Math.max(8, backendPart + prefsPart + lcuPart + profilePart + fontsPart + assetsPart);
    return Math.min(96, raw);
  }, [
    enabled,
    allReady,
    backendStatus.diskReady,
    backendStatus.networkReady,
    backendStatus.progress,
    preferencesReady,
    lcuInitialChecked,
    profileInitialReady,
    fontsReady,
    assetsReady,
  ]);

  useEffect(() => {
    setMaxProgress((prev) => Math.max(prev, computedProgress));
  }, [computedProgress]);

  const stageKey: StartupStageKey = useMemo(() => {
    if (!enabled || allReady) return "ready";
    if (!preferencesReady) return "settings";
    if (!backendStatus.diskReady) return "cache";
    if (!lcuInitialChecked || !profileInitialReady) return "client";
    if (!fontsReady || !assetsReady) return "assets";
    return "syncing";
  }, [
    enabled,
    allReady,
    preferencesReady,
    backendStatus.diskReady,
    lcuInitialChecked,
    profileInitialReady,
    fontsReady,
    assetsReady,
  ]);

  // 6. Trigger smooth exit sequence once everything is ready
  useEffect(() => {
    if (!enabled || !allReady || completionStartedRef.current) return;
    completionStartedRef.current = true;
    setMaxProgress(100);

    const exitDuration = reducedMotion ? 0 : EXIT_ANIMATION_MS;
    let unmountTimer: number | undefined;
    const beatTimer = window.setTimeout(() => {
      setIsExiting(true);
      unmountTimer = window.setTimeout(() => {
        setIsBooting(false);
      }, exitDuration);
    }, EXIT_COMPLETE_BEAT_MS);

    return () => {
      window.clearTimeout(beatTimer);
      if (unmountTimer !== undefined) {
        window.clearTimeout(unmountTimer);
      }
    };
  }, [enabled, allReady, reducedMotion]);

  return {
    isBooting,
    isExiting,
    progress: allReady ? 100 : maxProgress,
    stageKey,
  };
}

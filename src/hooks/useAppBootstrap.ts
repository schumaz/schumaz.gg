import { useEffect, useMemo, useRef, useState } from "react";
import type { BootstrapStatus } from "../types";
import { isTauriWindowAvailable } from "../services/ipc/window.ipc";
import {
  ASSETS_PRELOAD_TIMEOUT_MS,
  EXIT_ANIMATION_MS,
  EXIT_COMPLETE_BEAT_MS,
  FONTS_READY_TIMEOUT_MS,
  MAX_SPLASH_TIMEOUT_MS,
  MIN_SPLASH_DURATION_MS,
  NETWORK_WAIT_AFTER_CORE_MS,
  type StartupStageKey,
  type UseAppBootstrapOptions,
  type UseAppBootstrapResult,
  computeBootstrapProgress,
  preloadImage,
  resolveBootstrapStageKey,
  subscribeToBackendBootstrap,
} from "./appBootstrapHelpers";

export {
  ASSETS_PRELOAD_TIMEOUT_MS,
  EXIT_ANIMATION_MS,
  EXIT_COMPLETE_BEAT_MS,
  FONTS_READY_TIMEOUT_MS,
  MAX_SPLASH_TIMEOUT_MS,
  MIN_SPLASH_DURATION_MS,
  NETWORK_WAIT_AFTER_CORE_MS,
  type StartupStageKey,
  type UseAppBootstrapOptions,
  type UseAppBootstrapResult,
};

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
    return subscribeToBackendBootstrap((next) => {
      setBackendStatus((prev) => ({
        stage: next.progress >= prev.progress ? next.stage : prev.stage,
        progress: Math.max(prev.progress, Math.min(100, next.progress)),
        diskReady: prev.diskReady || next.diskReady,
        networkReady: prev.networkReady || next.networkReady,
      }));
    });
  }, [enabled]);

  // 2. Minimum splash duration + hard global safety timeout
  useEffect(() => {
    if (!enabled) return;
    const minTimer = window.setTimeout(() => setMinDurationElapsed(true), MIN_SPLASH_DURATION_MS);
    const maxTimer = window.setTimeout(() => setHardTimedOut(true), MAX_SPLASH_TIMEOUT_MS);

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

  const computedProgress = useMemo(
    () =>
      computeBootstrapProgress({
        enabled,
        allReady,
        backendStatus,
        preferencesReady,
        lcuInitialChecked,
        profileInitialReady,
        fontsReady,
        assetsReady,
      }),
    [
      enabled,
      allReady,
      backendStatus,
      preferencesReady,
      lcuInitialChecked,
      profileInitialReady,
      fontsReady,
      assetsReady,
    ]
  );

  useEffect(() => {
    setMaxProgress((prev) => Math.max(prev, computedProgress));
  }, [computedProgress]);

  const stageKey: StartupStageKey = useMemo(
    () =>
      resolveBootstrapStageKey({
        enabled,
        allReady,
        preferencesReady,
        diskReady: backendStatus.diskReady,
        lcuInitialChecked,
        profileInitialReady,
        fontsReady,
        assetsReady,
      }),
    [
      enabled,
      allReady,
      preferencesReady,
      backendStatus.diskReady,
      lcuInitialChecked,
      profileInitialReady,
      fontsReady,
      assetsReady,
    ]
  );

  // 6. Trigger smooth exit sequence once everything is ready
  useEffect(() => {
    if (!enabled || !allReady || completionStartedRef.current) return;
    completionStartedRef.current = true;
    setMaxProgress(100);

    const exitDuration = reducedMotion ? 0 : EXIT_ANIMATION_MS;
    let unmountTimer: number | undefined;
    const beatTimer = window.setTimeout(() => {
      setIsExiting(true);
      unmountTimer = window.setTimeout(() => setIsBooting(false), exitDuration);
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

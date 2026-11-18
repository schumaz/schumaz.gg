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

export function isValidBootstrapStatus(value: unknown): value is BootstrapStatus {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<BootstrapStatus>;
  return (
    typeof candidate.stage === "string" &&
    typeof candidate.progress === "number" &&
    typeof candidate.diskReady === "boolean" &&
    typeof candidate.networkReady === "boolean"
  );
}

export function preloadImage(url: string): Promise<void> {
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

export function subscribeToBackendBootstrap(
  onUpdate: (next: BootstrapStatus) => void
): () => void {
  if (!isTauriWindowAvailable()) {
    onUpdate({ stage: "ready", progress: 100, diskReady: true, networkReady: true });
    return () => {};
  }

  let active = true;
  let unlisten: (() => void) | undefined;

  const apply = (next: BootstrapStatus) => {
    if (active) onUpdate(next);
  };

  void windowIpc
    .onBootstrapProgress((status) => {
      if (isValidBootstrapStatus(status)) {
        apply(status);
      }
    })
    .then((fn) => {
      if (!active) {
        fn();
      } else {
        unlisten = fn;
      }
    });

  void windowIpc
    .getBootstrapStatus()
    .then((status) => {
      if (isValidBootstrapStatus(status)) {
        apply(status);
      } else {
        // Graceful fallback if command returns null/unmocked payload
        apply({ stage: "ready", progress: 100, diskReady: true, networkReady: true });
      }
    })
    .catch(() => {
      apply({ stage: "ready", progress: 100, diskReady: true, networkReady: true });
    });

  return () => {
    active = false;
    unlisten?.();
  };
}

export function computeBootstrapProgress(params: {
  enabled: boolean;
  allReady: boolean;
  backendStatus: BootstrapStatus;
  preferencesReady: boolean;
  lcuInitialChecked: boolean;
  profileInitialReady: boolean;
  fontsReady: boolean;
  assetsReady: boolean;
}): number {
  if (!params.enabled || params.allReady) return 100;
  const backendPart = params.backendStatus.networkReady
    ? 45
    : params.backendStatus.diskReady
    ? 36
    : Math.round((Math.min(params.backendStatus.progress, 80) / 80) * 36);
  const prefsPart = params.preferencesReady ? 15 : 0;
  const lcuPart = params.lcuInitialChecked ? 10 : 0;
  const profilePart = params.profileInitialReady ? 10 : 0;
  const fontsPart = params.fontsReady ? 5 : 0;
  const assetsPart = params.assetsReady ? 15 : 0;
  const raw = Math.max(8, backendPart + prefsPart + lcuPart + profilePart + fontsPart + assetsPart);
  return Math.min(96, raw);
}

export function resolveBootstrapStageKey(params: {
  enabled: boolean;
  allReady: boolean;
  preferencesReady: boolean;
  diskReady: boolean;
  lcuInitialChecked: boolean;
  profileInitialReady: boolean;
  fontsReady: boolean;
  assetsReady: boolean;
}): StartupStageKey {
  if (!params.enabled || params.allReady) return "ready";
  if (!params.preferencesReady) return "settings";
  if (!params.diskReady) return "cache";
  if (!params.lcuInitialChecked || !params.profileInitialReady) return "client";
  if (!params.fontsReady || !params.assetsReady) return "assets";
  return "syncing";
}

import { useCallback, useEffect, useRef, useState } from "react";
import {
  lcuIpc,
  updaterIpc,
  type AppDistributionInfo,
  type UpdaterUpdateHandle,
} from "../services/ipc";

export const UPDATE_BLOCKED_PHASES = new Set([
  "ChampSelect",
  "GameStart",
  "InProgress",
  "Reconnect",
]);

export function isUpdateDownloadBlockedPhase(
  gameflowPhase: string,
  isInChampSelect: boolean
): boolean {
  return isInChampSelect || UPDATE_BLOCKED_PHASES.has(gameflowPhase);
}

export type UpdateCheckResult = "idle" | "up-to-date" | "error";

export type AppUpdateState =
  | { status: "idle" }
  | { status: "checking" }
  | { status: "deferred"; version: string }
  | { status: "downloading"; version: string }
  | { status: "ready"; version: string }
  | { status: "installing"; version: string }
  | { status: "portable-available"; version: string; releasesUrl: string };

export interface CheckForUpdatesOptions {
  manual?: boolean;
}

export interface UseAppUpdaterOptions {
  enabled?: boolean;
  lcuInitialChecked: boolean;
  gameflowPhase: string;
  isInChampSelect: boolean;
}

export function useAppUpdater({
  enabled = true,
  lcuInitialChecked,
  gameflowPhase,
  isInChampSelect,
}: UseAppUpdaterOptions) {
  const [updateState, setUpdateState] = useState<AppUpdateState>({ status: "idle" });
  const [distributionInfo, setDistributionInfo] = useState<AppDistributionInfo | null>(null);
  const [lastCheckResult, setLastCheckResult] = useState<UpdateCheckResult>("idle");
  const isBlocked = isUpdateDownloadBlockedPhase(gameflowPhase, isInChampSelect);

  const activeUpdateRef = useRef<UpdaterUpdateHandle | null>(null);
  const dismissedVersionRef = useRef<string | null>(null);
  const distributionInfoRef = useRef<AppDistributionInfo | null>(null);
  const isPortableRef = useRef(false);
  const downloadStartedRef = useRef(false);
  const installingRef = useRef(false);
  const hasCheckedStartupRef = useRef(false);
  const isMountedRef = useRef(true);
  const blockedRef = useRef(isBlocked);
  const checkSeqRef = useRef(0);

  blockedRef.current = isBlocked;

  const closeActiveUpdate = useCallback(async () => {
    const handle = activeUpdateRef.current;
    activeUpdateRef.current = null;
    downloadStartedRef.current = false;
    if (handle) {
      await handle.close().catch(() => {});
    }
  }, []);

  const startBackgroundDownload = useCallback(
    async (handle: UpdaterUpdateHandle, seq: number) => {
      if (isPortableRef.current || downloadStartedRef.current) return;

      const livePhase = await lcuIpc.getGameflowPhase?.().catch(() => null);
      if (
        !isMountedRef.current ||
        checkSeqRef.current !== seq ||
        activeUpdateRef.current !== handle
      ) {
        return;
      }
      if (blockedRef.current || (livePhase && isUpdateDownloadBlockedPhase(livePhase, false))) {
        setUpdateState({ status: "deferred", version: handle.version });
        return;
      }

      downloadStartedRef.current = true;
      setUpdateState({ status: "downloading", version: handle.version });

      try {
        await handle.download();
        if (
          !isMountedRef.current ||
          checkSeqRef.current !== seq ||
          activeUpdateRef.current !== handle ||
          dismissedVersionRef.current === handle.version
        ) {
          return;
        }
        setUpdateState({ status: "ready", version: handle.version });
      } catch {
        // Network or signature verification failure: silently clean up resource
        if (activeUpdateRef.current === handle) {
          await closeActiveUpdate();
        }
        if (isMountedRef.current && checkSeqRef.current === seq) {
          setUpdateState({ status: "idle" });
          setLastCheckResult("error");
        }
      }
    },
    [closeActiveUpdate]
  );

  const checkForUpdates = useCallback(
    async (options?: CheckForUpdatesOptions) => {
      if (installingRef.current) return;

      const manual = Boolean(options?.manual);
      if (manual) {
        dismissedVersionRef.current = null;
      }

      const seq = ++checkSeqRef.current;
      await closeActiveUpdate();
      if (!isMountedRef.current || checkSeqRef.current !== seq) return;

      setLastCheckResult("idle");
      setUpdateState({ status: "checking" });

      try {
        const fetchedDistInfo = await updaterIpc.getDistributionInfo().catch(() => null);
        const distInfo = fetchedDistInfo ?? distributionInfoRef.current;
        if (!distInfo || !isMountedRef.current || checkSeqRef.current !== seq) {
          if (isMountedRef.current && checkSeqRef.current === seq) {
            setUpdateState({ status: "idle" });
            setLastCheckResult(manual ? "error" : "idle");
          }
          return;
        }

        distributionInfoRef.current = distInfo;
        isPortableRef.current = distInfo.isPortable;
        setDistributionInfo(distInfo);

        const update = await updaterIpc.checkForUpdate();
        if (!update) {
          if (isMountedRef.current && checkSeqRef.current === seq) {
            setUpdateState({ status: "idle" });
            setLastCheckResult("up-to-date");
          }
          return;
        }

        if (
          !isMountedRef.current ||
          checkSeqRef.current !== seq ||
          dismissedVersionRef.current === update.version
        ) {
          await update.close().catch(() => {});
          if (isMountedRef.current && checkSeqRef.current === seq) {
            setUpdateState({ status: "idle" });
          }
          return;
        }

        if (distInfo.isPortable) {
          const version = update.version;
          await update.close().catch(() => {});
          if (isMountedRef.current && checkSeqRef.current === seq) {
            setUpdateState({
              status: "portable-available",
              version,
              releasesUrl: distInfo.releasesUrl,
            });
          }
          return;
        }

        activeUpdateRef.current = update;
        if (blockedRef.current) {
          setUpdateState({ status: "deferred", version: update.version });
          return;
        }

        await startBackgroundDownload(update, seq);
      } catch {
        // Network, endpoint, or signature validation failure: fail silently
        await closeActiveUpdate();
        if (isMountedRef.current && checkSeqRef.current === seq) {
          setUpdateState({ status: "idle" });
          setLastCheckResult("error");
        }
      }
    },
    [closeActiveUpdate, startBackgroundDownload]
  );

  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
      checkSeqRef.current += 1;
      if (!installingRef.current) {
        void closeActiveUpdate();
      }
    };
  }, [closeActiveUpdate]);

  useEffect(() => {
    let cancelled = false;
    void updaterIpc
      .getDistributionInfo()
      .then((info) => {
        if (cancelled || !isMountedRef.current || !info) return;
        distributionInfoRef.current = info;
        isPortableRef.current = info.isPortable;
        setDistributionInfo(info);
      })
      .catch(() => {});

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!enabled || !lcuInitialChecked || hasCheckedStartupRef.current) return;
    hasCheckedStartupRef.current = true;
    void checkForUpdates();
  }, [enabled, lcuInitialChecked, checkForUpdates]);

  useEffect(() => {
    if (
      updateState.status === "deferred" &&
      !isBlocked &&
      !isPortableRef.current &&
      activeUpdateRef.current &&
      !downloadStartedRef.current
    ) {
      void startBackgroundDownload(activeUpdateRef.current, checkSeqRef.current);
    }
  }, [updateState.status, isBlocked, startBackgroundDownload]);

  const installAndRestart = useCallback(async () => {
    const handle = activeUpdateRef.current;
    if (isPortableRef.current || !handle || updateState.status !== "ready") return;
    installingRef.current = true;
    setUpdateState({ status: "installing", version: handle.version });
    try {
      await handle.install();
    } catch {
      installingRef.current = false;
      await closeActiveUpdate();
      if (isMountedRef.current) {
        setUpdateState({ status: "idle" });
        setLastCheckResult("error");
      }
    }
  }, [updateState.status, closeActiveUpdate]);

  const openPortableRelease = useCallback(async () => {
    if (updateState.status !== "portable-available") return;
    await updaterIpc.openReleasesPage(updateState.releasesUrl).catch(() => {});
  }, [updateState]);

  const dismissUpdate = useCallback(async () => {
    if ("version" in updateState) {
      dismissedVersionRef.current = updateState.version;
    }
    checkSeqRef.current += 1;
    await closeActiveUpdate();
    if (isMountedRef.current) {
      setUpdateState({ status: "idle" });
      setLastCheckResult("idle");
    }
  }, [updateState, closeActiveUpdate]);

  return {
    updateState,
    distributionInfo,
    lastCheckResult,
    installAndRestart,
    openPortableRelease,
    dismissUpdate,
    checkForUpdates,
  };
}

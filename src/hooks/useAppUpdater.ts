import { useCallback, useEffect, useRef, useState } from "react";
import {
  updaterIpc,
  type AppDistributionInfo,
  type UpdaterUpdateHandle,
} from "../services/ipc";
import {
  UPDATE_BLOCKED_PHASES,
  isUpdateDownloadBlockedPhase,
  performUpdateCheck,
  runBackgroundDownload,
  type AppUpdateState,
  type CheckForUpdatesOptions,
  type UpdateCheckResult,
  type UseAppUpdaterOptions,
} from "./appUpdaterHelpers";

export {
  UPDATE_BLOCKED_PHASES,
  isUpdateDownloadBlockedPhase,
  type AppUpdateState,
  type CheckForUpdatesOptions,
  type UpdateCheckResult,
  type UseAppUpdaterOptions,
};

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

  const getRefs = () => ({
    activeUpdateRef,
    dismissedVersionRef,
    distributionInfoRef,
    isPortableRef,
    downloadStartedRef,
    isMountedRef,
    blockedRef,
    checkSeqRef,
  });

  const getActions = () => ({
    closeActiveUpdate,
    setUpdateState,
    setDistributionInfo,
    setLastCheckResult,
  });

  const startBackgroundDownload = useCallback(
    async (handle: UpdaterUpdateHandle, seq: number) =>
      runBackgroundDownload(handle, seq, getRefs(), getActions()),
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
        await performUpdateCheck(manual, seq, getRefs(), getActions());
      } catch {
        // Network, endpoint, or signature validation failure: fail silently
        await closeActiveUpdate();
        if (isMountedRef.current && checkSeqRef.current === seq) {
          setUpdateState({ status: "idle" });
          setLastCheckResult("error");
        }
      }
    },
    [closeActiveUpdate]
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

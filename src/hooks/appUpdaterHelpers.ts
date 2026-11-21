import type { MutableRefObject } from "react";
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

export interface UpdaterRefs {
  activeUpdateRef: MutableRefObject<UpdaterUpdateHandle | null>;
  dismissedVersionRef: MutableRefObject<string | null>;
  distributionInfoRef: MutableRefObject<AppDistributionInfo | null>;
  isPortableRef: MutableRefObject<boolean>;
  downloadStartedRef: MutableRefObject<boolean>;
  isMountedRef: MutableRefObject<boolean>;
  blockedRef: MutableRefObject<boolean>;
  checkSeqRef: MutableRefObject<number>;
}

export interface UpdaterActions {
  closeActiveUpdate: () => Promise<void>;
  setUpdateState: (state: AppUpdateState) => void;
  setDistributionInfo: (info: AppDistributionInfo) => void;
  setLastCheckResult: (result: UpdateCheckResult) => void;
  setLastCheckEpochSecs: (epochSecs: number | null) => void;
}

export async function runBackgroundDownload(
  handle: UpdaterUpdateHandle,
  seq: number,
  refs: UpdaterRefs,
  actions: UpdaterActions
): Promise<void> {
  if (refs.isPortableRef.current || refs.downloadStartedRef.current) return;

  const livePhase = await lcuIpc.getGameflowPhase?.().catch(() => null);
  if (
    !refs.isMountedRef.current ||
    refs.checkSeqRef.current !== seq ||
    refs.activeUpdateRef.current !== handle
  ) {
    return;
  }
  if (refs.blockedRef.current || (livePhase && isUpdateDownloadBlockedPhase(livePhase, false))) {
    actions.setUpdateState({ status: "deferred", version: handle.version });
    return;
  }

  refs.downloadStartedRef.current = true;
  actions.setUpdateState({ status: "downloading", version: handle.version });

  try {
    await handle.download();
    if (
      !refs.isMountedRef.current ||
      refs.checkSeqRef.current !== seq ||
      refs.activeUpdateRef.current !== handle ||
      refs.dismissedVersionRef.current === handle.version
    ) {
      return;
    }
    actions.setUpdateState({ status: "ready", version: handle.version });
  } catch {
    // Network or signature verification failure: silently clean up resource
    if (refs.activeUpdateRef.current === handle) {
      await actions.closeActiveUpdate();
    }
    if (refs.isMountedRef.current && refs.checkSeqRef.current === seq) {
      actions.setUpdateState({ status: "idle" });
      actions.setLastCheckResult("error");
    }
  }
}

export async function performUpdateCheck(
  manual: boolean,
  seq: number,
  refs: UpdaterRefs,
  actions: UpdaterActions
): Promise<void> {
  const fetchedDistInfo = await updaterIpc.getDistributionInfo().catch(() => null);
  const distInfo = fetchedDistInfo ?? refs.distributionInfoRef.current;
  if (!distInfo || !refs.isMountedRef.current || refs.checkSeqRef.current !== seq) {
    if (refs.isMountedRef.current && refs.checkSeqRef.current === seq) {
      actions.setUpdateState({ status: "idle" });
      actions.setLastCheckResult(manual ? "error" : "idle");
    }
    return;
  }

  refs.distributionInfoRef.current = distInfo;
  refs.isPortableRef.current = distInfo.isPortable;
  actions.setDistributionInfo(distInfo);
  if (typeof distInfo.lastCheckEpochSecs === "number") {
    actions.setLastCheckEpochSecs(distInfo.lastCheckEpochSecs);
  }

  const probe = await updaterIpc.probeForUpdate(manual);
  if (!refs.isMountedRef.current || refs.checkSeqRef.current !== seq) return;

  if (!probe) {
    actions.setUpdateState({ status: "idle" });
    actions.setLastCheckResult(manual ? "error" : "idle");
    return;
  }

  if (typeof probe.lastCheckEpochSecs === "number") {
    actions.setLastCheckEpochSecs(probe.lastCheckEpochSecs);
  }

  if (probe.status === "cooldown") {
    actions.setUpdateState({ status: "idle" });
    actions.setLastCheckResult("idle");
    return;
  }

  if (probe.status === "up-to-date") {
    actions.setUpdateState({ status: "idle" });
    actions.setLastCheckResult("up-to-date");
    return;
  }

  if (probe.status === "error") {
    actions.setUpdateState({ status: "idle" });
    actions.setLastCheckResult("error");
    return;
  }

  const probedVersion = probe.latestVersion ?? "";
  if (!manual && probedVersion && refs.dismissedVersionRef.current === probedVersion) {
    actions.setUpdateState({ status: "idle" });
    return;
  }

  const isPortable = probe.isPortable || distInfo.isPortable;
  refs.isPortableRef.current = isPortable;
  if (isPortable) {
    actions.setUpdateState({
      status: "portable-available",
      version: probedVersion || distInfo.currentVersion,
      releasesUrl: probe.releasesUrl || distInfo.releasesUrl,
    });
    return;
  }

  // Installed (NSIS) with a newer GitHub release confirmed: now fetch signed latest.json
  const update = await updaterIpc.checkForUpdate();
  const recordedTs = await updaterIpc.recordUpdateCheckCompleted().catch(() => null);
  if (!refs.isMountedRef.current || refs.checkSeqRef.current !== seq) {
    if (update) await update.close().catch(() => {});
    return;
  }
  if (typeof recordedTs === "number") {
    actions.setLastCheckEpochSecs(recordedTs);
  }

  if (!update) {
    actions.setUpdateState({ status: "idle" });
    actions.setLastCheckResult("up-to-date");
    return;
  }

  if (refs.dismissedVersionRef.current === update.version) {
    await update.close().catch(() => {});
    actions.setUpdateState({ status: "idle" });
    return;
  }

  refs.activeUpdateRef.current = update;
  if (refs.blockedRef.current) {
    actions.setUpdateState({ status: "deferred", version: update.version });
    return;
  }

  await runBackgroundDownload(update, seq, refs, actions);
}

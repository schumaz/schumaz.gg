import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import {
  useAppUpdater,
  isUpdateDownloadBlockedPhase,
} from "../hooks/useAppUpdater";
import {
  lcuIpc,
  updaterIpc,
  type AppDistributionInfo,
  type UpdateProbeResult,
  type UpdaterUpdateHandle,
} from "../services/ipc";

const RELEASES_URL = "https://github.com/schumaz/schumaz.gg/releases/latest";

function makeDistInfo(overrides?: Partial<AppDistributionInfo>): AppDistributionInfo {
  return {
    isPortable: false,
    currentVersion: "0.1.0",
    releasesUrl: RELEASES_URL,
    lastCheckEpochSecs: null,
    checkCooldownSecs: 86400,
    ...overrides,
  };
}

function makeProbe(overrides?: Partial<UpdateProbeResult>): UpdateProbeResult {
  return {
    status: "update-available",
    latestVersion: "0.2.0",
    releasesUrl: RELEASES_URL,
    isPortable: false,
    lastCheckEpochSecs: null,
    ...overrides,
  };
}

export function createMockUpdateHandle(version = "0.2.0"): UpdaterUpdateHandle {
  return {
    version,
    currentVersion: "0.1.0",
    download: vi.fn().mockResolvedValue(undefined),
    install: vi.fn().mockResolvedValue(undefined),
    close: vi.fn().mockResolvedValue(undefined),
  };
}

describe("useAppUpdater — NSIS, Cooldown & Deferral", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.spyOn(lcuIpc, "getGameflowPhase").mockResolvedValue("Lobby");
    vi.spyOn(updaterIpc, "recordUpdateCheckCompleted").mockResolvedValue(1_760_000_100);
  });

  it("identifies ChampSelect, GameStart, InProgress, and Reconnect as blocked phases", () => {
    expect(isUpdateDownloadBlockedPhase("ChampSelect", false)).toBe(true);
    expect(isUpdateDownloadBlockedPhase("InProgress", false)).toBe(true);
    expect(isUpdateDownloadBlockedPhase("GameStart", false)).toBe(true);
    expect(isUpdateDownloadBlockedPhase("Reconnect", false)).toBe(true);
    expect(isUpdateDownloadBlockedPhase("Lobby", true)).toBe(true);
    expect(isUpdateDownloadBlockedPhase("Lobby", false)).toBe(false);
    expect(isUpdateDownloadBlockedPhase("", false)).toBe(false);
  });

  it("loads distributionInfo on initialization and skips checkForUpdate during 24h cooldown", async () => {
    const distInfo = makeDistInfo({ lastCheckEpochSecs: 1_760_000_000 });
    vi.spyOn(updaterIpc, "getDistributionInfo").mockResolvedValue(distInfo);
    const probeSpy = vi
      .spyOn(updaterIpc, "probeForUpdate")
      .mockResolvedValue(makeProbe({ status: "cooldown", latestVersion: null, lastCheckEpochSecs: 1_760_000_000 }));
    const checkSpy = vi.spyOn(updaterIpc, "checkForUpdate").mockResolvedValue(null);

    const { result } = renderHook(() =>
      useAppUpdater({
        enabled: true,
        lcuInitialChecked: true,
        gameflowPhase: "Lobby",
        isInChampSelect: false,
      })
    );

    await waitFor(() => {
      expect(probeSpy).toHaveBeenCalledWith(false);
    });
    expect(result.current.distributionInfo).toEqual(distInfo);
    expect(result.current.lastCheckEpochSecs).toBe(1_760_000_000);
    expect(result.current.lastCheckResult).toBe("idle");
    expect(result.current.updateState).toEqual({ status: "idle" });
    expect(checkSpy).not.toHaveBeenCalled();
  });

  it("downloads silently in background when probe confirms newer NSIS release and records timestamp", async () => {
    const mockHandle = createMockUpdateHandle("0.2.0");
    vi.spyOn(updaterIpc, "getDistributionInfo").mockResolvedValue(makeDistInfo());
    vi.spyOn(updaterIpc, "probeForUpdate").mockResolvedValue(makeProbe({ latestVersion: "0.2.0" }));
    vi.spyOn(updaterIpc, "checkForUpdate").mockResolvedValue(mockHandle);
    const recordSpy = vi.spyOn(updaterIpc, "recordUpdateCheckCompleted").mockResolvedValue(1_760_000_200);

    const { result } = renderHook(() =>
      useAppUpdater({
        enabled: true,
        lcuInitialChecked: true,
        gameflowPhase: "Lobby",
        isInChampSelect: false,
      })
    );

    await waitFor(() => {
      expect(result.current.updateState).toEqual({
        status: "ready",
        version: "0.2.0",
      });
    });

    expect(recordSpy).toHaveBeenCalledTimes(1);
    expect(result.current.lastCheckEpochSecs).toBe(1_760_000_200);
    expect(mockHandle.download).toHaveBeenCalledTimes(1);
    expect(mockHandle.install).not.toHaveBeenCalled();

    await act(async () => {
      await result.current.installAndRestart();
    });

    expect(mockHandle.install).toHaveBeenCalledTimes(1);
  });

  it("defers download during ChampSelect / InProgress and downloads once back outside match", async () => {
    const mockHandle = createMockUpdateHandle("0.2.0");
    vi.spyOn(updaterIpc, "getDistributionInfo").mockResolvedValue(makeDistInfo());
    vi.spyOn(updaterIpc, "probeForUpdate").mockResolvedValue(makeProbe({ latestVersion: "0.2.0" }));
    vi.spyOn(updaterIpc, "checkForUpdate").mockResolvedValue(mockHandle);
    const phaseSpy = vi
      .spyOn(lcuIpc, "getGameflowPhase")
      .mockResolvedValue("ChampSelect");

    const { result, rerender } = renderHook(
      ({ gameflowPhase, isInChampSelect }) =>
        useAppUpdater({
          enabled: true,
          lcuInitialChecked: true,
          gameflowPhase,
          isInChampSelect,
        }),
      {
        initialProps: {
          gameflowPhase: "ChampSelect",
          isInChampSelect: true,
        },
      }
    );

    await waitFor(() => {
      expect(result.current.updateState).toEqual({
        status: "deferred",
        version: "0.2.0",
      });
    });
    expect(mockHandle.download).not.toHaveBeenCalled();

    phaseSpy.mockResolvedValue("InProgress");
    rerender({ gameflowPhase: "InProgress", isInChampSelect: false });
    expect(mockHandle.download).not.toHaveBeenCalled();

    phaseSpy.mockResolvedValue("EndOfGame");
    rerender({ gameflowPhase: "EndOfGame", isInChampSelect: false });

    await waitFor(() => {
      expect(result.current.updateState).toEqual({
        status: "ready",
        version: "0.2.0",
      });
    });
    expect(mockHandle.download).toHaveBeenCalledTimes(1);
  });

  it("calls update.close() when dismissed and before a new update check", async () => {
    const firstHandle = createMockUpdateHandle("0.2.0");
    const secondHandle = createMockUpdateHandle("0.3.0");
    vi.spyOn(updaterIpc, "getDistributionInfo").mockResolvedValue(makeDistInfo());
    vi.spyOn(updaterIpc, "probeForUpdate")
      .mockResolvedValueOnce(makeProbe({ latestVersion: "0.2.0" }))
      .mockResolvedValueOnce(makeProbe({ latestVersion: "0.3.0" }));
    const checkSpy = vi
      .spyOn(updaterIpc, "checkForUpdate")
      .mockResolvedValueOnce(firstHandle)
      .mockResolvedValueOnce(secondHandle);

    const { result } = renderHook(() =>
      useAppUpdater({
        enabled: true,
        lcuInitialChecked: true,
        gameflowPhase: "Lobby",
        isInChampSelect: false,
      })
    );

    await waitFor(() => {
      expect(result.current.updateState).toEqual({ status: "ready", version: "0.2.0" });
    });

    await act(async () => {
      await result.current.dismissUpdate();
    });

    expect(firstHandle.close).toHaveBeenCalledTimes(1);
    expect(result.current.updateState).toEqual({ status: "idle" });

    await act(async () => {
      await result.current.checkForUpdates();
    });

    expect(checkSpy).toHaveBeenCalledTimes(2);
    await waitFor(() => {
      expect(result.current.updateState).toEqual({ status: "ready", version: "0.3.0" });
    });
  });
});

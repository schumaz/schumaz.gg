import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useAppUpdater } from "../hooks/useAppUpdater";
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

function createMockUpdateHandle(version = "0.2.0"): UpdaterUpdateHandle {
  return {
    version,
    currentVersion: "0.1.0",
    download: vi.fn().mockResolvedValue(undefined),
    install: vi.fn().mockResolvedValue(undefined),
    close: vi.fn().mockResolvedValue(undefined),
  };
}

describe("useAppUpdater — Manual Check, Portable & Errors", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.spyOn(lcuIpc, "getGameflowPhase").mockResolvedValue("Lobby");
    vi.spyOn(updaterIpc, "recordUpdateCheckCompleted").mockResolvedValue(1_760_000_500);
  });

  it("clears dismissedVersionRef and bypasses cooldown on manual checkForUpdates({ manual: true })", async () => {
    const firstHandle = createMockUpdateHandle("0.2.0");
    const manualRetryHandle = createMockUpdateHandle("0.2.0");
    vi.spyOn(updaterIpc, "getDistributionInfo").mockResolvedValue(makeDistInfo());
    const probeSpy = vi
      .spyOn(updaterIpc, "probeForUpdate")
      .mockResolvedValue(makeProbe({ latestVersion: "0.2.0" }));
    const checkSpy = vi
      .spyOn(updaterIpc, "checkForUpdate")
      .mockResolvedValueOnce(firstHandle)
      .mockResolvedValueOnce(manualRetryHandle);

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

    // Non-manual check respects dismissedVersionRef and never fetches latest.json
    await act(async () => {
      await result.current.checkForUpdates();
    });
    expect(checkSpy).toHaveBeenCalledTimes(1);
    expect(result.current.updateState).toEqual({ status: "idle" });

    // Manual check passes manual=true (bypasses cooldown) and clears dismissedVersionRef
    await act(async () => {
      await result.current.checkForUpdates({ manual: true });
    });
    expect(probeSpy).toHaveBeenLastCalledWith(true);
    await waitFor(() => {
      expect(result.current.updateState).toEqual({ status: "ready", version: "0.2.0" });
    });
  });

  it("sets lastCheckResult to up-to-date from HEAD probe without downloading latest.json", async () => {
    vi.spyOn(updaterIpc, "getDistributionInfo").mockResolvedValue(makeDistInfo());
    vi.spyOn(updaterIpc, "probeForUpdate").mockResolvedValue(
      makeProbe({ status: "up-to-date", latestVersion: null, lastCheckEpochSecs: 1_760_000_300 })
    );
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
      expect(result.current.lastCheckResult).toBe("up-to-date");
    });
    expect(result.current.updateState).toEqual({ status: "idle" });
    expect(result.current.lastCheckEpochSecs).toBe(1_760_000_300);
    expect(checkSpy).not.toHaveBeenCalled();
  });

  it("reports portable updates directly from HEAD probe without fetching latest.json", async () => {
    vi.spyOn(updaterIpc, "getDistributionInfo").mockResolvedValue(
      makeDistInfo({ isPortable: true })
    );
    vi.spyOn(updaterIpc, "probeForUpdate").mockResolvedValue(
      makeProbe({ isPortable: true, latestVersion: "0.2.0", lastCheckEpochSecs: 1_760_000_400 })
    );
    const checkSpy = vi.spyOn(updaterIpc, "checkForUpdate").mockResolvedValue(null);
    const openSpy = vi
      .spyOn(updaterIpc, "openReleasesPage")
      .mockResolvedValue(undefined);

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
        status: "portable-available",
        version: "0.2.0",
        releasesUrl: RELEASES_URL,
      });
    });

    expect(checkSpy).not.toHaveBeenCalled();
    expect(result.current.lastCheckEpochSecs).toBe(1_760_000_400);

    await act(async () => {
      await result.current.openPortableRelease();
    });

    expect(openSpy).toHaveBeenCalledWith(RELEASES_URL);
  });

  it("does not record timestamp when checkForUpdate fails and handles download errors silently", async () => {
    const mockHandle = createMockUpdateHandle("0.2.0");
    vi.mocked(mockHandle.download).mockRejectedValue(new Error("signature mismatch"));
    vi.spyOn(updaterIpc, "getDistributionInfo").mockResolvedValue(makeDistInfo());
    vi.spyOn(updaterIpc, "probeForUpdate")
      .mockResolvedValueOnce(makeProbe({ latestVersion: "0.2.0" }))
      .mockResolvedValueOnce(makeProbe({ latestVersion: "0.2.0" }));
    const checkSpy = vi
      .spyOn(updaterIpc, "checkForUpdate")
      .mockRejectedValueOnce(new Error("network offline"))
      .mockResolvedValueOnce(mockHandle);
    const recordSpy = vi.spyOn(updaterIpc, "recordUpdateCheckCompleted").mockResolvedValue(1_760_000_600);

    const { result } = renderHook(() =>
      useAppUpdater({
        enabled: true,
        lcuInitialChecked: true,
        gameflowPhase: "Lobby",
        isInChampSelect: false,
      })
    );

    await waitFor(() => {
      expect(result.current.lastCheckResult).toBe("error");
    });
    expect(recordSpy).not.toHaveBeenCalled();

    // Subsequent manual check succeeds at manifest check, then fails during binary download
    await act(async () => {
      await result.current.checkForUpdates({ manual: true });
    });

    await waitFor(() => {
      expect(mockHandle.close).toHaveBeenCalledTimes(1);
    });
    expect(checkSpy).toHaveBeenCalledTimes(2);
    expect(result.current.updateState).toEqual({ status: "idle" });
    expect(result.current.lastCheckResult).toBe("error");
  });
});

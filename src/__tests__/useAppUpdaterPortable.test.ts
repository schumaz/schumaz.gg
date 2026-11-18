import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useAppUpdater } from "../hooks/useAppUpdater";
import { lcuIpc, updaterIpc, type UpdaterUpdateHandle } from "../services/ipc";

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
  });

  it("clears dismissedVersionRef on manual checkForUpdates({ manual: true }) so dismissed versions reappear", async () => {
    const firstHandle = createMockUpdateHandle("0.2.0");
    const autoRetryHandle = createMockUpdateHandle("0.2.0");
    const manualRetryHandle = createMockUpdateHandle("0.2.0");
    vi.spyOn(updaterIpc, "getDistributionInfo").mockResolvedValue({
      isPortable: false,
      currentVersion: "0.1.0",
      releasesUrl: "https://github.com/schumaz/schumaz.gg/releases/latest",
    });
    vi.spyOn(updaterIpc, "checkForUpdate")
      .mockResolvedValueOnce(firstHandle)
      .mockResolvedValueOnce(autoRetryHandle)
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

    // Non-manual check respects dismissedVersionRef
    await act(async () => {
      await result.current.checkForUpdates();
    });
    expect(autoRetryHandle.close).toHaveBeenCalledTimes(1);
    expect(result.current.updateState).toEqual({ status: "idle" });

    // Manual check clears dismissedVersionRef
    await act(async () => {
      await result.current.checkForUpdates({ manual: true });
    });
    await waitFor(() => {
      expect(result.current.updateState).toEqual({ status: "ready", version: "0.2.0" });
    });
  });

  it("sets lastCheckResult to up-to-date when no update is available", async () => {
    vi.spyOn(updaterIpc, "getDistributionInfo").mockResolvedValue({
      isPortable: false,
      currentVersion: "0.1.0",
      releasesUrl: "https://github.com/schumaz/schumaz.gg/releases/latest",
    });
    vi.spyOn(updaterIpc, "checkForUpdate").mockResolvedValue(null);

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
  });

  it("never downloads or installs in portable mode, closes handle, and opens GitHub Release via opener", async () => {
    const mockHandle = createMockUpdateHandle("0.2.0");
    vi.spyOn(updaterIpc, "getDistributionInfo").mockResolvedValue({
      isPortable: true,
      currentVersion: "0.1.0",
      releasesUrl: "https://github.com/schumaz/schumaz.gg/releases/latest",
    });
    vi.spyOn(updaterIpc, "checkForUpdate").mockResolvedValue(mockHandle);
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
        releasesUrl: "https://github.com/schumaz/schumaz.gg/releases/latest",
      });
    });

    expect(mockHandle.download).not.toHaveBeenCalled();
    expect(mockHandle.install).not.toHaveBeenCalled();
    expect(mockHandle.close).toHaveBeenCalledTimes(1);

    await act(async () => {
      await result.current.openPortableRelease();
    });

    expect(openSpy).toHaveBeenCalledWith(
      "https://github.com/schumaz/schumaz.gg/releases/latest"
    );
  });

  it("handles network or signature failures silently, sets lastCheckResult to error, and cleans up", async () => {
    const mockHandle = createMockUpdateHandle("0.2.0");
    vi.mocked(mockHandle.download).mockRejectedValue(
      new Error("signature mismatch")
    );
    vi.spyOn(updaterIpc, "getDistributionInfo").mockResolvedValue({
      isPortable: false,
      currentVersion: "0.1.0",
      releasesUrl: "https://github.com/schumaz/schumaz.gg/releases/latest",
    });
    vi.spyOn(updaterIpc, "checkForUpdate").mockResolvedValue(mockHandle);

    const { result } = renderHook(() =>
      useAppUpdater({
        enabled: true,
        lcuInitialChecked: true,
        gameflowPhase: "Lobby",
        isInChampSelect: false,
      })
    );

    await waitFor(() => {
      expect(mockHandle.close).toHaveBeenCalledTimes(1);
    });
    expect(result.current.updateState).toEqual({ status: "idle" });
    expect(result.current.lastCheckResult).toBe("error");
  });
});

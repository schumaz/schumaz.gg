import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import {
  useAppUpdater,
  isUpdateDownloadBlockedPhase,
} from "../hooks/useAppUpdater";
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

describe("useAppUpdater", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.spyOn(lcuIpc, "getGameflowPhase").mockResolvedValue("Lobby");
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

  it("downloads silently in background when installed (NSIS) and installs on restart", async () => {
    const mockHandle = createMockUpdateHandle("0.2.0");
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
      expect(result.current.updateState).toEqual({
        status: "ready",
        version: "0.2.0",
      });
    });

    expect(mockHandle.download).toHaveBeenCalledTimes(1);
    expect(mockHandle.install).not.toHaveBeenCalled();

    await act(async () => {
      await result.current.installAndRestart();
    });

    expect(mockHandle.install).toHaveBeenCalledTimes(1);
  });

  it("defers download during ChampSelect / InProgress and downloads once back outside match", async () => {
    const mockHandle = createMockUpdateHandle("0.2.0");
    vi.spyOn(updaterIpc, "getDistributionInfo").mockResolvedValue({
      isPortable: false,
      currentVersion: "0.1.0",
      releasesUrl: "https://github.com/schumaz/schumaz.gg/releases/latest",
    });
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

    // Transition to InProgress: still deferred
    phaseSpy.mockResolvedValue("InProgress");
    rerender({ gameflowPhase: "InProgress", isInChampSelect: false });
    expect(mockHandle.download).not.toHaveBeenCalled();

    // Match ends -> EndOfGame: download resumes automatically
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
    vi.spyOn(updaterIpc, "getDistributionInfo").mockResolvedValue({
      isPortable: false,
      currentVersion: "0.1.0",
      releasesUrl: "https://github.com/schumaz/schumaz.gg/releases/latest",
    });
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
      expect(result.current.updateState).toEqual({
        status: "ready",
        version: "0.2.0",
      });
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
      expect(result.current.updateState).toEqual({
        status: "ready",
        version: "0.3.0",
      });
    });
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

  it("handles network or signature failures silently and cleans up the update resource", async () => {
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
  });
});

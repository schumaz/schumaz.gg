import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { listen } from "@tauri-apps/api/event";
import { settingsIpc } from "../../services/ipc";
import { useCacheClearInvalidation } from "../useCacheClearInvalidation";
import * as assetsUtils from "../../utils/assets";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue({
    totalBytes: 0,
    assetsBytes: 0,
    dataBytes: 0,
    fileCount: 0,
  }),
  convertFileSrc: vi.fn((path: string) => `asset://localhost/${path}`),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

describe("useCacheClearInvalidation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("flushes matching L1 RAM caches per scope on both IPC events and local clearLocalCache calls", async () => {
    let emittedHandler: ((event: { payload: unknown }) => void) | undefined;
    vi.mocked(listen).mockImplementationOnce((_event, handler) => {
      emittedHandler = handler as (event: { payload: unknown }) => void;
      return Promise.resolve(() => {});
    });

    const onInvalidateBuilds = vi.fn();
    const clearAssetsSpy = vi.spyOn(assetsUtils, "clearAssetUrlCache");

    const { unmount } = renderHook(() => useCacheClearInvalidation({ onInvalidateBuilds }));

    await act(async () => {
      await Promise.resolve();
    });

    act(() => {
      emittedHandler?.({ payload: "images" });
    });
    expect(onInvalidateBuilds).not.toHaveBeenCalled();
    expect(clearAssetsSpy).toHaveBeenCalledTimes(1);

    await act(async () => {
      await settingsIpc.clearLocalCache("builds");
    });
    expect(onInvalidateBuilds).toHaveBeenCalledTimes(1);
    expect(clearAssetsSpy).toHaveBeenCalledTimes(1);

    await act(async () => {
      await settingsIpc.clearLocalCache("all");
    });
    expect(onInvalidateBuilds).toHaveBeenCalledTimes(2);
    expect(clearAssetsSpy).toHaveBeenCalledTimes(2);

    unmount();
  });
});

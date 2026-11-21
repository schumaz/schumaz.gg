import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import {
  useAppBootstrap,
  MIN_SPLASH_DURATION_MS,
  NETWORK_WAIT_AFTER_CORE_MS,
  MAX_SPLASH_TIMEOUT_MS,
  EXIT_COMPLETE_BEAT_MS,
  EXIT_ANIMATION_MS,
  FONTS_READY_TIMEOUT_MS,
} from "../hooks/useAppBootstrap";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((path: string, protocol = "asset") => `${protocol}://localhost/${path}`),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockImplementation(() => Promise.resolve(() => {})),
}));

describe("useAppBootstrap — Timeouts, Warm Reopen Fast-Path & Offline Grace", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
  });

  afterEach(() => {
    vi.useRealTimers();
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  it("skips MIN_SPLASH_DURATION_MS fast-path when backend already reports diskReady on warm reopen", async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === "get_bootstrap_status") {
        return Promise.resolve({
          stage: "ready",
          progress: 100,
          diskReady: true,
          networkReady: true,
        });
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() =>
      useAppBootstrap({
        enabled: true,
        preferencesReady: true,
        lcuInitialChecked: true,
        profileInitialReady: true,
        criticalImageUrls: [],
        reducedMotion: true,
      })
    );

    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

    // Already ready without waiting MIN_SPLASH_DURATION_MS (650ms)
    expect(result.current.progress).toBe(100);
    expect(result.current.stageKey).toBe("ready");

    // Advances only EXIT_COMPLETE_BEAT_MS (150ms < MIN_SPLASH_DURATION_MS)
    await act(async () => {
      vi.advanceTimersByTime(EXIT_COMPLETE_BEAT_MS + 10);
    });
    expect(result.current.isBooting).toBe(false);
  });

  it("does not hang offline when backend disk is ready but network prewarm is slow", async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === "get_bootstrap_status") {
        return Promise.resolve({
          stage: "tier_lists_disk",
          progress: 80,
          diskReady: true,
          networkReady: false,
        });
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() =>
      useAppBootstrap({
        enabled: true,
        preferencesReady: true,
        lcuInitialChecked: true,
        profileInitialReady: true,
        criticalImageUrls: [],
        reducedMotion: true,
      })
    );

    await act(async () => {
      await Promise.resolve();
      vi.advanceTimersByTime(Math.max(MIN_SPLASH_DURATION_MS, FONTS_READY_TIMEOUT_MS) + 20);
    });

    // Core is ready, waiting up to NETWORK_WAIT_AFTER_CORE_MS
    expect(result.current.isBooting).toBe(true);
    expect(result.current.stageKey).toBe("syncing");

    await act(async () => {
      vi.advanceTimersByTime(NETWORK_WAIT_AFTER_CORE_MS + 20);
    });
    await act(async () => {
      vi.advanceTimersByTime(EXIT_COMPLETE_BEAT_MS + 20);
    });

    expect(result.current.progress).toBe(100);
    expect(result.current.stageKey).toBe("ready");
    expect(result.current.isBooting).toBe(false);
  });

  it("triggers hard safety timeout if any startup dependency never resolves", async () => {
    vi.mocked(invoke).mockImplementation(() => new Promise(() => {}));

    const { result } = renderHook(() =>
      useAppBootstrap({
        enabled: true,
        preferencesReady: false,
        lcuInitialChecked: false,
        profileInitialReady: false,
        criticalImageUrls: [],
        reducedMotion: false,
      })
    );

    expect(result.current.isBooting).toBe(true);
    expect(result.current.stageKey).toBe("settings");

    await act(async () => {
      vi.advanceTimersByTime(MAX_SPLASH_TIMEOUT_MS + 20);
    });
    await act(async () => {
      vi.advanceTimersByTime(EXIT_COMPLETE_BEAT_MS + 20);
    });
    await act(async () => {
      vi.advanceTimersByTime(EXIT_ANIMATION_MS + 20);
    });

    expect(result.current.isBooting).toBe(false);
    expect(result.current.progress).toBe(100);
  });
});

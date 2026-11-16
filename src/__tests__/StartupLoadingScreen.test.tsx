import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act, renderHook } from "@testing-library/react";
import App from "../App";
import { invoke } from "@tauri-apps/api/core";
import { defaultAppInvokeMock, resetMockAppSettings } from "./appTestFixtures";
import {
  useAppBootstrap,
  MIN_SPLASH_DURATION_MS,
  NETWORK_WAIT_AFTER_CORE_MS,
  MAX_SPLASH_TIMEOUT_MS,
  EXIT_COMPLETE_BEAT_MS,
  EXIT_ANIMATION_MS,
  ASSETS_PRELOAD_TIMEOUT_MS,
  FONTS_READY_TIMEOUT_MS,
} from "../hooks/useAppBootstrap";
import type { AppSettings, BootstrapStatus } from "../types";

let eventListeners: Record<string, (event: { payload: unknown }) => void> = {};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args?: unknown) =>
    defaultAppInvokeMock(cmd, args as Record<string, unknown> | undefined)
  ),
  convertFileSrc: vi.fn((path: string, protocol = "asset") => `${protocol}://localhost/${path}`),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockImplementation((event: string, cb: (payload: unknown) => void) => {
    eventListeners[event] = cb;
    return Promise.resolve(() => {
      delete eventListeners[event];
    });
  }),
}));

function makeIniSettings(generalOverrides: Partial<AppSettings["general"]> = {}): AppSettings {
  return {
    general: {
      language: "pt_BR",
      languageOnboardingCompleted: true,
      flashOnboardingCompleted: true,
      ...generalOverrides,
    },
    automation: {
      autoInjectRunes: true,
      autoInjectSpells: true,
      autoInjectItems: true,
      flashKey: "D",
      patch48hProtection: true,
      autoMarkDailyMissionsRead: false,
      language: "pt_BR",
    },
    profileAppearance: {
      profileBlurPercent: 10,
      wallpaperVisible: true,
    },
    graphics: {
      hardwareAcceleration: true,
    },
    window: {
      windowWidth: 980,
      windowHeight: 720,
      windowX: null,
      windowY: null,
      windowMaximized: false,
    },
    navigation: {
      savedAramMode: null,
      profileMatchModeFilter: "all",
      aramMayhemRarityFilter: "all",
      aramMayhemLastChampion: null,
      rankedFilterRegion: "global",
      rankedFilterTier: "emerald_plus",
      rankedFilterLane: "all",
      rankedFilterPatch: null,
      rankedChampionTierFilter: "all",
      lastSummonerId: null,
    },
  };
}

describe("Startup Loading Screen & Bootstrap Gate (Option A)", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    eventListeners = {};
    resetMockAppSettings();
    vi.clearAllMocks();
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  afterEach(() => {
    vi.useRealTimers();
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  it("shows StartupLoadingScreen on launch, waits for backend + frontend hydration, and then reveals the app", async () => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};

    let resolveBootstrapStatus!: (value: BootstrapStatus) => void;
    const bootstrapStatusPromise = new Promise<BootstrapStatus>((resolve) => {
      resolveBootstrapStatus = resolve;
    });

    vi.mocked(invoke).mockImplementation((cmd: string, args?: unknown) => {
      if (cmd === "get_bootstrap_status") return bootstrapStatusPromise;
      if (cmd === "get_app_settings") return Promise.resolve(makeIniSettings());
      if (cmd === "get_hardware_acceleration_status") {
        return Promise.resolve({ enabled: true, active_at_startup: true });
      }
      return defaultAppInvokeMock(cmd, args as Record<string, unknown> | undefined);
    });

    render(<App forceStartupLoading />);

    const splash = screen.getByTestId("startup-loading-screen");
    expect(splash).toBeInTheDocument();
    expect(splash).toHaveTextContent(/schumaz\.gg/i);
    expect(screen.getByTestId("startup-progress-bar")).toHaveAttribute("role", "progressbar");
    expect(splash.querySelector(".startup-icon-img")).toBeInTheDocument();

    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

    // Still booting because backend disk_ready is false
    expect(screen.getByTestId("startup-loading-screen")).toBeInTheDocument();

    // Backend finishes disk + network bootstrap
    await act(async () => {
      resolveBootstrapStatus({
        stage: "ready",
        progress: 100,
        diskReady: true,
        networkReady: true,
      });
      await Promise.resolve();
    });

    // Advance past asset preload timeout and minimum splash duration
    await act(async () => {
      vi.advanceTimersByTime(
        Math.max(MIN_SPLASH_DURATION_MS, ASSETS_PRELOAD_TIMEOUT_MS, FONTS_READY_TIMEOUT_MS) + 50
      );
      await Promise.resolve();
    });

    expect(screen.getByTestId("startup-progress-bar")).toHaveAttribute("aria-valuenow", "100");

    // Advance through beat + exit transition
    await act(async () => {
      vi.advanceTimersByTime(EXIT_COMPLETE_BEAT_MS + 20);
    });
    await act(async () => {
      vi.advanceTimersByTime(EXIT_ANIMATION_MS + 20);
    });

    expect(screen.queryByTestId("startup-loading-screen")).not.toBeInTheDocument();
    expect(screen.getByTestId("status-disconnected")).toBeInTheDocument();
  });

  it("defers onboarding modals until the startup loading screen finishes", async () => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};

    vi.mocked(invoke).mockImplementation((cmd: string, args?: unknown) => {
      if (cmd === "get_bootstrap_status") {
        return Promise.resolve({
          stage: "ready",
          progress: 100,
          diskReady: true,
          networkReady: true,
        });
      }
      if (cmd === "get_app_settings") {
        return Promise.resolve(
          makeIniSettings({
            language: "en_US",
            languageOnboardingCompleted: false,
            flashOnboardingCompleted: false,
          })
        );
      }
      if (cmd === "get_hardware_acceleration_status") {
        return Promise.resolve({ enabled: true, active_at_startup: true });
      }
      return defaultAppInvokeMock(cmd, args as Record<string, unknown> | undefined);
    });

    render(<App forceStartupLoading />);

    // Loading screen is visible and onboarding modal is NOT mounted yet
    expect(screen.getByTestId("startup-loading-screen")).toBeInTheDocument();
    expect(screen.queryByTestId("language-onboarding-modal")).not.toBeInTheDocument();

    // Flush IPC microtasks so profileInitialReady schedules asset preload
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
      await Promise.resolve();
    });

    await act(async () => {
      vi.advanceTimersByTime(
        Math.max(MIN_SPLASH_DURATION_MS, ASSETS_PRELOAD_TIMEOUT_MS, FONTS_READY_TIMEOUT_MS) + 50
      );
    });
    await act(async () => {
      vi.advanceTimersByTime(EXIT_COMPLETE_BEAT_MS + 20);
    });
    await act(async () => {
      vi.advanceTimersByTime(EXIT_ANIMATION_MS + 20);
    });

    expect(screen.queryByTestId("startup-loading-screen")).not.toBeInTheDocument();
    expect(screen.getByTestId("language-onboarding-modal")).toBeInTheDocument();
  });

  it("does not hang offline when backend disk is ready but network prewarm is slow", async () => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};

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
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};

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

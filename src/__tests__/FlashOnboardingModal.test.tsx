import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import App from "../App";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings } from "../types";
import { defaultAppInvokeMock, resetMockAppSettings } from "./appTestFixtures";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

function makeIniSettings(overrides: Partial<AppSettings["general"]> = {}): AppSettings {
  return {
    general: {
      language: "pt_BR",
      languageOnboardingCompleted: true,
      flashOnboardingCompleted: false,
      ...overrides,
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
      windowWidth: 1280,
      windowHeight: 720,
      windowX: null,
      windowY: null,
      windowMaximized: false,
      windowMonitor: null,
    },
    navigation: {
      savedAramMode: null,
      profileMatchModeFilter: "all",
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

describe("First-Launch Flash Key Onboarding Screen", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetMockAppSettings();
    vi.mocked(invoke).mockImplementation((cmd, args) => defaultAppInvokeMock(cmd, args as Record<string, unknown>));
  });

  it("opens after language onboarding, lets user switch D/F without persisting until confirm, and persists F on confirm", async () => {
    resetMockAppSettings({
      general: {
        language: null,
        languageOnboardingCompleted: false,
        flashOnboardingCompleted: false,
      },
    });

    render(<App />);

    // First screen: Language onboarding
    const langModal = await screen.findByTestId("language-onboarding-modal");
    expect(langModal).toBeInTheDocument();
    expect(screen.queryByTestId("flash-onboarding-modal")).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId("onboarding-lang-pt_BR"));
    fireEvent.click(screen.getByTestId("onboarding-continue-btn"));

    // Second screen: Flash onboarding opens immediately in chosen language
    const flashModal = await screen.findByTestId("flash-onboarding-modal");
    expect(flashModal).toHaveAttribute("role", "dialog");
    expect(flashModal).toHaveAttribute("aria-modal", "true");
    expect(flashModal).toHaveTextContent("Em qual tecla você usa o Flash?");

    const optionD = screen.getByTestId("onboarding-flash-D");
    const optionF = screen.getByTestId("onboarding-flash-F");
    expect(optionD).toHaveAttribute("aria-checked", "true");
    expect(optionF).toHaveAttribute("aria-checked", "false");

    // Selecting F updates radio state without dismissing or persisting yet
    fireEvent.click(optionF);
    expect(screen.getByTestId("flash-onboarding-modal")).toBeInTheDocument();
    expect(optionF).toHaveAttribute("aria-checked", "true");
    expect(optionD).toHaveAttribute("aria-checked", "false");

    // Confirming persists flashKey F and closes the modal
    fireEvent.click(screen.getByTestId("flash-onboarding-continue-btn"));
    await waitFor(() => {
      expect(screen.queryByTestId("flash-onboarding-modal")).not.toBeInTheDocument();
    });

    expect(invoke).toHaveBeenCalledWith("complete_flash_onboarding", { flashKey: "F" });

    fireEvent.click(screen.getByRole("button", { name: /^configurações$/i }));
    expect(screen.getByRole("button", { name: "F" })).toHaveAttribute("aria-pressed", "true");
  });

  it("shows Flash onboarding once to existing users who already completed language onboarding, then skips on subsequent opens", async () => {
    resetMockAppSettings({
      general: {
        language: "pt_BR",
        languageOnboardingCompleted: true,
        flashOnboardingCompleted: false,
      },
      automation: {
        language: "pt_BR",
        flashKey: "D",
        autoInjectRunes: true,
        autoInjectSpells: true,
        autoInjectItems: true,
        patch48hProtection: true,
        autoMarkDailyMissionsRead: false,
      },
    });

    const { unmount } = render(<App />);

    const flashModal = await screen.findByTestId("flash-onboarding-modal");
    expect(flashModal).toBeInTheDocument();
    expect(screen.queryByTestId("language-onboarding-modal")).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId("flash-onboarding-continue-btn"));

    await waitFor(() => {
      expect(screen.queryByTestId("flash-onboarding-modal")).not.toBeInTheDocument();
    });
    expect(invoke).toHaveBeenCalledWith("complete_flash_onboarding", { flashKey: "D" });

    unmount();
    render(<App />);
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /^Configurações$/i })).toBeInTheDocument();
    });
    expect(screen.queryByTestId("language-onboarding-modal")).not.toBeInTheDocument();
    expect(screen.queryByTestId("flash-onboarding-modal")).not.toBeInTheDocument();
  });

  it("uses complete_flash_onboarding IPC when backed by UserSettings.ini", async () => {
    const iniBefore = makeIniSettings({ flashOnboardingCompleted: false });
    const iniAfter = {
      ...iniBefore,
      general: { ...iniBefore.general, flashOnboardingCompleted: true },
      automation: { ...iniBefore.automation, flashKey: "F" as const },
    };

    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === "get_app_settings") return Promise.resolve(iniBefore);
      if (cmd === "complete_flash_onboarding") return Promise.resolve(iniAfter);
      if (cmd === "get_hardware_acceleration_status") {
        return Promise.resolve({ enabled: true, active_at_startup: true });
      }
      return Promise.resolve({});
    });

    render(<App />);

    const flashModal = await screen.findByTestId("flash-onboarding-modal");
    expect(flashModal).toBeInTheDocument();
    expect(screen.queryByTestId("language-onboarding-modal")).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId("onboarding-flash-F"));
    fireEvent.click(screen.getByTestId("flash-onboarding-continue-btn"));

    await waitFor(() => {
      expect(screen.queryByTestId("flash-onboarding-modal")).not.toBeInTheDocument();
    });
    expect(invoke).toHaveBeenCalledWith("complete_flash_onboarding", { flashKey: "F" });
  });
});

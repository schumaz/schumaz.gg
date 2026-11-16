import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import App from "../App";
import { invoke } from "@tauri-apps/api/core";
import { defaultAppInvokeMock, makeDefaultMockAppSettings } from "./appTestFixtures";
import { setDefaultFallbackLocale } from "../i18n";

let eventListeners: Record<string, (event: { payload: unknown }) => void> = {};

let mockAppSettings = makeDefaultMockAppSettings();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args?: Record<string, unknown>) => {
    if (cmd === "get_app_settings") {
      return Promise.resolve(mockAppSettings);
    }
    if (cmd === "complete_language_onboarding") {
      mockAppSettings.general.language = (args?.locale as any) ?? "pt_BR";
      mockAppSettings.general.languageOnboardingCompleted = true;
      mockAppSettings.automation.language = (args?.locale as any) ?? "pt_BR";
      return Promise.resolve(mockAppSettings);
    }
    if (cmd === "complete_flash_onboarding") {
      mockAppSettings.general.flashOnboardingCompleted = true;
      mockAppSettings.automation.flashKey = (args?.flashKey as any) ?? "D";
      return Promise.resolve(mockAppSettings);
    }
    return defaultAppInvokeMock(cmd, args);
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockImplementation((event: string, cb: (payload: unknown) => void) => {
    eventListeners[event] = cb;
    return Promise.resolve(() => {
      delete eventListeners[event];
    });
  }),
}));

describe("First-Launch Language Onboarding Screen", () => {
  beforeEach(() => {
    eventListeners = {};
    setDefaultFallbackLocale("en_US");
    mockAppSettings = makeDefaultMockAppSettings();
    mockAppSettings.general.language = null;
    mockAppSettings.automation.language = null as any;
    mockAppSettings.general.languageOnboardingCompleted = false;
    mockAppSettings.general.flashOnboardingCompleted = false;
  });

  it("opens in English on first launch, updates UI immediately on flag click without saving, and transitions to Flash onboarding on Continue", async () => {
    render(<App />);

    // 1. Blurred onboarding overlay appears on first open with initial language = English (en_US)
    const modal = await screen.findByTestId("language-onboarding-modal");
    expect(modal).toBeInTheDocument();
    expect(modal).toHaveAttribute("role", "dialog");
    expect(modal).toHaveAttribute("aria-modal", "true");
    expect(modal).toHaveClass("language-onboarding-backdrop");
    expect(document.documentElement.lang).toBe("en-US");

    // Modal & underlying app UI start in English
    expect(modal).toHaveTextContent("Choose your language");
    const continueBtn = screen.getByTestId("onboarding-continue-btn");
    expect(continueBtn).toHaveTextContent("Continue");
    expect(screen.getByTestId("status-disconnected")).toHaveTextContent(/Waiting for LoL client/i);
    expect(screen.getByRole("button", { name: /^Settings$/i })).toBeInTheDocument();

    // Flag choices render bundled flag assets and en_US is initially selected
    const enBtn = screen.getByTestId("onboarding-lang-en_US");
    const ptBtn = screen.getByTestId("onboarding-lang-pt_BR");
    expect(enBtn).toHaveAttribute("aria-checked", "true");
    expect(ptBtn).toHaveAttribute("aria-checked", "false");
    expect(enBtn.querySelector("img")).toHaveAttribute("src", expect.stringMatching(/us\.svg|data:image/i));
    expect(ptBtn.querySelector("img")).toHaveAttribute("src", expect.stringMatching(/br\.svg|data:image/i));

    // Not persisted yet before Continue
    expect(localStorage.getItem("schumaz_language")).toBeNull();

    // 2. Clicking the Brazil flag immediately translates BOTH the modal and the app UI to Portuguese,
    //    WITHOUT saving to UserSettings.ini or closing the modal yet.
    fireEvent.click(ptBtn);

    expect(screen.getByTestId("language-onboarding-modal")).toBeInTheDocument();
    expect(ptBtn).toHaveAttribute("aria-checked", "true");
    expect(enBtn).toHaveAttribute("aria-checked", "false");
    expect(document.documentElement.lang).toBe("pt-BR");

    expect(modal).toHaveTextContent("Escolha seu idioma");
    expect(continueBtn).toHaveTextContent("Continuar");
    expect(screen.getByTestId("status-disconnected")).toHaveTextContent(/Aguardando cliente do LoL/i);
    expect(screen.getByRole("button", { name: /^Configurações$/i })).toBeInTheDocument();
    expect(localStorage.getItem("schumaz_language")).toBeNull();

    // Clicking USA flag switches immediately back to English without saving
    fireEvent.click(enBtn);
    expect(modal).toHaveTextContent("Choose your language");
    expect(continueBtn).toHaveTextContent("Continue");
    expect(localStorage.getItem("schumaz_language")).toBeNull();

    // Switch back to Portuguese and click Continue to persist and advance to Flash onboarding
    fireEvent.click(ptBtn);
    fireEvent.click(continueBtn);

    await waitFor(() => {
      expect(screen.queryByTestId("language-onboarding-modal")).not.toBeInTheDocument();
    });

    expect(invoke).toHaveBeenCalledWith("complete_language_onboarding", { locale: "pt_BR" });
    expect(screen.getByRole("button", { name: /^Configurações$/i })).toBeInTheDocument();
    expect(screen.getByTestId("flash-onboarding-modal")).toHaveTextContent("Em qual tecla você usa o Flash?");
  });

  it("persists English and Flash key sequentially on first launch and skips both onboarding screens on subsequent launches", async () => {
    const { unmount } = render(<App />);

    const langModal = await screen.findByTestId("language-onboarding-modal");
    expect(langModal).toBeInTheDocument();
    fireEvent.click(screen.getByTestId("onboarding-continue-btn"));

    await waitFor(() => {
      expect(screen.queryByTestId("language-onboarding-modal")).not.toBeInTheDocument();
    });

    expect(invoke).toHaveBeenCalledWith("complete_language_onboarding", { locale: "en_US" });

    // Flash onboarding opens right after language selection in English
    const flashModal = screen.getByTestId("flash-onboarding-modal");
    expect(flashModal).toHaveTextContent("Which key do you use for Flash?");
    fireEvent.click(screen.getByTestId("flash-onboarding-continue-btn"));

    await waitFor(() => {
      expect(screen.queryByTestId("flash-onboarding-modal")).not.toBeInTheDocument();
    });
    expect(invoke).toHaveBeenCalledWith("complete_flash_onboarding", { flashKey: "D" });

    // Re-mounting App skips both onboarding screens since both are persisted
    unmount();
    render(<App />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /^Settings$/i })).toBeInTheDocument();
    });
    expect(screen.queryByTestId("language-onboarding-modal")).not.toBeInTheDocument();
    expect(screen.queryByTestId("flash-onboarding-modal")).not.toBeInTheDocument();
  });
});

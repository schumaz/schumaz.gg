import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import App, { LANGUAGE_STORAGE_KEY } from "../App";
import { invoke } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(null),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

describe("First-Launch Language Onboarding Screen", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
  });

  it("opens in English on first launch, updates UI immediately on flag click without saving, and persists only on Continue", async () => {
    render(<App />);

    // 1. Blurred onboarding overlay appears on first open with initial language = English (en_US)
    const modal = screen.getByTestId("language-onboarding-modal");
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
    expect(localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBeNull();

    // 2. Clicking the Brazil flag immediately translates BOTH the modal and the app UI to Portuguese,
    //    WITHOUT saving to localStorage or closing the modal yet.
    fireEvent.click(ptBtn);

    expect(screen.getByTestId("language-onboarding-modal")).toBeInTheDocument();
    expect(ptBtn).toHaveAttribute("aria-checked", "true");
    expect(enBtn).toHaveAttribute("aria-checked", "false");
    expect(document.documentElement.lang).toBe("pt-BR");

    expect(modal).toHaveTextContent("Escolha seu idioma");
    expect(continueBtn).toHaveTextContent("Continuar");
    expect(screen.getByTestId("status-disconnected")).toHaveTextContent(/Aguardando cliente do LoL/i);
    expect(screen.getByRole("button", { name: /^Configurações$/i })).toBeInTheDocument();
    expect(localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBeNull();

    // Clicking USA flag switches immediately back to English without saving
    fireEvent.click(enBtn);
    expect(modal).toHaveTextContent("Choose your language");
    expect(continueBtn).toHaveTextContent("Continue");
    expect(localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBeNull();

    // Switch back to Portuguese and click Continue to persist and dismiss
    fireEvent.click(ptBtn);
    fireEvent.click(continueBtn);

    await waitFor(() => {
      expect(screen.queryByTestId("language-onboarding-modal")).not.toBeInTheDocument();
    });

    expect(localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBe("pt_BR");
    expect(invoke).toHaveBeenCalledWith("set_locale", { locale: "pt_BR" });
    expect(screen.getByRole("button", { name: /^Configurações$/i })).toBeInTheDocument();
  });

  it("persists English when clicking Continue directly on first launch and skips onboarding on subsequent launches", async () => {
    const { unmount } = render(<App />);

    expect(screen.getByTestId("language-onboarding-modal")).toBeInTheDocument();
    fireEvent.click(screen.getByTestId("onboarding-continue-btn"));

    await waitFor(() => {
      expect(screen.queryByTestId("language-onboarding-modal")).not.toBeInTheDocument();
    });

    expect(localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBe("en_US");
    expect(invoke).toHaveBeenCalledWith("set_locale", { locale: "en_US" });

    // Re-mounting App skips onboarding since language is already persisted
    unmount();
    render(<App />);
    expect(screen.queryByTestId("language-onboarding-modal")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /^Settings$/i })).toBeInTheDocument();
  });
});

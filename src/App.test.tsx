import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import App, { FLASH_ONBOARDING_STORAGE_KEY } from "./App";
import { invoke } from "@tauri-apps/api/core";
import { defaultAppInvokeMock, mockAhriBuild } from "./__tests__/appTestFixtures";

let eventListeners: Record<string, (event: { payload: unknown }) => void> = {};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args?: Record<string, unknown>) =>
    defaultAppInvokeMock(cmd, args)
  ),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockImplementation((event: string, cb: (payload: unknown) => void) => {
    eventListeners[event] = cb;
    return Promise.resolve(() => {
      delete eventListeners[event];
    });
  }),
}));

describe("App Component", () => {
  beforeEach(() => {
    eventListeners = {};
    localStorage.clear();
    localStorage.setItem("schumaz_language", "pt_BR");
    localStorage.setItem(FLASH_ONBOARDING_STORAGE_KEY, "true");
  });

  it("renders schumaz.gg title, initial LCU disconnected status, and starts on Perfil tab", () => {
    render(<App />);
    expect(screen.getByText("schumaz.gg")).toBeInTheDocument();
    expect(screen.getByTestId("status-disconnected")).toHaveTextContent(/Aguardando cliente do LoL/i);
    expect(screen.getByTestId("profile-waiting-client")).toHaveTextContent(/Aguardando Cliente do LoL/i);
  });

  it("does not render manual injection buttons as injection is strictly automatic", () => {
    render(<App />);
    expect(screen.queryByRole("button", { name: /Testar Injeção/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Reaplicar Build/i })).not.toBeInTheDocument();
  });

  it("renders only waiting state initially, and renders all 5 shop item blocks once build is received", async () => {
    render(<App />);
    expect(screen.getByText(/Aguardando Seleção de Campeões/i)).toBeInTheDocument();
    expect(screen.getByText("schumaz.gg · Runas")).toBeInTheDocument();
    const runesWaiting = screen.getByTestId("runes-waiting-champ");
    expect(runesWaiting.querySelector(".idle-radar-ring")).toBeInTheDocument();
    expect(screen.getByTestId("runes-waiting-champ-dots").querySelectorAll(".loading-dot")).toHaveLength(3);
    expect(screen.queryByText(/Estrutura dos Conjuntos de Itens na Loja/i)).not.toBeInTheDocument();
    expect(screen.queryByTestId("item-blocks")).not.toBeInTheDocument();

    await waitFor(() => {
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    act(() => {
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    await waitFor(() => {
      expect(screen.getByTestId("item-blocks")).toBeInTheDocument();
    });

    expect(screen.getByText(/1. Itens Iniciais/i)).toBeInTheDocument();
    expect(screen.getByText(/2. Primeiros 3 Itens/i)).toBeInTheDocument();
    expect(screen.getByText(/3. 3 Botas Mais Feitas/i)).toBeInTheDocument();
    expect(screen.getByText(/4. Build Final/i)).toBeInTheDocument();
    expect(screen.getByText(/5. Itens Situacionais \/ Úteis/i)).toBeInTheDocument();
  });

  it("renders the floating bottom dock with 5 navigation tabs in order: Perfil, Runas, Ranqueada, Aram and Configurações", () => {
    render(<App />);
    const dockNav = screen.getByRole("navigation", { name: /navegação principal/i });
    expect(dockNav).toBeInTheDocument();

    const buttons = Array.from(dockNav.querySelectorAll("button"));
    expect(buttons.map((b) => b.textContent?.trim())).toEqual(["Perfil", "Runas", "Ranqueada", "Aram", "Configurações"]);

    const profileBtn = screen.getByRole("button", { name: /^perfil$/i });
    const runesBtn = screen.getByRole("button", { name: /^runas$/i });
    const aramBtn = screen.getByRole("button", { name: /^aram$/i });
    const settingsBtn = screen.getByRole("button", { name: /^configurações$/i });

    expect(profileBtn).toHaveAttribute("aria-current", "page");
    expect(runesBtn).not.toHaveAttribute("aria-current");
    expect(aramBtn).not.toHaveAttribute("aria-current");
    expect(settingsBtn).not.toHaveAttribute("aria-current");
  });

  it("fetches player profile automatically when LCU connects and resets when disconnected", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["lcu-connected"]).toBeDefined();
      expect(eventListeners["lcu-disconnected"]).toBeDefined();
    });

    await act(async () => {
      eventListeners["lcu-connected"]({
        payload: {
          summonerId: 987654321,
          gameName: "Schuma",
          tagLine: "BR1",
          displayName: "Schuma",
          summonerLevel: 350,
          profileIconId: 548,
        },
      });
    });

    await waitFor(() => { expect(invoke).toHaveBeenCalledWith("get_player_profile", { forceRefresh: false }); });

    await act(async () => {
      eventListeners["lcu-disconnected"]({ payload: null });
    });

    expect(screen.getByTestId("profile-waiting-client")).toBeInTheDocument();
  });

  it("switches to the Configurações view and displays the settings groups", () => {
    render(<App />);
    const settingsBtn = screen.getByRole("button", { name: /^configurações$/i });
    fireEvent.click(settingsBtn);

    expect(settingsBtn).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("heading", { name: /Injeção Automática/i })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /Posicionamento do Flash/i })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: /Automação do Cliente/i })).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/Aceitar Partida Automaticamente/i)).not.toBeInTheDocument();

    expect(screen.getByLabelText("Injetar Runas Automaticamente")).toBeChecked();
    expect(screen.getByLabelText("Injetar Feitiços de Invocador")).toBeChecked();
    expect(screen.getByLabelText("Injetar Conjuntos de Itens na Loja")).toBeChecked();
    expect(screen.getByRole("button", { name: "D" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.queryByRole("heading", { name: /Critério da Build Recomendada/i })).not.toBeInTheDocument();
  });

  it("toggles preferences and persists them to localStorage", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /^configurações$/i }));

    const runesToggle = screen.getByLabelText("Injetar Runas Automaticamente");
    fireEvent.click(runesToggle);
    expect(runesToggle).not.toBeChecked();

    const flashFBtn = screen.getByRole("button", { name: "F" });
    fireEvent.click(flashFBtn);
    expect(flashFBtn).toHaveAttribute("aria-pressed", "true");

    const patchProtToggle = screen.getByTestId("patch-48h-protection-toggle");
    expect(patchProtToggle).toBeChecked();
    fireEvent.click(patchProtToggle);
    expect(screen.getByTestId("patch-48h-protection-warning")).toBeInTheDocument();
    fireEvent.click(screen.getByTestId("patch-48h-accept-btn"));
    expect(patchProtToggle).not.toBeChecked();

    const saved = JSON.parse(localStorage.getItem("schumaz_user_preferences") || "{}");
    expect(saved.autoInjectRunes).toBe(false);
    expect(saved.flashKey).toBe("F");
    expect(saved.patch48hProtection).toBe(false);
  });
});

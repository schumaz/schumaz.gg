import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import App from "./App";

// Mock Tauri API calls for unit testing
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd) => {
    if (cmd === "get_summoner_status") {
      return Promise.resolve(null);
    }
    if (cmd === "test_inject_runes") {
      return Promise.resolve({
        champion_name: "Ahri",
        runes_injected: true,
        spells_injected: false,
        items_injected: true,
        message: "Runas injetadas | Itens injetados",
      });
    }
    return Promise.resolve({});
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

describe("App Component", () => {
  it("renders schumaz.gg title and initial LCU disconnected status", () => {
    render(<App />);
    expect(screen.getByText("schumaz.gg")).toBeInTheDocument();
    expect(
      screen.getByText(/Aguardando cliente do LoL/i)
    ).toBeInTheDocument();
  });

  it("renders the manual injection test button in the Runas tab", () => {
    render(<App />);
    const btn = screen.getByRole("button", {
      name: /Testar Injeção de Runas no LoL/i,
    });
    expect(btn).toBeInTheDocument();
    // Disabled initially when summoner is not connected
    expect(btn).toBeDisabled();
  });

  it("renders all 4 requested shop item block categories in the Runas tab", () => {
    render(<App />);
    expect(screen.getByText(/1. Itens Iniciais/i)).toBeInTheDocument();
    expect(screen.getByText(/2. Primeiros 3 Itens/i)).toBeInTheDocument();
    expect(screen.getByText(/3. Build Final/i)).toBeInTheDocument();
    expect(screen.getByText(/4. 3 Botas Mais Feitas/i)).toBeInTheDocument();
  });

  it("renders the floating bottom dock with exactly 2 navigation tabs: Runas and Configurações", () => {
    render(<App />);
    const dockNav = screen.getByRole("navigation", { name: /navegação principal/i });
    expect(dockNav).toBeInTheDocument();

    const runesBtn = screen.getByRole("button", { name: /^runas$/i });
    const settingsBtn = screen.getByRole("button", { name: /^configurações$/i });

    expect(runesBtn).toBeInTheDocument();
    expect(settingsBtn).toBeInTheDocument();

    // Runas tab is active by default
    expect(runesBtn).toHaveAttribute("aria-current", "page");
    expect(settingsBtn).not.toHaveAttribute("aria-current");
  });

  it("switches to the Configurações view when the settings dock button is clicked", () => {
    render(<App />);
    const runesBtn = screen.getByRole("button", { name: /^runas$/i });
    const settingsBtn = screen.getByRole("button", { name: /^configurações$/i });

    // Initially in Runas tab
    expect(screen.getByText("Seleção de Campeões")).toBeInTheDocument();

    // Click Configurações
    fireEvent.click(settingsBtn);

    // Settings tab is now active
    expect(settingsBtn).toHaveAttribute("aria-current", "page");
    expect(runesBtn).not.toHaveAttribute("aria-current");
    expect(screen.getByText("Ajustes & Preferências")).toBeInTheDocument();

    // Click back to Runas
    fireEvent.click(runesBtn);
    expect(runesBtn).toHaveAttribute("aria-current", "page");
    expect(screen.getByText("Seleção de Campeões")).toBeInTheDocument();
  });
});

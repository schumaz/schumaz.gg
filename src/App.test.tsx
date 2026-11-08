import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
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
  it("renders schumaz.gg title and initial LCU disconnected status", async () => {
    render(<App />);
    expect(screen.getByText("schumaz.gg")).toBeInTheDocument();
    expect(
      screen.getByText(/Aguardando cliente do LoL/i)
    ).toBeInTheDocument();
  });

  it("renders the manual injection test button", () => {
    render(<App />);
    const btn = screen.getByRole("button", {
      name: /Testar Injeção de Runas no LoL/i,
    });
    expect(btn).toBeInTheDocument();
    // Disabled initially when summoner is not connected
    expect(btn).toBeDisabled();
  });

  it("renders all 4 requested shop item block categories", () => {
    render(<App />);
    expect(screen.getByText(/1. Itens Iniciais/i)).toBeInTheDocument();
    expect(screen.getByText(/2. Primeiros 3 Itens/i)).toBeInTheDocument();
    expect(screen.getByText(/3. Build Final/i)).toBeInTheDocument();
    expect(screen.getByText(/4. 3 Botas Mais Feitas/i)).toBeInTheDocument();
  });
});

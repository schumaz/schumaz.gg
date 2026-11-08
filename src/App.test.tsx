import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import App, { ChampionBuild } from "./App";

// Sample mock recommended build for Ahri
const mockAhriBuild: ChampionBuild = {
  champion_id: 103,
  champion_name: "Ahri",
  win_rate: 51.8,
  pick_rate: 8.4,
  primary_style_id: 8100,
  sub_style_id: 8200,
  selected_perk_ids: [8112, 8126, 8138, 8135, 8226, 8210, 5008, 5008, 5001],
  spell1_id: 4,
  spell2_id: 14,
  champion_icon_url: "https://ddragon.leagueoflegends.com/cdn/14.23.1/img/champion/Ahri.png",
  primary_style_name: "Dominação",
  primary_style_icon_url: "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/7200_Domination.png",
  sub_style_name: "Feitiçaria",
  sub_style_icon_url: "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/7202_Sorcery.png",
  perks: [
    { id: 8112, key: "Electrocute", name: "Eletrocutar", icon_url: "electrocute.png" },
    { id: 8126, key: "CheapShot", name: "Golpe Desleal", icon_url: "cheapshot.png" },
    { id: 8138, key: "EyeballCollection", name: "Globos Oculares", icon_url: "eyeball.png" },
    { id: 8135, key: "TreasureHunter", name: "Caçador de Tesouros", icon_url: "treasure.png" },
    { id: 8226, key: "ManaflowBand", name: "Faixa de Fluxo", icon_url: "manaflow.png" },
    { id: 8210, key: "Transcendence", name: "Transcendência", icon_url: "transcendence.png" },
    { id: 5008, key: "AdaptiveForce", name: "Força Adaptativa", icon_url: "adaptive.png" },
    { id: 5008, key: "AdaptiveForce", name: "Força Adaptativa", icon_url: "adaptive.png" },
    { id: 5001, key: "HealthScaling", name: "Vida Escalonável", icon_url: "health.png" },
  ],
  spells: [
    { id: 4, name: "Flash", icon_url: "flash.png" },
    { id: 14, name: "Incendiar", icon_url: "ignite.png" },
  ],
  item_blocks: [
    { title: "Itens Iniciais", item_ids: [1056, 2003], item_icon_urls: ["1056.png", "2003.png"] },
    { title: "Primeiros 3 Itens", item_ids: [3285, 3020, 3089], item_icon_urls: ["3285.png", "3020.png", "3089.png"] },
    { title: "Build Final", item_ids: [3285, 3020, 3089, 3157, 3135], item_icon_urls: ["3285.png", "3020.png", "3089.png", "3157.png", "3135.png"] },
    { title: "3 Botas Mais Feitas", item_ids: [3020, 3111, 3047], item_icon_urls: ["3020.png", "3111.png", "3047.png"] },
  ],
};

let eventListeners: Record<string, (event: { payload: unknown }) => void> = {};

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
    if (cmd === "inject_build_for_champion") {
      return Promise.resolve({
        champion_name: "Ahri",
        runes_injected: true,
        spells_injected: true,
        items_injected: true,
        message: "Runas injetadas | Feitiços injetados | Itens injetados",
      });
    }
    if (cmd === "get_recommended_build_for_champion") {
      return Promise.resolve(mockAhriBuild);
    }
    return Promise.resolve({});
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

describe("App Component", () => {
  beforeEach(() => {
    eventListeners = {};
  });

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
      name: /Testar Injeção/i,
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
    expect(screen.getByText(/Seleção de Campeões/i)).toBeInTheDocument();

    // Click Configurações
    fireEvent.click(settingsBtn);

    // Settings tab is now active
    expect(settingsBtn).toHaveAttribute("aria-current", "page");
    expect(runesBtn).not.toHaveAttribute("aria-current");
    expect(screen.getByText("Ajustes & Preferências")).toBeInTheDocument();

    // Click back to Runas
    fireEvent.click(runesBtn);
    expect(runesBtn).toHaveAttribute("aria-current", "page");
    expect(screen.getByText(/Seleção de Campeões/i)).toBeInTheDocument();
  });

  it("renders full match dashboard when recommended build is received", async () => {
    render(<App />);

    // Wait until event listeners are established
    await waitFor(() => {
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    // Simulate recommended-build event delivery from backend
    act(() => {
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    await waitFor(() => {
      expect(screen.getByTestId("champ-hero")).toBeInTheDocument();
    });

    // Check champion name, win rate and pick rate
    expect(screen.getByText("Ahri")).toBeInTheDocument();
    expect(screen.getByText(/51.8% Win Rate/i)).toBeInTheDocument();
    expect(screen.getByText(/8.4% Pick Rate/i)).toBeInTheDocument();

    // Check prominent quick action button
    const reapplyBtn = screen.getByTestId("reapply-build-btn");
    expect(reapplyBtn).toBeInTheDocument();
    expect(reapplyBtn).toHaveTextContent("Reaplicar Build Agora");

    // Check runes tree
    expect(screen.getByText("Dominação")).toBeInTheDocument();
    expect(screen.getByText("Eletrocutar")).toBeInTheDocument();
    expect(screen.getByText("Feitiçaria")).toBeInTheDocument();
    expect(screen.getByText("Fragmentos de Atributo")).toBeInTheDocument();

    // Check summoner spells
    expect(screen.getByText("Flash")).toBeInTheDocument();
    expect(screen.getByText("Incendiar")).toBeInTheDocument();

    // Check shop item blocks
    expect(screen.getByText(/1. Itens Iniciais/i)).toBeInTheDocument();
    expect(screen.getByText(/2. Primeiros 3 Itens/i)).toBeInTheDocument();
    expect(screen.getByText(/3. Build Final/i)).toBeInTheDocument();
    expect(screen.getByText(/4. 3 Botas Mais Feitas/i)).toBeInTheDocument();
  });
});

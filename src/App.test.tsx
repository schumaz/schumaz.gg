import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import App, { ChampionBuild } from "./App";
import { invoke } from "@tauri-apps/api/core";

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
    { title: "Primeiros 3 Itens", item_ids: [3285, 3157, 3089], item_icon_urls: ["3285.png", "3157.png", "3089.png"] },
    { title: "3 Botas Mais Feitas", item_ids: [3020, 3111, 3047], item_icon_urls: ["3020.png", "3111.png", "3047.png"] },
    { title: "Build Final", item_ids: [3285, 3157, 3089, 3135, 4645], item_icon_urls: ["3285.png", "3157.png", "3089.png", "3135.png", "4645.png"] },
    { title: "Itens Situacionais / Úteis", item_ids: [4629, 3165, 3102], item_icon_urls: ["4629.png", "3165.png", "3102.png"] },
  ],
  active_position: "mid",
  active_tier: "emerald_plus",
  active_region: "global",
  active_patch: "16.22",
  available_patches: ["16.22", "16.21"],
  patch_fallback_applied: false,
};

let eventListeners: Record<string, (event: { payload: unknown }) => void> = {};

// Mock Tauri API calls for unit testing
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd, args) => {
    if (cmd === "get_summoner_status") {
      return Promise.resolve(null);
    }

    if (cmd === "get_recommended_build_for_champion") {
      return Promise.resolve(mockAhriBuild);
    }

    if (cmd === "update_build_filters") {
      return Promise.resolve({
        ...mockAhriBuild,
        active_position: args?.position || mockAhriBuild.active_position,
        active_tier: args?.tier || mockAhriBuild.active_tier,
        active_region: args?.region || mockAhriBuild.active_region,
        active_patch: args?.patch || mockAhriBuild.active_patch,
      });
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
    localStorage.clear();
  });

  it("renders schumaz.gg title and initial LCU disconnected status", () => {
    render(<App />);
    expect(screen.getByText("schumaz.gg")).toBeInTheDocument();
    expect(
      screen.getByText(/Aguardando cliente do LoL/i)
    ).toBeInTheDocument();
  });

  it("does not render manual injection buttons as injection is strictly automatic", () => {
    render(<App />);
    expect(screen.queryByRole("button", { name: /Testar Injeção/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Reaplicar Build/i })).not.toBeInTheDocument();
  });

  it("renders only waiting state initially, and renders all 5 shop item blocks once build is received", async () => {
    render(<App />);
    // Initial idle screen displays only waiting for champion select
    expect(screen.getByText(/Aguardando Seleção de Campeões/i)).toBeInTheDocument();
    expect(screen.queryByText(/Estrutura dos Conjuntos de Itens na Loja/i)).not.toBeInTheDocument();
    expect(screen.queryByTestId("item-blocks")).not.toBeInTheDocument();

    // When build is delivered, all 5 blocks appear
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

  it("switches to the Configurações view and displays the settings groups", () => {
    render(<App />);
    const settingsBtn = screen.getByRole("button", { name: /^configurações$/i });

    // Click Configurações
    fireEvent.click(settingsBtn);

    // Settings tab is now active
    expect(settingsBtn).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("heading", { name: /Injeção Automática/i })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /Posicionamento do Flash/i })).toBeInTheDocument();

    // Ensure Match Automation / Auto-Accept is strictly not rendered
    expect(screen.queryByRole("heading", { name: /Automação do Cliente/i })).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/Aceitar Partida Automaticamente/i)).not.toBeInTheDocument();

    // Check individual controls
    expect(screen.getByLabelText("Injetar Runas Automaticamente")).toBeChecked();
    expect(screen.getByLabelText("Injetar Feitiços de Invocador")).toBeChecked();
    expect(screen.getByLabelText("Injetar Conjuntos de Itens na Loja")).toBeChecked();
    expect(screen.getByRole("button", { name: /Flash no D/i })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("heading", { name: /Critério da Build Recomendada/i })).toBeInTheDocument();
    expect(screen.getByTestId("priority-most-popular")).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByTestId("priority-highest-winrate")).toHaveAttribute("aria-pressed", "false");
  });

  it("toggles preferences and persists them to localStorage", () => {
    render(<App />);
    const settingsBtn = screen.getByRole("button", { name: /^configurações$/i });
    fireEvent.click(settingsBtn);

    // Toggle runes off
    const runesToggle = screen.getByLabelText("Injetar Runas Automaticamente");
    fireEvent.click(runesToggle);
    expect(runesToggle).not.toBeChecked();

    // Switch Flash to F
    const flashFBtn = screen.getByRole("button", { name: /Flash no F/i });
    fireEvent.click(flashFBtn);
    expect(flashFBtn).toHaveAttribute("aria-pressed", "true");

    // Check localStorage
    const saved = JSON.parse(localStorage.getItem("schumaz_user_preferences") || "{}");
    expect(saved.autoInjectRunes).toBe(false);
    expect(saved.flashKey).toBe("F");
  });

  it("swaps summoner spell slots and supports inverting spells with immediate injection in champion select", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    act(() => {
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    await waitFor(() => {
      expect(screen.getByTestId("spells-card")).toBeInTheDocument();
    });

    // Ensure injection report is strictly not rendered in spells-card
    expect(screen.queryByTestId("injection-report")).not.toBeInTheDocument();

    // Default: Flash on D (spell index 0), Ignite on F (spell index 1)
    const spellItemsBefore = screen.getAllByText(/Flash|Incendiar/);
    expect(spellItemsBefore[0]).toHaveTextContent("Flash");
    expect(spellItemsBefore[1]).toHaveTextContent("Incendiar");

    // Click "Inverter Feitiços" button
    const invertBtn = screen.getByTestId("invert-spells-btn");
    expect(invertBtn).toBeInTheDocument();
    expect(invertBtn).toHaveTextContent(/Inverter Feitiços/i);

    await act(async () => {
      fireEvent.click(invertBtn);
    });

    // Now Ignite is in slot D and Flash is in slot F
    const spellItemsAfter = screen.getAllByText(/Flash|Incendiar/);
    expect(spellItemsAfter[0]).toHaveTextContent("Incendiar");
    expect(spellItemsAfter[1]).toHaveTextContent("Flash");

    // Verify invoke was called for inject_summoner_spells with the inverted pair (Ignite 14, Flash 4)
    expect(invoke).toHaveBeenCalledWith("inject_summoner_spells", {
      spell1Id: 14,
      spell2Id: 4,
    });

    // Check localStorage reflects the unified preference (F)
    const saved = JSON.parse(localStorage.getItem("schumaz_user_preferences") || "{}");
    expect(saved.flashKey).toBe("F");

    // Test changing flashKey from Settings also immediately injects spells if in active draft
    const settingsBtn = screen.getByRole("button", { name: /^configurações$/i });
    fireEvent.click(settingsBtn);

    const flashDBtn = screen.getByRole("button", { name: /Flash no D/i });
    await act(async () => {
      fireEvent.click(flashDBtn);
    });

    // Switching to Flash on D immediately injects Flash (4) on D and Ignite (14) on F
    expect(invoke).toHaveBeenCalledWith("inject_summoner_spells", {
      spell1Id: 4,
      spell2Id: 14,
    });
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

    // Manual reapply button must not exist in automatic mode
    expect(screen.queryByTestId("reapply-build-btn")).not.toBeInTheDocument();

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
    expect(screen.getByText(/3. 3 Botas Mais Feitas/i)).toBeInTheDocument();
    expect(screen.getByText(/4. Build Final/i)).toBeInTheDocument();
    expect(screen.getByText(/5. Itens Situacionais \/ Úteis/i)).toBeInTheDocument();
  });

  it("renders the 4 filter selectors (Elo, Rota, Servidor, Patch) and handles filter updates", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    act(() => {
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    await waitFor(() => {
      expect(screen.getByTestId("champ-filters-bar")).toBeInTheDocument();
    });

    // Check 4 select elements exist with correct initial values
    const tierSelect = screen.getByTestId("filter-tier") as HTMLSelectElement;
    const posSelect = screen.getByTestId("filter-position") as HTMLSelectElement;
    const regionSelect = screen.getByTestId("filter-region") as HTMLSelectElement;
    const patchSelect = screen.getByTestId("filter-patch") as HTMLSelectElement;

    expect(tierSelect.value).toBe("emerald_plus");
    const tierOptionValues = Array.from(tierSelect.options).map((opt) => opt.value);
    expect(tierOptionValues).toEqual([
      "all",
      "challenger",
      "grandmaster",
      "master_plus",
      "diamond_plus",
      "emerald_plus",
      "platinum_plus",
      "gold_plus",
      "silver",
      "bronze",
      "iron",
    ]);
    expect(posSelect.value).toBe("mid");
    expect(regionSelect.value).toBe("global");
    expect(patchSelect.value).toBe("16.22");

    // Change Rota to adc
    await act(async () => {
      fireEvent.change(posSelect, { target: { value: "adc" } });
    });

    expect(invoke).toHaveBeenCalledWith("update_build_filters", {
      championId: 103,
      queueId: 420,
      position: "adc",
      tier: "emerald_plus",
      region: "global",
      patch: "16.22",
    });

    // Change Elo to diamond_plus
    await act(async () => {
      fireEvent.change(tierSelect, { target: { value: "diamond_plus" } });
    });

    expect(invoke).toHaveBeenCalledWith("update_build_filters", expect.objectContaining({
      championId: 103,
      tier: "diamond_plus",
    }));
  });

  it("displays the patch fallback badge when latest patch is less than 48 hours old", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    act(() => {
      eventListeners["recommended-build"]({
        payload: {
          ...mockAhriBuild,
          active_patch: "16.21",
          patch_fallback_applied: true,
        },
      });
    });

    await waitFor(() => {
      expect(screen.getByTestId("patch-fallback-badge")).toBeInTheDocument();
    });

    expect(screen.getByText(/Volume Protegido \(16\.21\)/i)).toBeInTheDocument();
  });

  it("displays Practice Tool badge and ranked items when practice tool session is detected", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["champ-selected"]).toBeDefined();
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    // Simulate champ-selected in Practice Tool
    act(() => {
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 103,
          queue_id: -1,
          queue_mode: "PracticeTool",
          locked_in: true,
        },
      });
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    await waitFor(() => {
      expect(screen.getByTestId("champ-hero")).toBeInTheDocument();
    });

    expect(screen.getByText("Modo Treino (Build Ranqueada)")).toBeInTheDocument();
    expect(screen.getByText(/1. Itens Iniciais/i)).toBeInTheDocument();
    expect(screen.getByText(/2. Primeiros 3 Itens/i)).toBeInTheDocument();
    expect(screen.getByText(/3. 3 Botas Mais Feitas/i)).toBeInTheDocument();
    expect(screen.getByText(/4. Build Final/i)).toBeInTheDocument();
    expect(screen.getByText(/5. Itens Situacionais \/ Úteis/i)).toBeInTheDocument();
  });

  it("updates build priority preference and re-fetches build when champion select is active", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["champ-selected"]).toBeDefined();
    });

    // Enter champ select with Ahri
    act(() => {
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 103,
          queue_id: 420,
          queue_mode: "SummonersRift",
          locked_in: true,
        },
      });
    });

    // Switch to Settings tab
    act(() => {
      const settingsBtn = screen.getByRole("button", { name: /^configurações$/i });
      fireEvent.click(settingsBtn);
    });

    // Switch priority to highest win rate
    const winrateBtn = screen.getByTestId("priority-highest-winrate");
    await act(async () => {
      fireEvent.click(winrateBtn);
    });

    expect(winrateBtn).toHaveAttribute("aria-pressed", "true");
    expect(invoke).toHaveBeenCalledWith("get_recommended_build_for_champion", {
      championId: 103,
      queueId: 420,
    });
  });
});

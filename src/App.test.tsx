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
    rune_pages: [
    {
      primary_style_id: 8100,
      sub_style_id: 8200,
      selected_perk_ids: [8112, 8126, 8138, 8135, 8226, 8210, 5008, 5008, 5001],
      play: 45000,
      win: 23500,
      pick_rate: 62.5,
      primary_style_name: "Dominação",
      primary_style_icon_url: "domination.png",
      sub_style_name: "Feitiçaria",
      sub_style_icon_url: "sorcery.png",
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
    },
    {
      primary_style_id: 8300,
      sub_style_id: 8100,
      selected_perk_ids: [8351, 8304, 8345, 8352, 8126, 8135, 5008, 5008, 5001],
      play: 12000,
      win: 6200,
      pick_rate: 16.7,
      primary_style_name: "Inspiração",
      primary_style_icon_url: "inspiration.png",
      sub_style_name: "Dominação",
      sub_style_icon_url: "domination.png",
      perks: [
        { id: 8351, key: "GlacialAugment", name: "Aprimoramento Glacial", icon_url: "glacial.png" },
        { id: 8304, key: "MagicalFootwear", name: "Calçados Mágicos", icon_url: "footwear.png" },
        { id: 8345, key: "BiscuitDelivery", name: "Entrega de Biscoitos", icon_url: "biscuits.png" },
        { id: 8352, key: "TimeWarpTonic", name: "Tônico de Distorção", icon_url: "timewarp.png" },
        { id: 8126, key: "CheapShot", name: "Golpe Desleal", icon_url: "cheapshot.png" },
        { id: 8135, key: "TreasureHunter", name: "Caçador de Tesouros", icon_url: "treasure.png" },
        { id: 5008, key: "AdaptiveForce", name: "Força Adaptativa", icon_url: "adaptive.png" },
        { id: 5008, key: "AdaptiveForce", name: "Força Adaptativa", icon_url: "adaptive.png" },
        { id: 5001, key: "HealthScaling", name: "Vida Escalonável", icon_url: "health.png" },
      ],
    },
    {
      primary_style_id: 8200,
      sub_style_id: 8300,
      selected_perk_ids: [8229, 8226, 8210, 8237, 8304, 8345, 5008, 5008, 5001],
      play: 8000,
      win: 4100,
      pick_rate: 11.1,
      primary_style_name: "Feitiçaria",
      primary_style_icon_url: "sorcery.png",
      sub_style_name: "Inspiração",
      sub_style_icon_url: "inspiration.png",
      perks: [
        { id: 8229, key: "ArcaneComet", name: "Cometa Arcano", icon_url: "comet.png" },
        { id: 8226, key: "ManaflowBand", name: "Faixa de Fluxo", icon_url: "manaflow.png" },
        { id: 8210, key: "Transcendence", name: "Transcendência", icon_url: "transcendence.png" },
        { id: 8237, key: "Scorch", name: "Chamuscar", icon_url: "scorch.png" },
        { id: 8304, key: "MagicalFootwear", name: "Calçados Mágicos", icon_url: "footwear.png" },
        { id: 8345, key: "BiscuitDelivery", name: "Entrega de Biscoitos", icon_url: "biscuits.png" },
        { id: 5008, key: "AdaptiveForce", name: "Força Adaptativa", icon_url: "adaptive.png" },
        { id: 5008, key: "AdaptiveForce", name: "Força Adaptativa", icon_url: "adaptive.png" },
        { id: 5001, key: "HealthScaling", name: "Vida Escalonável", icon_url: "health.png" },
      ],
    },
  ],
  spell_choices: [
    { id: 14, name: "Incendiar", icon_url: "ignite.png" },
    { id: 12, name: "Teleporte", icon_url: "teleport.png" },
  ],
  spells: [
    { id: 4, name: "Flash", icon_url: "flash.png" },
    { id: 14, name: "Incendiar", icon_url: "ignite.png" },
  ],
    build_options: [
    {
      name: "Build 1 (Mais Feita)",
      core_item_ids: [3285, 3157, 3089],
      core_item_icon_urls: ["3285.png", "3157.png", "3089.png"],
      final_item_ids: [3285, 3157, 3089, 3135, 4645, 3102],
      final_item_icon_urls: ["3285.png", "3157.png", "3089.png", "3135.png", "4645.png", "3102.png"],
      situational_item_ids: [4629, 3165, 3102],
      situational_item_icon_urls: ["4629.png", "3165.png", "3102.png"],
      play: 25000,
      win: 13000,
      pick_rate: 65.0,
    },
    {
      name: "Build 2",
      core_item_ids: [3285, 4645, 3089],
      core_item_icon_urls: ["3285.png", "4645.png", "3089.png"],
      final_item_ids: [3285, 4645, 3089, 3157, 3135, 3102],
      final_item_icon_urls: ["3285.png", "4645.png", "3089.png", "3157.png", "3135.png", "3102.png"],
      situational_item_ids: [3165, 4629, 3116],
      situational_item_icon_urls: ["3165.png", "4629.png", "3116.png"],
      play: 8000,
      win: 4200,
      pick_rate: 20.8,
    },
    {
      name: "Build 3",
      core_item_ids: [3285, 3165, 3089],
      core_item_icon_urls: ["3285.png", "3165.png", "3089.png"],
      final_item_ids: [3285, 3165, 3089, 3157, 3135, 4629],
      final_item_icon_urls: ["3285.png", "3165.png", "3089.png", "3157.png", "3135.png", "4629.png"],
      situational_item_ids: [4645, 3102, 3041],
      situational_item_icon_urls: ["4645.png", "3102.png", "3041.png"],
      play: 3000,
      win: 1550,
      pick_rate: 7.8,
    },
  ],
  core_build_options: [
    {
      item_ids: [3285, 3157, 3089],
      item_icon_urls: ["3285.png", "3157.png", "3089.png"],
      play: 25000,
      win: 13000,
      pick_rate: 65.0,
    },
    {
      item_ids: [3285, 4645, 3089],
      item_icon_urls: ["3285.png", "4645.png", "3089.png"],
      play: 8000,
      win: 4200,
      pick_rate: 20.8,
    },
    {
      item_ids: [3285, 3165, 3089],
      item_icon_urls: ["3285.png", "3165.png", "3089.png"],
      play: 3000,
      win: 1550,
      pick_rate: 7.8,
    },
  ],
  final_build_options: [
    {
      item_ids: [3285, 3157, 3089, 3135, 4645, 3102],
      item_icon_urls: ["3285.png", "3157.png", "3089.png", "3135.png", "4645.png", "3102.png"],
      play: 12000,
      win: 6500,
      pick_rate: 55.0,
    },
    {
      item_ids: [3285, 3157, 3089, 4629, 3135, 3102],
      item_icon_urls: ["3285.png", "3157.png", "3089.png", "4629.png", "3135.png", "3102.png"],
      play: 5000,
      win: 2700,
      pick_rate: 22.9,
    },
  ],
  item_blocks: [
    { title: "Itens Iniciais", item_ids: [1056, 2003], item_icon_urls: ["1056.png", "2003.png"] },
    { title: "Primeiros 3 Itens", item_ids: [3285, 3157, 3089], item_icon_urls: ["3285.png", "3157.png", "3089.png"] },
    { title: "3 Botas Mais Feitas", item_ids: [3020, 3111, 3047], item_icon_urls: ["3020.png", "3111.png", "3047.png"] },
    { title: "Build Final", item_ids: [3285, 3157, 3089, 3135, 4645, 3102], item_icon_urls: ["3285.png", "3157.png", "3089.png", "3135.png", "4645.png", "3102.png"] },
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

    if (cmd === "get_player_profile") {
      return Promise.resolve(null);
    }

    if (cmd === "get_recommended_build_for_champion") {
      if (args?.queueId === 2400) {
        return Promise.resolve({
          ...mockAhriBuild,
          active_position: "",
          supports_runes: false,
          rune_pages: [],
        });
      }
      if (args?.queueId === 450) {
        return Promise.resolve({
          ...mockAhriBuild,
          active_position: "",
          supports_runes: true,
        });
      }
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

    // Perfil tab is active by default
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

    expect(invoke).toHaveBeenCalledWith("get_player_profile");

    await act(async () => {
      eventListeners["lcu-disconnected"]({ payload: null });
    });

    expect(screen.getByTestId("profile-waiting-client")).toBeInTheDocument();
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
    expect(screen.getByRole("button", { name: "D" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.queryByRole("heading", { name: /Critério da Build Recomendada/i })).not.toBeInTheDocument();
    expect(screen.queryByTestId("priority-most-popular")).not.toBeInTheDocument();
    expect(screen.queryByTestId("priority-highest-winrate")).not.toBeInTheDocument();
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
    const flashFBtn = screen.getByRole("button", { name: "F" });
    fireEvent.click(flashFBtn);
    expect(flashFBtn).toHaveAttribute("aria-pressed", "true");

    // Toggle 48h patch protection off -> opens confirmation popup first
    const patchProtToggle = screen.getByTestId("patch-48h-protection-toggle");
    expect(patchProtToggle).toBeChecked();
    fireEvent.click(patchProtToggle);
    expect(screen.getByTestId("patch-48h-protection-warning")).toBeInTheDocument();
    expect(patchProtToggle).toBeChecked();
    fireEvent.click(screen.getByTestId("patch-48h-accept-btn"));
    expect(patchProtToggle).not.toBeChecked();

    // Check localStorage
    const saved = JSON.parse(localStorage.getItem("schumaz_user_preferences") || "{}");
    expect(saved.autoInjectRunes).toBe(false);
    expect(saved.flashKey).toBe("F");
    expect(saved.patch48hProtection).toBe(false);
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
      championId: null,
      secondarySpellId: null,
    });

    // Check localStorage reflects the unified preference (F)
    const saved = JSON.parse(localStorage.getItem("schumaz_user_preferences") || "{}");
    expect(saved.flashKey).toBe("F");

    // Test changing flashKey from Settings also immediately injects spells if in active draft
    const settingsBtn = screen.getByRole("button", { name: /^configurações$/i });
    fireEvent.click(settingsBtn);

    const flashDBtn = screen.getByRole("button", { name: "D" });
    await act(async () => {
      fireEvent.click(flashDBtn);
    });

    // Switching to Flash on D immediately injects Flash (4) on D and Ignite (14) on F
    expect(invoke).toHaveBeenCalledWith("inject_summoner_spells", {
      spell1Id: 4,
      spell2Id: 14,
      championId: null,
      secondarySpellId: null,
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

    // Check champion name and pick rate (win rate badge is deprecated and removed)
    expect(screen.getByText("Ahri")).toBeInTheDocument();
    expect(screen.queryByText(/Win Rate/i)).not.toBeInTheDocument();
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
    expect(screen.getAllByText("Incendiar")[0]).toBeInTheDocument();

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

    // Check 4 filter controls exist with correct initial values
    const tierSelect = screen.getByTestId("filter-tier");
    const posSelect = screen.getByTestId("filter-position");
    const regionSelect = screen.getByTestId("filter-region");
    const patchSelect = screen.getByTestId("filter-patch");

    expect(tierSelect).toHaveAttribute("data-value", "emerald_plus");
    fireEvent.click(tierSelect);
    const tierOptionValues = Array.from(
      screen.getByRole("listbox", { name: "Elo:" }).querySelectorAll('[role="option"]')
    ).map((opt) => opt.getAttribute("data-value"));
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
    fireEvent.keyDown(tierSelect, { key: "Escape" });
    expect(posSelect).toHaveAttribute("data-value", "mid");
    expect(regionSelect).toHaveAttribute("data-value", "global");
    expect(patchSelect).toHaveAttribute("data-value", "16.22");

    // Change Rota to adc
    await act(async () => {
      fireEvent.click(posSelect);
    });
    await act(async () => {
      fireEvent.click(screen.getByRole("option", { name: "Atirador (Bot)" }));
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
      fireEvent.click(tierSelect);
    });
    await act(async () => {
      fireEvent.click(screen.getByRole("option", { name: "Diamante +" }));
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

  it("does not render build priority controls or win rate badges in settings and dashboard", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    act(() => {
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    await waitFor(() => {
      expect(screen.getByTestId("champ-hero")).toBeInTheDocument();
    });

    // Win rate must strictly not be present in the dashboard
    expect(screen.queryByText(/Win Rate/i)).not.toBeInTheDocument();

    // Switch to Settings tab
    const settingsBtn = screen.getByRole("button", { name: /^configurações$/i });
    fireEvent.click(settingsBtn);

    // Build priority controls must strictly not exist
    expect(screen.queryByRole("heading", { name: /Critério da Build Recomendada/i })).not.toBeInTheDocument();
    expect(screen.queryByTestId("priority-most-popular")).not.toBeInTheDocument();
    expect(screen.queryByTestId("priority-highest-winrate")).not.toBeInTheDocument();
  });

  it("renders secondary spell choices and allows toggling to the alternative spell", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["champ-selected"]).toBeDefined();
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    act(() => {
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 103,
          queue_id: 420,
          queue_mode: "RankedSolo",
          locked_in: true,
        },
      });
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    await waitFor(() => {
      expect(screen.getByTestId("secondary-spell-selector")).toBeInTheDocument();
    });

    expect(screen.getByText("Escolha de Segundo Feitiço")).toBeInTheDocument();
    const igniteBtn = screen.getByTestId("sec-spell-btn-14");
    const tpBtn = screen.getByTestId("sec-spell-btn-12");
    expect(igniteBtn).toBeInTheDocument();
    expect(tpBtn).toBeInTheDocument();

    expect(igniteBtn).toHaveClass("active");
    expect(tpBtn).not.toHaveClass("active");

    act(() => {
      fireEvent.click(tpBtn);
    });

    expect(tpBtn).toHaveClass("active");
    expect(igniteBtn).not.toHaveClass("active");

    expect(invoke).toHaveBeenCalledWith("inject_summoner_spells", {
      spell1Id: 4,
      spell2Id: 12,
      championId: 103,
      secondarySpellId: 12,
    });
  });
  it("renders rune page selector with 3 most played options, displays keystone/sub-style and game count, and allows switching rune pages with LCU injection", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    act(() => {
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    await waitFor(() => {
      expect(screen.getByTestId("rune-pages-selector")).toBeInTheDocument();
    });

    // Check all 3 rune page buttons
    const opt0 = screen.getByTestId("rune-page-btn-0");
    const opt1 = screen.getByTestId("rune-page-btn-1");
    const opt2 = screen.getByTestId("rune-page-btn-2");

    expect(opt0).toBeInTheDocument();
    expect(opt1).toBeInTheDocument();
    expect(opt2).toBeInTheDocument();

    expect(opt0).toHaveClass("active");
    expect(opt1).not.toHaveClass("active");
    expect(opt2).not.toHaveClass("active");

    expect(screen.getByText("Opção 1 (Mais Usada)")).toBeInTheDocument();
    expect(screen.getAllByText("Opção 2").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Opção 3").length).toBeGreaterThan(0);
    expect(screen.getByText(/45[.,]000 partidas/i)).toBeInTheDocument();
    expect(screen.getByText(/12[.,]000 partidas/i)).toBeInTheDocument();

    // Default keystone is Electrocute
    expect(screen.getByText("Eletrocutar")).toBeInTheDocument();

    // Click Option 2 (Inspiration: Glacial Augment)
    act(() => {
      fireEvent.click(opt1);
    });

    expect(opt1).toHaveClass("active");
    expect(opt0).not.toHaveClass("active");

    // Displays Glacial Augment in the tree
    expect(screen.getByText("Aprimoramento Glacial")).toBeInTheDocument();

    // Injected immediately via LCU
    expect(invoke).toHaveBeenCalledWith("inject_rune_page", {
      championId: 103,
      championName: "Ahri",
      pageIndex: 1,
      primaryStyleId: 8300,
      subStyleId: 8100,
      selectedPerkIds: [8351, 8304, 8345, 8352, 8126, 8135, 5008, 5008, 5001],
    });
  });

  it("persists user manual rune page and secondary spell selections across draft lock-in for the same champion", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["champ-selected"]).toBeDefined();
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    // 1. Hover Ahri (not locked in)
    act(() => {
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 103,
          queue_id: 420,
          queue_mode: "RankedSolo",
          locked_in: false,
        },
      });
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    await waitFor(() => {
      expect(screen.getByTestId("sec-spell-btn-12")).toBeInTheDocument();
      expect(screen.getByTestId("rune-page-btn-1")).toBeInTheDocument();
    });

    // User chooses secondary spell Teleport (12) and rune page Option 2 (index 1)
    await act(async () => {
      fireEvent.click(screen.getByTestId("sec-spell-btn-12"));
      fireEvent.click(screen.getByTestId("rune-page-btn-1"));
    });

    expect(screen.getByTestId("sec-spell-btn-12")).toHaveClass("active");
    expect(screen.getByTestId("rune-page-btn-1")).toHaveClass("active");

    // 2. Draft Lock-In: LoL client emits champ-selected with locked_in: true for Ahri (103)
    await act(async () => {
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 103,
          queue_id: 420,
          queue_mode: "RankedSolo",
          locked_in: true,
        },
      });
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    // Selections MUST persist across lock-in!
    expect(screen.getByTestId("sec-spell-btn-12")).toHaveClass("active");
    expect(screen.getByTestId("rune-page-btn-1")).toHaveClass("active");
    expect(screen.getByText("Aprimoramento Glacial")).toBeInTheDocument();

    // 3. If a DIFFERENT champion is selected, selections reset back to default
    await act(async () => {
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 804,
          queue_id: 420,
          queue_mode: "RankedSolo",
          locked_in: false,
        },
      });
      eventListeners["recommended-build"]({
        payload: {
          ...mockAhriBuild,
          champion_id: 804,
          champion_name: "Yunara",
        },
      });
    });

    // Should reset to Option 1 (index 0) and default secondary spell
    expect(screen.getByTestId("rune-page-btn-0")).toHaveClass("active");
    expect(screen.getByTestId("rune-page-btn-1")).not.toHaveClass("active");
  });

  it("renders unified build selector with 3 cohesive build options and preview icons, allows selection with LCU injection, and persists choice across lock-in", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["champ-selected"]).toBeDefined();
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    // 1. Initial hover
    await act(async () => {
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 103,
          queue_id: 420,
          queue_mode: "RankedSolo",
          locked_in: false,
        },
      });
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    await waitFor(() => {
      expect(screen.getByTestId("unified-build-selector")).toBeInTheDocument();
    });

    // Verify 3 unified build buttons
    const buildOpt0 = screen.getByTestId("build-opt-btn-0");
    const buildOpt1 = screen.getByTestId("build-opt-btn-1");
    const buildOpt2 = screen.getByTestId("build-opt-btn-2");
    expect(buildOpt0).toBeInTheDocument();
    expect(buildOpt1).toBeInTheDocument();
    expect(buildOpt2).toBeInTheDocument();

    expect(buildOpt0).toHaveClass("active");
    expect(buildOpt1).not.toHaveClass("active");
    expect(buildOpt2).not.toHaveClass("active");

    // Verify preview mini-icons exist inside each button
    expect(buildOpt0.querySelectorAll(".build-pill-mini-icon")).toHaveLength(3);
    expect(buildOpt1.querySelectorAll(".build-pill-mini-icon")).toHaveLength(3);

    // Initial Block 2 and Block 4 reflect Build 1 (3157 in core)
    expect(screen.getAllByAltText("Item 3157").length).toBeGreaterThan(0);

    // Click Build Option 2 (index 1: core has 4645 Shadowflame)
    await act(async () => {
      fireEvent.click(buildOpt1);
    });

    expect(buildOpt1).toHaveClass("active");
    expect(buildOpt0).not.toHaveClass("active");
    expect(invoke).toHaveBeenCalledWith("inject_item_build", {
      championId: 103,
      queueId: 420,
      buildIndex: 1,
      coreIndex: 1,
      finalIndex: 1,
    });

    // 2. Draft Lock-in: Confirm champion Ahri (103)
    await act(async () => {
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 103,
          queue_id: 420,
          queue_mode: "RankedSolo",
          locked_in: true,
        },
      });
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    // Choice MUST persist across lock-in!
    expect(buildOpt1).toHaveClass("active");
    expect(buildOpt0).not.toHaveClass("active");
  });

  it("updates situational items dynamically when switching unified build options", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["champ-selected"]).toBeDefined();
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    await act(async () => {
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 103,
          queue_id: 420,
          queue_mode: "RankedSolo",
          locked_in: false,
        },
      });
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    await waitFor(() => {
      expect(screen.getByTestId("unified-build-selector")).toBeInTheDocument();
    });

    // Initially Build 1 situational items (item 3116 is in Build 2, not in Build 1)
    expect(screen.queryByAltText("Item 3116")).not.toBeInTheDocument();

    // Switch to Build Option 2
    const buildOpt1 = screen.getByTestId("build-opt-btn-1");
    await act(async () => {
      fireEvent.click(buildOpt1);
    });

    // Now Item 3116 should be rendered in the Situational block
    expect(screen.getByAltText("Item 3116")).toBeInTheDocument();
  });

  it("hides Rota selector, omits position in filter updates, and forwards active queueId to inject_item_build in ARAM (450)", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["champ-selected"]).toBeDefined();
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    await act(async () => {
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 103,
          queue_id: 450,
          queue_mode: "Aram",
          locked_in: true,
        },
      });
      eventListeners["recommended-build"]({
        payload: {
          ...mockAhriBuild,
          active_position: "",
          supports_runes: true,
        },
      });
    });

    await waitFor(() => {
      expect(screen.getByTestId("champ-filters-bar")).toBeInTheDocument();
    });

    expect(screen.getByText("ARAM (Howling Abyss)")).toBeInTheDocument();
    expect(screen.queryByTestId("filter-position")).not.toBeInTheDocument();

    const tierSelect = screen.getByTestId("filter-tier");
    await act(async () => {
      fireEvent.click(tierSelect);
    });
    await act(async () => {
      fireEvent.click(screen.getByRole("option", { name: "Diamante +" }));
    });

    expect(invoke).toHaveBeenCalledWith("update_build_filters", {
      championId: 103,
      queueId: 450,
      position: null,
      tier: "diamond_plus",
      region: "global",
      patch: "16.22",
    });

    const buildOpt1 = screen.getByTestId("build-opt-btn-1");
    await act(async () => {
      fireEvent.click(buildOpt1);
    });

    expect(invoke).toHaveBeenCalledWith("inject_item_build", {
      championId: 103,
      queueId: 450,
      buildIndex: 1,
      coreIndex: 1,
      finalIndex: 1,
    });
  });

  it("hides Rota selector and runes, omits position in filter updates, and forwards queueId 2400 in ARAM Mayhem", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["champ-selected"]).toBeDefined();
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    await act(async () => {
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 103,
          queue_id: 2400,
          queue_mode: "AramMayhem",
          locked_in: true,
        },
      });
      eventListeners["recommended-build"]({
        payload: {
          ...mockAhriBuild,
          active_position: "",
          supports_runes: false,
          rune_pages: [],
        },
      });
    });

    await waitFor(() => {
      expect(screen.getByTestId("champ-filters-bar")).toBeInTheDocument();
    });

    expect(screen.getByText("ARAM Mayhem (Howling Abyss)")).toBeInTheDocument();
    expect(screen.queryByTestId("filter-position")).not.toBeInTheDocument();
    expect(screen.queryByTestId("rune-pages-selector")).not.toBeInTheDocument();

    const tierSelect = screen.getByTestId("filter-tier");
    await act(async () => {
      fireEvent.click(tierSelect);
    });
    await act(async () => {
      fireEvent.click(screen.getByRole("option", { name: "Mestre +" }));
    });

    expect(invoke).toHaveBeenCalledWith("update_build_filters", {
      championId: 103,
      queueId: 2400,
      position: null,
      tier: "master_plus",
      region: "global",
      patch: "16.22",
    });

    const buildOpt1 = screen.getByTestId("build-opt-btn-1");
    await act(async () => {
      fireEvent.click(buildOpt1);
    });

    expect(invoke).toHaveBeenCalledWith("inject_item_build", {
      championId: 103,
      queueId: 2400,
      buildIndex: 1,
      coreIndex: 1,
      finalIndex: 1,
    });
  });

  it("renders full-screen profile wallpaper only on Perfil tab, applies blur slider from Configurações, and enforces 30s refresh cooldown", async () => {
    vi.mocked(invoke).mockImplementation((cmd: string, args?: any) => {
      if (cmd === "get_summoner_status") return Promise.resolve(null);
      if (cmd === "get_player_profile") {
        return Promise.resolve({
          summoner: {
            summonerId: 987654321,
            gameName: "Schuma",
            tagLine: "BR1",
            displayName: "Schuma",
            summonerLevel: 350,
            profileIconId: 548,
          },
          profile_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/profileicon/548.png",
          background_skin_id: 103015,
          background_skin_name: "",
          background_splash_url: "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_15.jpg",
          background_splash_fallback_url: "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_0.jpg",
          ranked_stats: [],
          top_masteries: [],
          recent_matches: [],
        });
      }
      if (cmd === "get_recommended_build_for_champion") return Promise.resolve(mockAhriBuild);
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
    });

    render(<App />);

    await waitFor(() => {
      expect(eventListeners["lcu-connected"]).toBeDefined();
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

    await waitFor(() => {
      expect(screen.getByTestId("profile-wallpaper")).toBeInTheDocument();
    });

    // Wallpaper blur stays fixed at 1px; profile UI defaults to 10% (3.2px)
    const wpImg = screen.getByTestId("profile-wallpaper").querySelector("img")!;
    expect(wpImg.style.filter).toContain("blur(1px)");
    expect(screen.getByTestId("view-profile").style.getPropertyValue("--profile-ui-blur")).toBe("3.2px");
    expect(screen.getByTestId("top-masteries-card").style.backdropFilter).toBe("blur(3.2px)");

    // Switch to Configurações, type 100% via clickable number and verify slider + persistence
    fireEvent.click(screen.getByRole("button", { name: /^configurações$/i }));
    expect(screen.queryByTestId("profile-wallpaper")).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId("profile-blur-value"));
    const numInput = screen.getByTestId("profile-blur-input");
    fireEvent.change(numInput, { target: { value: "100" } });
    fireEvent.keyDown(numInput, { key: "Enter" });
    expect(localStorage.getItem("schumaz_profile_ui_blur")).toBe("100");
    expect(screen.getByTestId("profile-blur-value")).toHaveTextContent("100%");

    // Return to Perfil: wallpaper stays at 1px fixed while profile interface blur updates to 32px
    fireEvent.click(screen.getByRole("button", { name: /^perfil$/i }));
    const wpImgAfter = screen.getByTestId("profile-wallpaper").querySelector("img")!;
    expect(wpImgAfter.style.filter).toContain("blur(1px)");
    expect(screen.getByTestId("view-profile").style.getPropertyValue("--profile-ui-blur")).toBe("32px");
    expect(screen.getByTestId("top-masteries-card").style.backdropFilter).toBe("blur(32px)");

    // Toggle Fundo Visível off in Configurações and confirm wallpaper hides in Perfil
    fireEvent.click(screen.getByRole("button", { name: /^configurações$/i }));
    fireEvent.click(screen.getByTestId("wallpaper-visible-toggle"));
    expect(localStorage.getItem("schumaz_profile_wallpaper_visible")).toBe("false");
    fireEvent.click(screen.getByRole("button", { name: /^perfil$/i }));
    expect(screen.queryByTestId("profile-wallpaper")).not.toBeInTheDocument();

    // Clicking Atualizar triggers 30s cooldown timer
    const refreshBtn = screen.getByTestId("refresh-profile-btn");
    await act(async () => {
      fireEvent.click(refreshBtn);
    });
    expect(refreshBtn).toBeDisabled();
    expect(refreshBtn).toHaveTextContent("Atualizar (30s)");
  });

  it("automatically navigates to Runas when entering a match and pulling a champion, and returns to Perfil when the match ends", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["champ-selected"]).toBeDefined();
      expect(eventListeners["recommended-build"]).toBeDefined();
      expect(eventListeners["gameflow-phase-changed"]).toBeDefined();
      expect(eventListeners["champ-select-ended"]).toBeDefined();
      expect(eventListeners["match-ended"]).toBeDefined();
    });

    const profileBtn = screen.getByRole("button", { name: /^perfil$/i });
    const runesBtn = screen.getByRole("button", { name: /^runas$/i });
    const settingsBtn = screen.getByRole("button", { name: /^configurações$/i });

    // Starts on Perfil
    expect(profileBtn).toHaveAttribute("aria-current", "page");
    expect(runesBtn).not.toHaveAttribute("aria-current");

    // User enters Champ Select and champion build is pulled
    await act(async () => {
      eventListeners["gameflow-phase-changed"]({ payload: "ChampSelect" });
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 103,
          queue_id: 420,
          queue_mode: "RankedSolo",
          locked_in: false,
        },
      });
    });

    // Automatically switches to Runas once the champion build is pulled
    await waitFor(() => {
      expect(runesBtn).toHaveAttribute("aria-current", "page");
    });
    expect(screen.getByTestId("view-runes")).toHaveClass("active-tab");
    expect(screen.getByTestId("champ-hero")).toBeInTheDocument();

    // If user manually switches to Configurações during the same champ select, lock-in does not override it
    fireEvent.click(settingsBtn);
    expect(settingsBtn).toHaveAttribute("aria-current", "page");

    await act(async () => {
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 103,
          queue_id: 420,
          queue_mode: "RankedSolo",
          locked_in: true,
        },
      });
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });
    expect(settingsBtn).toHaveAttribute("aria-current", "page");

    // User switches back to Runas, match enters InProgress and champ-select ends -> build stays visible
    fireEvent.click(runesBtn);
    await act(async () => {
      eventListeners["gameflow-phase-changed"]({ payload: "InProgress" });
      eventListeners["champ-select-ended"]({ payload: null });
    });
    expect(runesBtn).toHaveAttribute("aria-current", "page");
    expect(screen.getByTestId("champ-hero")).toBeInTheDocument();

    // Match ends -> automatically returns to Perfil and clears build
    await act(async () => {
      eventListeners["gameflow-phase-changed"]({ payload: "EndOfGame" });
      eventListeners["match-ended"]({ payload: null });
    });
    expect(profileBtn).toHaveAttribute("aria-current", "page");
    expect(runesBtn).not.toHaveAttribute("aria-current");
    expect(screen.queryByTestId("champ-hero")).not.toBeInTheDocument();

    // Entering a second match switches back to Runas automatically again
    await act(async () => {
      eventListeners["gameflow-phase-changed"]({ payload: "ChampSelect" });
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });
    expect(runesBtn).toHaveAttribute("aria-current", "page");
    expect(screen.getByTestId("champ-hero")).toBeInTheDocument();
  });

  it("keeps view-settings inactive and hidden unless Configurações tab is selected", () => {
    render(<App />);

    expect(screen.getByTestId("view-profile")).toHaveClass("active-tab");
    expect(screen.getByTestId("view-settings")).toHaveClass("inactive-tab");
    expect(screen.getByTestId("view-settings")).toHaveAttribute("aria-hidden", "true");

    fireEvent.click(screen.getByRole("button", { name: /^runas$/i }));
    expect(screen.getByTestId("view-runes")).toHaveClass("active-tab");
    expect(screen.getByTestId("view-settings")).toHaveClass("inactive-tab");
    expect(screen.getByTestId("view-settings")).toHaveAttribute("aria-hidden", "true");

    fireEvent.click(screen.getByRole("button", { name: /^configurações$/i }));
    expect(screen.getByTestId("view-settings")).toHaveClass("active-tab");
    expect(screen.getByTestId("view-settings")).toHaveAttribute("aria-hidden", "false");
    expect(screen.getByTestId("view-profile")).toHaveClass("inactive-tab");
  });
});

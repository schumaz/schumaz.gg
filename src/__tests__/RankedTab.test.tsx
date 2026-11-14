import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, within } from "@testing-library/react";
import { renderHook, act } from "@testing-library/react";
import { NavigationDock } from "../components/NavigationDock";
import { RankedTab } from "../components/RankedTab";
import { useRanked } from "../hooks/useRanked";
import {
  RANKED_FILTERS_STORAGE_KEY,
  commands,
} from "../types";
import type {
  ChampionBuild_Serialize,
  RankedChampionEntry_Serialize,
  RankedTierListResponse_Serialize,
} from "../types/bindings";

const mockRankedChampions: RankedChampionEntry_Serialize[] = [
  {
    id: 245,
    key: "ekko",
    name: "Ekko",
    icon_url: "https://example.com/ekko.png",
    lane: "mid",
    tier: 0,
    rank: 2,
    overall_rank: 1,
    win_rate: 52.2,
    pick_rate: 5.3,
    ban_rate: 3.4,
    role_rate: 55.0,
  },
  {
    id: 54,
    key: "malphite",
    name: "Malphite",
    icon_url: "https://example.com/malphite.png",
    lane: "top",
    tier: 0,
    rank: 1,
    overall_rank: 2,
    win_rate: 51.7,
    pick_rate: 6.8,
    ban_rate: 18.0,
    role_rate: 88.0,
  },
  {
    id: 103,
    key: "ahri",
    name: "Ahri",
    icon_url: "https://example.com/ahri.png",
    lane: "mid",
    tier: 0,
    rank: 1,
    overall_rank: 3,
    win_rate: 51.2,
    pick_rate: 11.5,
    ban_rate: 3.6,
    role_rate: 94.0,
  },
  {
    id: 86,
    key: "garen",
    name: "Garen",
    icon_url: "https://example.com/garen.png",
    lane: "top",
    tier: 1,
    rank: 2,
    overall_rank: 4,
    win_rate: 50.8,
    pick_rate: 6.2,
    ban_rate: 4.5,
    role_rate: 89.0,
  },
  {
    id: 245,
    key: "ekko",
    name: "Ekko",
    icon_url: "https://example.com/ekko.png",
    lane: "jungle",
    tier: 2,
    rank: 10,
    overall_rank: 5,
    win_rate: 50.7,
    pick_rate: 4.5,
    ban_rate: 3.4,
    role_rate: 45.0,
  },
];

const mockTierResponse: RankedTierListResponse_Serialize = {
  region: "global",
  tier: "emerald_plus",
  active_patch: "16.22",
  available_patches: ["16.22", "16.21"],
  patch_fallback_applied: false,
  patch_protection_remaining_hours: null,
  champions: mockRankedChampions,
};

const mockProtectedTierResponse: RankedTierListResponse_Serialize = {
  region: "global",
  tier: "emerald_plus",
  active_patch: "16.21",
  available_patches: ["16.22", "16.21"],
  patch_fallback_applied: true,
  patch_protection_remaining_hours: 38,
  champions: mockRankedChampions,
};

const mockAhriBuild: ChampionBuild_Serialize = {
  champion_id: 103,
  champion_name: "Ahri",
  win_rate: 51.2,
  pick_rate: 11.5,
  primary_style_id: 8100,
  sub_style_id: 8200,
  selected_perk_ids: [8112, 8126, 8138, 8135, 8226, 8210, 5008, 5008, 5001],
  spell1_id: 4,
  spell2_id: 14,
  champion_icon_url: "https://example.com/ahri.png",
  primary_style_name: "Dominação",
  primary_style_icon_url: "dom.png",
  sub_style_name: "Feitiçaria",
  sub_style_icon_url: "sorc.png",
  perks: [
    { id: 8112, key: "Electrocute", name: "Eletrocutar", icon_url: "elec.png" },
    { id: 8126, key: "CheapShot", name: "Golpe Desleal", icon_url: "cheap.png" },
    { id: 8138, key: "EyeballCollection", name: "Globos", icon_url: "eyeball.png" },
    { id: 8135, key: "TreasureHunter", name: "Tesouro", icon_url: "treasure.png" },
    { id: 8226, key: "ManaflowBand", name: "Faixa", icon_url: "mana.png" },
    { id: 8210, key: "Transcendence", name: "Transcendência", icon_url: "trans.png" },
    { id: 5008, key: "AdaptiveForce", name: "Força Adaptativa", icon_url: "af.png" },
    { id: 5008, key: "AdaptiveForce", name: "Força Adaptativa", icon_url: "af.png" },
    { id: 5001, key: "HealthScaling", name: "Vida Escalonável", icon_url: "hp.png" },
  ],
  rune_pages: [],
  core_build_options: [],
  build_options: [],
  final_build_options: [],
  spells: [
    { id: 4, name: "Flash", icon_url: "flash.png" },
    { id: 14, name: "Incendiar", icon_url: "ignite.png" },
  ],
  spell_choices: [{ id: 14, name: "Incendiar", icon_url: "ignite.png" }],
  item_blocks: [
    { title: "Itens Iniciais", item_ids: [1056, 2003], item_icon_urls: ["1056.png", "2003.png"] },
  ],
  active_position: "mid",
  active_tier: "emerald_plus",
  active_region: "global",
  active_patch: "16.22",
  available_patches: ["16.22", "16.21"],
  patch_fallback_applied: false,
  patch_protection_remaining_hours: null,
  supports_runes: true,
};

describe("NavigationDock Ranqueada position", () => {
  it("places Ranqueada button strictly between Runas and Aram", () => {
    render(<NavigationDock activeTab="ranked" onTabChange={vi.fn()} />);
    const buttons = screen.getAllByRole("button");
    expect(buttons.map((b) => b.textContent?.trim())).toEqual([
      "Perfil",
      "Runas",
      "Ranqueada",
      "Aram",
      "Configurações",
    ]);
    expect(screen.getByRole("button", { name: "Ranqueada" })).toHaveAttribute(
      "aria-current",
      "page"
    );
  });
});

describe("RankedTab UI", () => {
  it("combines all lanes in 'Todas as Rotas', differentiates global rank and lane per champ, and removes duplicate Rota dropdown", () => {
    const onTierListFilterChange = vi.fn();
    render(
      <RankedTab
        isActive={true}
        filters={{ region: "global", tier: "emerald_plus", lane: "all", patch: null }}
        tierData={mockTierResponse}
        tierLoading={false}
        tierError={null}
        selectedChampion={null}
        championBuild={null}
        buildLoading={false}
        buildError={null}
        orderedSpells={[]}
        selectedSecondarySpellId={null}
        selectedRunePageIndex={0}
        selectedBuildIndex={0}
        onTierListFilterChange={onTierListFilterChange}
        onDetailFilterChange={vi.fn()}
        onSelectChampion={vi.fn()}
        onBackToTierList={vi.fn()}
        onRetryTierList={vi.fn()}
        onRetryBuild={vi.fn()}
        onSelectSecondarySpell={vi.fn()}
        onInvertSpells={vi.fn()}
        onSelectRunePage={vi.fn()}
        onSelectBuild={vi.fn()}
      />
    );

    // Duplicate Rota dropdown between Elo and Servidor must NOT exist
    expect(screen.queryByTestId("ranked-filter-position")).not.toBeInTheDocument();
    expect(screen.getByTestId("ranked-filter-tier")).toBeInTheDocument();
    expect(screen.getByTestId("ranked-filter-region")).toBeInTheDocument();
    expect(screen.getByTestId("ranked-filter-patch")).toBeInTheDocument();

    // Bottom lane pills remain the single lane selector
    expect(screen.getByTestId("ranked-lane-tabs")).toBeInTheDocument();

    // Unified all-lanes view (NOT split into separate lane sections)
    expect(screen.queryByTestId("ranked-lane-section-top")).not.toBeInTheDocument();
    expect(screen.getByTestId("ranked-all-lanes-list")).toBeInTheDocument();

    // OP group contains Ekko (Mid #1), Malphite (Top #2), Ahri (Mid #3) all together
    const opGroup = screen.getByTestId("ranked-tier-group-all-0");
    expect(within(opGroup).getAllByText("OP")[0]).toHaveClass("tier-op");
    const opButtons = within(opGroup).getAllByRole("button");
    expect(opButtons.map((btn) => btn.getAttribute("data-testid"))).toEqual([
      "ranked-champ-mid-ekko",
      "ranked-champ-top-malphite",
      "ranked-champ-mid-ahri",
    ]);

    // Differentiates overall rank (#1, #2, #3) and lane badge (Meio vs Topo vs Selva)
    expect(within(screen.getByTestId("ranked-champ-mid-ekko")).getByText("#1")).toBeInTheDocument();
    expect(within(screen.getByTestId("ranked-champ-top-malphite")).getByText("#2")).toBeInTheDocument();
    expect(within(screen.getByTestId("ranked-champ-mid-ahri")).getByText("#3")).toBeInTheDocument();
    expect(screen.getByTestId("ranked-champ-lane-mid-ekko")).toHaveTextContent("Meio");
    expect(screen.getByTestId("ranked-champ-lane-top-malphite")).toHaveTextContent("Topo");
    expect(screen.getByTestId("ranked-champ-lane-jungle-ekko")).toHaveTextContent("Selva");

    // Tier 1 and Tier 2 groups are also combined across lanes
    const tier1Group = screen.getByTestId("ranked-tier-group-all-1");
    expect(within(tier1Group).getByText("Garen")).toBeInTheDocument();
    const tier2Group = screen.getByTestId("ranked-tier-group-all-2");
    expect(within(tier2Group).getByText("Ekko")).toBeInTheDocument();

    // Clicking a lane separation pill updates position filter
    fireEvent.click(screen.getByTestId("ranked-lane-tab-mid"));
    expect(onTierListFilterChange).toHaveBeenCalledWith("position", "mid");
  });

  it("filters to a single lane section sorted by lane rank when a specific lane pill is selected", () => {
    render(
      <RankedTab
        isActive={true}
        filters={{ region: "global", tier: "emerald_plus", lane: "mid", patch: null }}
        tierData={mockTierResponse}
        tierLoading={false}
        tierError={null}
        selectedChampion={null}
        championBuild={null}
        buildLoading={false}
        buildError={null}
        orderedSpells={[]}
        selectedSecondarySpellId={null}
        selectedRunePageIndex={0}
        selectedBuildIndex={0}
        onTierListFilterChange={vi.fn()}
        onDetailFilterChange={vi.fn()}
        onSelectChampion={vi.fn()}
        onBackToTierList={vi.fn()}
        onRetryTierList={vi.fn()}
        onRetryBuild={vi.fn()}
        onSelectSecondarySpell={vi.fn()}
        onInvertSpells={vi.fn()}
        onSelectRunePage={vi.fn()}
        onSelectBuild={vi.fn()}
      />
    );

    const midSection = screen.getByTestId("ranked-lane-section-mid");
    expect(within(midSection).getByText("Meio")).toBeInTheDocument();
    expect(within(midSection).queryByText("Malphite")).not.toBeInTheDocument();

    const opMidGroup = screen.getByTestId("ranked-tier-group-mid-0");
    // In single-lane Mid view, lane rank #1 Ahri comes before lane rank #2 Ekko
    const midOpButtons = within(opMidGroup).getAllByRole("button");
    expect(midOpButtons.map((btn) => btn.getAttribute("data-testid"))).toEqual([
      "ranked-champ-mid-ahri",
      "ranked-champ-mid-ekko",
    ]);
  });

  it("displays Patch dropdown with 48h protection indicator, tooltip, and manual override", () => {
    const onTierListFilterChange = vi.fn();
    render(
      <RankedTab
        isActive={true}
        filters={{ region: "global", tier: "emerald_plus", lane: "all", patch: null }}
        tierData={mockProtectedTierResponse}
        tierLoading={false}
        tierError={null}
        selectedChampion={null}
        championBuild={null}
        buildLoading={false}
        buildError={null}
        orderedSpells={[]}
        selectedSecondarySpellId={null}
        selectedRunePageIndex={0}
        selectedBuildIndex={0}
        onTierListFilterChange={onTierListFilterChange}
        onDetailFilterChange={vi.fn()}
        onSelectChampion={vi.fn()}
        onBackToTierList={vi.fn()}
        onRetryTierList={vi.fn()}
        onRetryBuild={vi.fn()}
        onSelectSecondarySpell={vi.fn()}
        onInvertSpells={vi.fn()}
        onSelectRunePage={vi.fn()}
        onSelectBuild={vi.fn()}
      />
    );

    expect(screen.getByTestId("ranked-patch-fallback-badge")).toHaveTextContent(
      /Volume Protegido \(16\.21\).*Faltam 38h/i
    );
    const indicator = screen.getByTestId("ranked-patch-protection-indicator");
    fireEvent.mouseEnter(indicator);
    const tooltip = screen.getByTestId("ranked-patch-protection-tooltip");
    expect(tooltip).toHaveTextContent(/48h/i);
    expect(tooltip).toHaveTextContent(/Faltam 38 horas/i);

    const patchTrigger = screen.getByTestId("ranked-filter-patch");
    expect(patchTrigger).toHaveAttribute("data-value", "16.21");
    fireEvent.click(patchTrigger);
    const listbox = screen.getByRole("listbox", { name: "Patch:" });
    fireEvent.click(within(listbox).getByRole("option", { name: "Patch 16.22 (Mais atual)" }));
    expect(onTierListFilterChange).toHaveBeenCalledWith("patch", "16.22");
  });

  it("renders Runes-identical build screen inside Ranked tab when a champion is selected", () => {
    const onBackToTierList = vi.fn();
    const onDetailFilterChange = vi.fn();

    render(
      <RankedTab
        isActive={true}
        filters={{ region: "global", tier: "emerald_plus", lane: "mid", patch: null }}
        tierData={mockTierResponse}
        tierLoading={false}
        tierError={null}
        selectedChampion={mockRankedChampions[2]}
        championBuild={mockAhriBuild}
        buildLoading={false}
        buildError={null}
        orderedSpells={mockAhriBuild.spells}
        selectedSecondarySpellId={null}
        selectedRunePageIndex={0}
        selectedBuildIndex={0}
        onTierListFilterChange={vi.fn()}
        onDetailFilterChange={onDetailFilterChange}
        onSelectChampion={vi.fn()}
        onBackToTierList={onBackToTierList}
        onRetryTierList={vi.fn()}
        onRetryBuild={vi.fn()}
        onSelectSecondarySpell={vi.fn()}
        onInvertSpells={vi.fn()}
        onSelectRunePage={vi.fn()}
        onSelectBuild={vi.fn()}
      />
    );

    const view = screen.getByTestId("view-ranked");
    expect(within(view).getByTestId("ranked-champion-detail")).toBeInTheDocument();
    expect(within(view).getByTestId("champ-hero")).toBeInTheDocument();
    expect(within(view).getByTestId("runes-tree")).toBeInTheDocument();
    expect(within(view).getByTestId("spells-card")).toBeInTheDocument();
    expect(within(view).getByTestId("item-blocks")).toBeInTheDocument();

    fireEvent.click(within(view).getByTestId("ranked-back-btn"));
    expect(onBackToTierList).toHaveBeenCalledTimes(1);
  });
});

describe("useRanked hook state and filter preservation", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.restoreAllMocks();
  });

  it("loads tier list, fetches read-only build on champion click, and preserves user filters across detail/back", async () => {
    const tierSpy = vi.spyOn(commands, "getRankedTierList").mockResolvedValue(mockTierResponse);
    const buildSpy = vi.spyOn(commands, "getRankedChampionBuild").mockResolvedValue(mockAhriBuild);
    const updateFiltersSpy = vi.spyOn(commands, "updateBuildFilters");

    const { result } = renderHook(() => useRanked(true, "D", true));

    await waitFor(() => {
      expect(tierSpy).toHaveBeenCalledWith("global", "emerald_plus", null);
    });

    // User selects server BR and patch 16.22 on the Tier List
    act(() => {
      result.current.handleTierListFilterChange("region", "br");
      result.current.handleTierListFilterChange("patch", "16.22");
    });

    await waitFor(() => {
      expect(tierSpy).toHaveBeenCalledWith("br", "emerald_plus", "16.22");
    });

    // Click Ahri (Mid) -> fetches read-only build with preserved filters (br, emerald_plus, 16.22)
    act(() => {
      result.current.selectChampion(mockRankedChampions[2]);
    });

    await waitFor(() => {
      expect(buildSpy).toHaveBeenCalledWith(103, "mid", "emerald_plus", "br", "16.22");
    });
    expect(updateFiltersSpy).not.toHaveBeenCalled();

    // Change Elo inside detail screen -> updates both build AND preserved Ranked filters
    act(() => {
      result.current.handleDetailFilterChange("tier", "diamond_plus");
    });

    await waitFor(() => {
      expect(buildSpy).toHaveBeenCalledWith(103, "mid", "diamond_plus", "br", "16.22");
    });

    // Go back to Tier List -> filters remain (br, diamond_plus, 16.22)
    act(() => {
      result.current.clearSelectedChampion();
    });

    expect(result.current.selectedChampion).toBeNull();
    expect(result.current.filters).toEqual({
      region: "br",
      tier: "diamond_plus",
      lane: "all",
      patch: "16.22",
    });
    expect(JSON.parse(localStorage.getItem(RANKED_FILTERS_STORAGE_KEY) || "{}")).toEqual({
      region: "br",
      tier: "diamond_plus",
      lane: "all",
      patch: "16.22",
    });
  });
});

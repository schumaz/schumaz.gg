import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { renderHook, act } from "@testing-library/react";
import { RankedTab } from "../RankedTab";
import { useRanked } from "../useRanked";
import type { ChampionBuild_Serialize } from "../../../types";
import {
  buildIpc,
  injectorIpc,
  lcuIpc,
  rankedIpc,
  settingsIpc,
} from "../../../services/ipc";
import {
  mockAhriBuild,
  mockRankedChampions,
  mockTierResponse,
} from "./rankedFixtures";

describe("useRanked hook state and filter preservation", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.restoreAllMocks();
  });

  it("loads tier list, fetches read-only build on champion click, and preserves user filters across detail/back", async () => {
    const tierSpy = vi.spyOn(rankedIpc, "getRankedTierList").mockResolvedValue(mockTierResponse);
    const buildSpy = vi.spyOn(rankedIpc, "getRankedChampionBuild").mockResolvedValue(mockAhriBuild);
    const updateFiltersSpy = vi.spyOn(buildIpc, "updateBuildFilters");
    const onNavigationPatch = vi.fn();

    const { result } = renderHook(() =>
      useRanked(true, "D", true, "pt_BR", { onNavigationPatch })
    );

    await waitFor(() => {
      expect(tierSpy).toHaveBeenCalledWith("global", "emerald_plus", null);
    });

    act(() => {
      result.current.handleTierListFilterChange("region", "br");
      result.current.handleTierListFilterChange("patch", "16.22");
    });

    await waitFor(() => {
      expect(tierSpy).toHaveBeenCalledWith("br", "emerald_plus", "16.22");
    });

    act(() => {
      result.current.selectChampion(mockRankedChampions[2]);
    });

    await waitFor(() => {
      expect(buildSpy).toHaveBeenCalledWith(103, "mid", "emerald_plus", "br", "16.22");
    });
    expect(updateFiltersSpy).not.toHaveBeenCalled();

    act(() => {
      result.current.handleDetailFilterChange("tier", "diamond_plus");
    });

    await waitFor(() => {
      expect(buildSpy).toHaveBeenCalledWith(103, "mid", "diamond_plus", "br", "16.22");
    });

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
    expect(onNavigationPatch).toHaveBeenCalledWith(
      expect.objectContaining({
        rankedFilterRegion: "br",
        rankedFilterTier: "diamond_plus",
        rankedFilterLane: "all",
        rankedFilterPatch: "16.22",
      })
    );
  });

  it("switches rune pages and build options locally in Ranked without injecting anything into the game", async () => {
    const mockMultiBuild: ChampionBuild_Serialize = {
      ...mockAhriBuild,
      skill_order: null,
      rune_pages: [
        {
          primary_style_id: 8100,
          sub_style_id: 8200,
          primary_style_name: "Dominação",
          primary_style_icon_url: "dom.png",
          sub_style_name: "Feitiçaria",
          sub_style_icon_url: "sorc.png",
          selected_perk_ids: [8112, 8126, 8138, 8135, 8226, 8210, 5008, 5008, 5001],
          perks: mockAhriBuild.perks,
          play: 5000,
          win: 2600,
          pick_rate: 70.0,
        },
        {
          primary_style_id: 8200,
          sub_style_id: 8300,
          primary_style_name: "Feitiçaria",
          primary_style_icon_url: "sorc.png",
          sub_style_name: "Inspiração",
          sub_style_icon_url: "insp.png",
          selected_perk_ids: [8229, 8226, 8210, 8237, 8304, 8345, 5008, 5008, 5001],
          perks: [
            { id: 8229, key: "ArcaneComet", name: "Cometa Arcano", icon_url: "comet.png", short_desc: "" },
            { id: 8226, key: "ManaflowBand", name: "Faixa", icon_url: "mana.png", short_desc: "" },
            { id: 8210, key: "Transcendence", name: "Transcendência", icon_url: "trans.png", short_desc: "" },
            { id: 8237, key: "Scorch", name: "Chamuscar", icon_url: "scorch.png", short_desc: "" },
            { id: 8304, key: "MagicalFootwear", name: "Calçados", icon_url: "boots.png", short_desc: "" },
            { id: 8345, key: "BiscuitDelivery", name: "Biscoito", icon_url: "biscuit.png", short_desc: "" },
            { id: 5008, key: "AdaptiveForce", name: "Força Adaptativa", icon_url: "af.png", short_desc: "" },
            { id: 5008, key: "AdaptiveForce", name: "Força Adaptativa", icon_url: "af.png", short_desc: "" },
            { id: 5001, key: "HealthScaling", name: "Vida Escalonável", icon_url: "hp.png", short_desc: "" },
          ],
          play: 2000,
          win: 1050,
          pick_rate: 30.0,
        },
      ],
      build_options: [
        {
          name: "Build 1 (Mais Feita)",
          core_item_ids: [3118, 4645, 3089],
          core_item_icon_urls: ["b1-1.png", "b1-2.png", "b1-3.png"],
          final_item_ids: [3118, 4645, 3089, 3020, 3157, 3135],
          final_item_icon_urls: ["b1-1.png", "b1-2.png", "b1-3.png", "b1-4.png", "b1-5.png", "b1-6.png"],
          situational_item_ids: [3157],
          situational_item_icon_urls: ["zhonya.png"],
          play: 8000,
          win: 4200,
          pick_rate: 70.0,
        },
        {
          name: "Build 2",
          core_item_ids: [6655, 3157, 3089],
          core_item_icon_urls: ["b2-1.png", "b2-2.png", "b2-3.png"],
          final_item_ids: [6655, 3157, 3089, 3020, 3165, 3102],
          final_item_icon_urls: ["b2-1.png", "b2-2.png", "b2-3.png", "b2-4.png", "b2-5.png", "b2-6.png"],
          situational_item_ids: [3102],
          situational_item_icon_urls: ["banshee.png"],
          play: 3000,
          win: 1600,
          pick_rate: 30.0,
        },
      ],
    };

    vi.spyOn(rankedIpc, "getRankedTierList").mockResolvedValue(mockTierResponse);
    vi.spyOn(rankedIpc, "getRankedChampionBuild").mockResolvedValue(mockMultiBuild);
    const injectRunesSpy = vi.spyOn(injectorIpc, "injectRunePage");
    const injectBuildSpy = vi.spyOn(injectorIpc, "injectItemBuild");
    const injectSpellsSpy = vi.spyOn(injectorIpc, "injectSummonerSpells");
    const updateFiltersSpy = vi.spyOn(buildIpc, "updateBuildFilters");
    const syncChampSpy = vi.spyOn(lcuIpc, "syncDisplayedChampion");
    const syncPrefsSpy = vi.spyOn(settingsIpc, "syncUserPreferences");

    function Harness() {
      const ranked = useRanked(true, "D", true);
      return (
        <RankedTab
          isActive={true}
          filters={ranked.filters}
          tierData={ranked.tierData}
          tierLoading={ranked.tierLoading}
          tierError={ranked.tierError}
          selectedChampion={ranked.selectedChampion}
          championBuild={ranked.championBuild}
          buildLoading={ranked.buildLoading}
          buildError={ranked.buildError}
          orderedSpells={ranked.orderedSpells}
          selectedRunePageIndex={ranked.selectedRunePageIndex}
          selectedBuildIndex={ranked.selectedBuildIndex}
          onTierListFilterChange={ranked.handleTierListFilterChange}
          onDetailFilterChange={ranked.handleDetailFilterChange}
          onSelectChampion={ranked.selectChampion}
          onBackToTierList={ranked.clearSelectedChampion}
          onRetryTierList={ranked.retryTierList}
          onRetryBuild={ranked.retryChampionBuild}
          onSelectRunePage={ranked.setSelectedRunePageIndex}
          onSelectBuild={ranked.setSelectedBuildIndex}
        />
      );
    }

    render(<Harness />);

    await waitFor(() => {
      expect(screen.getByTestId("ranked-champ-mid-ahri")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId("ranked-champ-mid-ahri"));

    await waitFor(() => {
      expect(screen.getByTestId("ranked-champion-detail")).toBeInTheDocument();
      expect(screen.getByText("Eletrocutar")).toBeInTheDocument();
      expect(screen.getByTestId("active-build-meta")).toHaveTextContent(/70.0% uso/i);
    });

    fireEvent.click(screen.getByTestId("rune-page-btn-1"));
    expect(screen.getByText("Cometa Arcano")).toBeInTheDocument();
    expect(screen.queryByText("Eletrocutar")).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId("build-opt-btn-1"));
    expect(screen.getByTestId("active-build-meta")).toHaveTextContent(/30.0% uso/i);

    expect(screen.getByTestId("spell-pair-info-0")).toHaveClass("active");
    fireEvent.click(screen.getByTestId("spell-pair-info-1"));
    expect(screen.getByTestId("spell-pair-info-0")).toHaveClass("active");
    expect(screen.getByTestId("spell-pair-info-1")).not.toHaveClass("active");
    expect(screen.getByTestId("invert-spells-readonly")).toBeInTheDocument();

    expect(injectRunesSpy).not.toHaveBeenCalled();
    expect(injectBuildSpy).not.toHaveBeenCalled();
    expect(injectSpellsSpy).not.toHaveBeenCalled();
    expect(updateFiltersSpy).not.toHaveBeenCalled();
    expect(syncChampSpy).not.toHaveBeenCalled();
    expect(syncPrefsSpy).not.toHaveBeenCalled();
  });
});

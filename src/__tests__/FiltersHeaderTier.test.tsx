import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { FiltersHeader } from "../components/ui/FiltersHeader";
import { ChampionBuild, ChampSelectedPayload } from "../types";

const baseBuild: ChampionBuild = {
  champion_id: 103,
  champion_name: "Ahri",
  win_rate: 52.4,
  pick_rate: 8.5,
  primary_style_id: 8000,
  sub_style_id: 8100,
  selected_perk_ids: [8005, 9111, 9104, 8014, 8139, 8135, 5008, 5008, 5001],
  spell1_id: 4,
  spell2_id: 14,
  item_blocks: [],
};

const summonerRiftChampSelect: ChampSelectedPayload = {
  champion_id: 103,
  queue_id: 420,
  queue_mode: "SummonersRift",
  locked_in: true,
};

describe("FiltersHeader — Champion Meta Tier Badge", () => {
  it.each([
    [0, "OP", "tier-op"],
    [1, "Tier 1", "tier-1"],
    [2, "Tier 2", "tier-2"],
    [3, "Tier 3", "tier-3"],
    [4, "Tier 4", "tier-4"],
  ])(
    "renders tier %i badge next to champion name with label %s and class %s",
    (tier, expectedLabel, expectedClass) => {
      render(
        <FiltersHeader
          build={{ ...baseBuild, tier }}
          champSelect={summonerRiftChampSelect}
          loading={false}
          onFilterChange={vi.fn()}
        />
      );

      const badge = screen.getByTestId("champ-tier-badge");
      expect(badge).toHaveTextContent(expectedLabel);
      expect(badge).toHaveClass("tier-pill", "champ-hero-tier-pill", expectedClass);

      const champName = screen.getByRole("heading", { name: "Ahri" });
      expect(champName.nextElementSibling).toBe(badge);
    }
  );

  it("omits the tier badge gracefully when tier is null or undefined", () => {
    const { rerender } = render(
      <FiltersHeader
        build={{ ...baseBuild, tier: null }}
        champSelect={summonerRiftChampSelect}
        loading={false}
        onFilterChange={vi.fn()}
      />
    );
    expect(screen.queryByTestId("champ-tier-badge")).not.toBeInTheDocument();

    rerender(
      <FiltersHeader
        build={{ ...baseBuild, tier: undefined }}
        champSelect={summonerRiftChampSelect}
        loading={false}
        onFilterChange={vi.fn()}
      />
    );
    expect(screen.queryByTestId("champ-tier-badge")).not.toBeInTheDocument();
  });
});

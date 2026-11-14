import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { FiltersHeader } from "../components/FiltersHeader";
import { ChampionBuild, ChampSelectedPayload } from "../types";

const mockBuild: ChampionBuild = {
  champion_id: 103,
  champion_name: "Ahri",
  win_rate: 52.4,
  pick_rate: 8.5,
  primary_style_id: 8000,
  sub_style_id: 8100,
  selected_perk_ids: [8005, 9111, 9104, 8014, 8139, 8135, 5008, 5008, 5001],
  spell1_id: 4,
  spell2_id: 32,
  item_blocks: [],
};

describe("FiltersHeader", () => {
  it("renders ARAM Mayhem badge and hides Rota selector when queue_mode is AramMayhem", () => {
    const mockChampSelect: ChampSelectedPayload = {
      champion_id: 103,
      queue_id: 2400,
      queue_mode: "AramMayhem",
      locked_in: true,
    };

    render(
      <FiltersHeader
        build={mockBuild}
        champSelect={mockChampSelect}
        loading={false}
        onFilterChange={vi.fn()}
      />
    );

    expect(screen.getByText("ARAM Mayhem (Howling Abyss)")).toBeInTheDocument();
    expect(screen.queryByTestId("filter-position")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Rota:")).not.toBeInTheDocument();
    expect(screen.getByTestId("filter-tier")).toBeInTheDocument();
    expect(screen.getByTestId("filter-region")).toBeInTheDocument();
    expect(screen.getByTestId("filter-patch")).toBeInTheDocument();
  });

  it("renders traditional ARAM badge and hides Rota selector when queue_mode is Aram", () => {
    const mockChampSelect: ChampSelectedPayload = {
      champion_id: 103,
      queue_id: 450,
      queue_mode: "Aram",
      locked_in: true,
    };

    render(
      <FiltersHeader
        build={mockBuild}
        champSelect={mockChampSelect}
        loading={false}
        onFilterChange={vi.fn()}
      />
    );

    expect(screen.getByText("ARAM (Howling Abyss)")).toBeInTheDocument();
    expect(screen.queryByTestId("filter-position")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Rota:")).not.toBeInTheDocument();
  });

  it("renders Rota selector when queue_mode is SummonersRift", () => {
    const mockChampSelect: ChampSelectedPayload = {
      champion_id: 103,
      queue_id: 420,
      queue_mode: "SummonersRift",
      locked_in: true,
    };

    render(
      <FiltersHeader
        build={{ ...mockBuild, active_position: "mid" }}
        champSelect={mockChampSelect}
        loading={false}
        onFilterChange={vi.fn()}
      />
    );

    expect(screen.getByTestId("filter-position")).toBeInTheDocument();
    expect(screen.getByLabelText("Rota:")).toBeInTheDocument();
  });
});

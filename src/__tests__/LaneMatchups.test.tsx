import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import { FiltersHeader } from "../components/ui/FiltersHeader";
import { LaneMatchupsCard } from "../components/ui/LaneMatchupsCard";
import { LaneMatchupsPopover } from "../components/ui/LaneMatchupsPopover";
import { mockAhriBuild } from "./mockAhriBuildFixture";
import type { ChampSelectedPayload, LaneMatchupsSummary } from "../types";

const sampleMatchups: LaneMatchupsSummary = {
  hard: [
    { champion_id: 1, key: "annie", name: "Annie", icon_url: "Annie.png", play: 2761, win: 1310, win_rate: 47.4 },
    { champion_id: 800, key: "mel", name: "Mel", icon_url: "Mel.png", play: 2067, win: 993, win_rate: 48.0 },
    { champion_id: 142, key: "zoe", name: "Zoe", icon_url: "", play: 2179, win: 1060, win_rate: 48.6 },
  ],
  favorable: [
    { champion_id: 268, key: "azir", name: "Azir", icon_url: "Azir.png", play: 1835, win: 1014, win_rate: 55.3 },
    { champion_id: 3, key: "galio", name: "Galio", icon_url: "Galio.png", play: 2786, win: 1532, win_rate: 55.0 },
  ],
};

describe("LaneMatchups UI & FiltersHeader visibility", () => {
  it("renders Hard vs Favorable columns, formatted rates/games, and gates interactivity on onSelectChampion", () => {
    const onSelectChampion = vi.fn();
    const { rerender } = render(
      <LaneMatchupsCard
        matchups={sampleMatchups}
        championName="Ahri"
        laneLabel="Meio"
        onSelectChampion={onSelectChampion}
      />
    );

    const hardCol = screen.getByTestId("lane-matchups-hard");
    const favCol = screen.getByTestId("lane-matchups-favorable");
    expect(within(hardCol).getByText("Annie")).toBeInTheDocument();
    expect(within(hardCol).getByText("47.4%")).toHaveClass("is-hard");
    expect(within(hardCol).getByText(/2\.761 partidas|2,761 partidas/i)).toBeInTheDocument();
    expect(within(favCol).getByText("Azir")).toBeInTheDocument();
    expect(within(favCol).getByText("55.3%")).toHaveClass("is-favorable");

    const annieBtn = screen.getByTestId("lane-matchup-row-1");
    expect(annieBtn.tagName).toBe("BUTTON");
    fireEvent.click(annieBtn);
    expect(onSelectChampion).toHaveBeenCalledWith(1);

    rerender(
      <LaneMatchupsCard
        matchups={{ hard: sampleMatchups.hard, favorable: [] }}
        championName="Ahri"
        laneLabel="Meio"
      />
    );
    const readOnlyRow = screen.getByTestId("lane-matchup-row-1");
    expect(readOnlyRow.tagName).toBe("DIV");
    expect(screen.getByText("Sem dados suficientes nesta rota.")).toBeInTheDocument();
  });

  it("opens and closes LaneMatchupsPopover via click, close button, Escape, outside click, selection, and resetKey", () => {
    const onSelectChampion = vi.fn();
    const { rerender } = render(
      <div>
        <button type="button" data-testid="outside-el">
          Fora
        </button>
        <LaneMatchupsPopover
          matchups={sampleMatchups}
          championName="Ahri"
          laneLabel="Meio"
          onSelectChampion={onSelectChampion}
          resetKey="103-mid"
        />
      </div>
    );

    const trigger = screen.getByTestId("lane-matchups-trigger");
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    expect(within(trigger).getAllByRole("img")).toHaveLength(2);
    expect(screen.queryByTestId("lane-matchups-popover")).not.toBeInTheDocument();

    fireEvent.click(trigger);
    expect(trigger).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByTestId("lane-matchups-popover")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("lane-matchups-close-btn"));
    expect(screen.queryByTestId("lane-matchups-popover")).not.toBeInTheDocument();

    fireEvent.click(trigger);
    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByTestId("lane-matchups-popover")).not.toBeInTheDocument();

    fireEvent.click(trigger);
    fireEvent.mouseDown(screen.getByTestId("outside-el"));
    expect(screen.queryByTestId("lane-matchups-popover")).not.toBeInTheDocument();

    fireEvent.click(trigger);
    fireEvent.click(screen.getByTestId("lane-matchup-row-1"));
    expect(onSelectChampion).toHaveBeenCalledWith(1);
    expect(screen.queryByTestId("lane-matchups-popover")).not.toBeInTheDocument();

    fireEvent.click(trigger);
    expect(screen.getByTestId("lane-matchups-popover")).toBeInTheDocument();
    rerender(
      <div>
        <button type="button" data-testid="outside-el">
          Fora
        </button>
        <LaneMatchupsPopover
          matchups={sampleMatchups}
          championName="Ahri"
          laneLabel="Topo"
          onSelectChampion={onSelectChampion}
          resetKey="103-top"
        />
      </div>
    );
    expect(screen.queryByTestId("lane-matchups-popover")).not.toBeInTheDocument();
  });

  it("shows matchups trigger in FiltersHeader for Summoner's Rift and hides it in ARAM, ARAM Mayhem, and Arena", () => {
    const srChampSelect: ChampSelectedPayload = {
      champion_id: 103,
      queue_id: 420,
      queue_mode: "SummonersRift",
      locked_in: true,
    };

    const { rerender } = render(
      <FiltersHeader
        build={mockAhriBuild}
        champSelect={srChampSelect}
        loading={false}
        onFilterChange={vi.fn()}
      />
    );
    expect(screen.getByTestId("lane-matchups-trigger")).toBeInTheDocument();
    expect(screen.getByTestId("skill-order-trigger")).toBeInTheDocument();

    rerender(
      <FiltersHeader
        build={{ ...mockAhriBuild, active_position: "" }}
        champSelect={{ champion_id: 103, queue_id: 450, queue_mode: "Aram", locked_in: true }}
        loading={false}
        onFilterChange={vi.fn()}
      />
    );
    expect(screen.queryByTestId("lane-matchups-trigger")).not.toBeInTheDocument();

    rerender(
      <FiltersHeader
        build={{ ...mockAhriBuild, active_position: "", supports_runes: false }}
        champSelect={{ champion_id: 103, queue_id: 2400, queue_mode: "AramMayhem", locked_in: true }}
        loading={false}
        onFilterChange={vi.fn()}
      />
    );
    expect(screen.queryByTestId("lane-matchups-trigger")).not.toBeInTheDocument();

    rerender(
      <FiltersHeader
        build={{ ...mockAhriBuild, active_tier: "arena", supports_runes: false, supports_spells: false }}
        champSelect={{ champion_id: 103, queue_id: 1700, queue_mode: "Arena", locked_in: true }}
        loading={false}
        onFilterChange={vi.fn()}
      />
    );
    expect(screen.queryByTestId("lane-matchups-trigger")).not.toBeInTheDocument();
  });
});

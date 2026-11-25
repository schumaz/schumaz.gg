import { describe, it, expect, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { FiltersHeader } from "../components/ui/FiltersHeader";
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

describe("FiltersHeader — Clash & Swiftplay lobby preselect", () => {
  it("renders Clash badge and Swiftplay Primary/Secondary slot switcher", () => {
    const clashChampSelect: ChampSelectedPayload = {
      champion_id: 103,
      queue_id: 700,
      queue_mode: "SummonersRift",
      locked_in: true,
    };

    const { rerender } = render(
      <FiltersHeader
        build={{ ...mockBuild, active_position: "mid" }}
        champSelect={clashChampSelect}
        loading={false}
        onFilterChange={vi.fn()}
      />
    );
    expect(screen.getByText("Clash (Build Ranqueada)")).toBeInTheDocument();
    expect(screen.queryByTestId("lobby-slots-bar")).not.toBeInTheDocument();

    const onSelectLobbySlot = vi.fn();
    const swiftplayChampSelect: ChampSelectedPayload = {
      champion_id: 103,
      queue_id: 480,
      queue_mode: "SummonersRift",
      locked_in: true,
      is_lobby_preselect: true,
      active_slot_index: 0,
      lobby_slots: [
        {
          slot_index: 0,
          champion_id: 103,
          champion_name: "Ahri",
          champion_icon_url: "",
          assigned_position: "mid",
        },
        {
          slot_index: 1,
          champion_id: 222,
          champion_name: "Jinx",
          champion_icon_url: "",
          assigned_position: "adc",
        },
      ],
    };

    rerender(
      <FiltersHeader
        build={{ ...mockBuild, active_position: "mid" }}
        champSelect={swiftplayChampSelect}
        loading={false}
        onFilterChange={vi.fn()}
        onSelectLobbySlot={onSelectLobbySlot}
      />
    );

    expect(screen.getByText("Jogo Rápido (Swiftplay)")).toBeInTheDocument();
    expect(screen.getByText("Pré-seleção no Lobby")).toBeInTheDocument();
    const slot0 = screen.getByTestId("lobby-slot-btn-0");
    const slot1 = screen.getByTestId("lobby-slot-btn-1");
    expect(slot0).toHaveAttribute("aria-pressed", "true");
    expect(slot1).toHaveAttribute("aria-pressed", "false");
    expect(slot0).toHaveTextContent("Primário");
    expect(slot1).toHaveTextContent("Secundário");
    fireEvent.click(slot1);
    expect(onSelectLobbySlot).toHaveBeenCalledWith(1);
  });
});

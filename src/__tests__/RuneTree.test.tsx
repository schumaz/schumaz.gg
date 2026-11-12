import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { RuneTree } from "../components/RuneTree";
import { ChampionBuild } from "../types";

const mockNormalBuild: ChampionBuild = {
  champion_id: 103,
  champion_name: "Ahri",
  win_rate: 52.4,
  pick_rate: 8.5,
  primary_style_id: 8000,
  sub_style_id: 8100,
  primary_style_name: "Precisão",
  sub_style_name: "Dominação",
  selected_perk_ids: [8005, 9111, 9104, 8014, 8139, 8135, 5008, 5008, 5001],
  spell1_id: 4,
  spell2_id: 14,
  item_blocks: [],
  supports_runes: true,
  perks: [
    { id: 8005, key: "PressTheAttack", name: "Pressione o Ataque", icon_url: "pta.png" },
    { id: 9111, key: "Triumph", name: "Triunfo", icon_url: "triumph.png" },
    { id: 9104, key: "LegendAlacrity", name: "Lenda: Espontaneidade", icon_url: "alacrity.png" },
    { id: 8014, key: "CoupDeGrace", name: "Golpe de Misericórdia", icon_url: "coup.png" },
    { id: 8139, key: "TasteOfBlood", name: "Gosto de Sangue", icon_url: "taste.png" },
    { id: 8135, key: "TreasureHunter", name: "Caçador de Tesouros", icon_url: "hunter.png" },
    { id: 5008, key: "Adaptive", name: "Força Adaptativa", icon_url: "stat.png" },
    { id: 5008, key: "Adaptive", name: "Força Adaptativa", icon_url: "stat.png" },
    { id: 5001, key: "Health", name: "Vida", icon_url: "stat.png" },
  ],
};

const mockMayhemBuild: ChampionBuild = {
  ...mockNormalBuild,
  supports_runes: false,
};

describe("RuneTree", () => {
  it("renders notice in left column when mode does not support runes (ARAM Mayhem)", () => {
    render(
      <RuneTree
        build={mockMayhemBuild}
        selectedRunePageIndex={0}
        onSelectRunePage={vi.fn()}
      />
    );

    expect(screen.getByTestId("runes-tree")).toBeInTheDocument();
    expect(screen.getByText("⚡ ARAM Mayhem")).toBeInTheDocument();
    expect(screen.getByText("Runas Desativadas")).toBeInTheDocument();
    expect(screen.getByText(/não suporta ajuste de runas/i)).toBeInTheDocument();
    expect(screen.getByText(/Feitiços dinâmicos recomendados/i)).toBeInTheDocument();
  });

  it("renders standard rune tree layout when runes are supported (normal ARAM / Ranked)", () => {
    render(
      <RuneTree
        build={mockNormalBuild}
        selectedRunePageIndex={0}
        onSelectRunePage={vi.fn()}
      />
    );

    expect(screen.getByTestId("runes-tree")).toBeInTheDocument();
    expect(screen.getByText("Precisão")).toBeInTheDocument();
    expect(screen.getByText("Dominação")).toBeInTheDocument();
    expect(screen.getByText("Pressione o Ataque")).toBeInTheDocument();
  });
});

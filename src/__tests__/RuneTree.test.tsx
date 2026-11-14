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

const mockMultiPageBuild: ChampionBuild = {
  ...mockNormalBuild,
  rune_pages: [
    {
      primary_style_id: 8000,
      sub_style_id: 8100,
      primary_style_name: "Precisão",
      sub_style_name: "Dominação",
      selected_perk_ids: [8005, 9111, 9104, 8014, 8139, 8135, 5008, 5008, 5001],
      perks: mockNormalBuild.perks,
      play: 4200,
      win: 2200,
      pick_rate: 68.5,
    },
    {
      primary_style_id: 8100,
      sub_style_id: 8200,
      primary_style_name: "Dominação",
      sub_style_name: "Feitiçaria",
      selected_perk_ids: [8112, 8126, 8138, 8135, 8226, 8210, 5008, 5008, 5001],
      perks: mockNormalBuild.perks,
      play: 1800,
      win: 920,
      pick_rate: 31.5,
    },
  ],
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

  it("renders simplified stat shard labels with icons and full tooltip names", () => {
    render(
      <RuneTree
        build={mockNormalBuild}
        selectedRunePageIndex={0}
        onSelectRunePage={vi.fn()}
      />
    );

    expect(screen.getAllByText("Adaptativa")).toHaveLength(2);
    expect(screen.getByText("Vida Esc.")).toBeInTheDocument();
    expect(screen.getAllByTitle("Força Adaptativa")).toHaveLength(2);
    expect(screen.getByTitle("Vida")).toBeInTheDocument();
    expect(screen.getAllByAltText("Força Adaptativa")).toHaveLength(2);
    expect(screen.getByAltText("Vida")).toBeInTheDocument();
  });

  it("uses Página labels instead of Opção labels in readOnly mode", () => {
    const { rerender } = render(
      <RuneTree
        build={mockMultiPageBuild}
        selectedRunePageIndex={0}
        onSelectRunePage={vi.fn()}
      />
    );

    expect(screen.getByText("Opção 1 (Mais Usada)")).toBeInTheDocument();
    expect(screen.getByText("Opção 2")).toBeInTheDocument();

    rerender(
      <RuneTree
        build={mockMultiPageBuild}
        selectedRunePageIndex={0}
        onSelectRunePage={vi.fn()}
        readOnly={true}
      />
    );

    expect(screen.getByText("Página 1 (Mais Usada)")).toBeInTheDocument();
    expect(screen.getByText("Página 2")).toBeInTheDocument();
    expect(screen.queryByText("Opção 1 (Mais Usada)")).not.toBeInTheDocument();
  });
});

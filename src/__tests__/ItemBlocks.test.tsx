import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ItemBlocks } from "../components/ItemBlocks";
import { ChampionBuild } from "../types";

describe("ItemBlocks", () => {
  const mockBuild: ChampionBuild = {
    champion_id: 103,
    champion_name: "Ahri",
    win_rate: 51.5,
    pick_rate: 10.0,
    primary_style_id: 8100,
    sub_style_id: 8200,
    selected_perk_ids: [8112, 8126, 8138, 8135, 8226, 8210, 5008, 5008, 5001],
    spell1_id: 4,
    spell2_id: 14,
    item_blocks: [
      { title: "Itens Iniciais", item_ids: [1056, 2003], item_icon_urls: ["starter1.png", "starter2.png"] },
      { title: "Primeiros 3 Itens", item_ids: [3089, 3157, 3135], item_icon_urls: ["1.png", "2.png", "3.png"] },
      { title: "3 Botas Mais Feitas", item_ids: [3020], item_icon_urls: ["boots.png"] },
      { title: "Build Final", item_ids: [3089, 3157, 3135, 3020, 3165, 3102] },
      { title: "Itens Situacionais / Úteis", item_ids: [3157, 3102] },
    ],
    build_options: [
      {
        name: "Build 1 (Mais Feita)",
        core_item_ids: [3089, 3157, 3135],
        core_item_icon_urls: ["1.png", "2.png", "3.png"],
        final_item_ids: [3089, 3157, 3135, 3020, 3165, 3102],
        final_item_icon_urls: ["1.png", "2.png", "3.png", "4.png", "5.png", "6.png"],
        situational_item_ids: [3157, 3102],
        situational_item_icon_urls: ["zhonya.png", "banshee.png"],
        play: 10000,
        win: 5500,
        pick_rate: 60.0,
      },
      {
        name: "Build 2",
        core_item_ids: [6653, 3157, 3089],
        core_item_icon_urls: ["liandry.png", "zhonya.png", "rabadon.png"],
        final_item_ids: [6653, 3157, 3089, 3020, 3165, 3102],
        final_item_icon_urls: ["liandry.png", "zhonya.png", "rabadon.png", "4.png", "5.png", "6.png"],
        situational_item_ids: [3116, 3151],
        situational_item_icon_urls: ["rylais.png", "morello.png"],
        play: 5000,
        win: 2600,
        pick_rate: 30.0,
      },
    ],
  };

  it("renders all 5 shop blocks with rebranded layout, telemetry pill and handles build option selection", () => {
    const handleSelectBuild = vi.fn();

    const { container } = render(
      <ItemBlocks
        build={mockBuild}
        selectedBuildIndex={0}
        onSelectBuild={handleSelectBuild}
      />
    );

    expect(screen.getByTestId("item-blocks")).toBeInTheDocument();
    expect(screen.getByText("Build na Loja")).toBeInTheDocument();
    expect(screen.getByTestId("active-build-meta")).toHaveTextContent(/60\.0% escolha/i);
    expect(screen.getByTestId("active-build-meta")).toHaveTextContent(/10[.,]000 partidas/i);

    expect(screen.getByText("1. Itens Iniciais")).toBeInTheDocument();
    expect(screen.getByText("2. Primeiros 3 Itens")).toBeInTheDocument();
    expect(screen.getByText("3. 3 Botas Mais Feitas")).toBeInTheDocument();
    expect(screen.getByText("4. Build Final")).toBeInTheDocument();
    expect(screen.getByText("5. Itens Situacionais / Úteis")).toBeInTheDocument();

    expect(screen.getByText("Core Trio")).toBeInTheDocument();
    expect(screen.getByText("6 Itens")).toBeInTheDocument();
    expect(screen.getByText("Opcional")).toBeInTheDocument();
    expect(container.querySelectorAll(".item-flow-arrow")).toHaveLength(2);
    expect(container.querySelector(".block-kind-starter")).toBeInTheDocument();
    expect(container.querySelector(".block-kind-boots")).toBeInTheDocument();
    expect(container.querySelector(".block-kind-core")).toBeInTheDocument();
    expect(container.querySelector(".block-kind-final")).toBeInTheDocument();
    expect(container.querySelector(".block-kind-situational")).toBeInTheDocument();

    const build2Btn = screen.getByTestId("build-opt-btn-1");
    fireEvent.click(build2Btn);

    expect(handleSelectBuild).toHaveBeenCalledWith(1);
  });

  it("uses consultation labels in readOnly mode", () => {
    render(
      <ItemBlocks
        build={mockBuild}
        selectedBuildIndex={0}
        onSelectBuild={vi.fn()}
        readOnly={true}
      />
    );

    expect(screen.getByText("Build Recomendada")).toBeInTheDocument();
    expect(screen.queryByText("Build na Loja")).not.toBeInTheDocument();
    expect(screen.getByTestId("active-build-meta")).toHaveTextContent(/60\.0% uso/i);
    expect(screen.queryByText(/escolha/i)).not.toBeInTheDocument();
  });
});

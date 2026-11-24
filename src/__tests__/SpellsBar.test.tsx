import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import { SpellsBar } from "../components/ui/SpellsBar";
import { SpellMetadata, SpellPairOption } from "../types";

describe("SpellsBar", () => {
  const flash: SpellMetadata = {
    id: 4,
    name: "Flash",
    icon_url: "flash.png",
    description: "Teleporta seu Campeão por uma curta distância na direção do cursor.",
    cooldown_secs: 300,
  };
  const ignite: SpellMetadata = {
    id: 14,
    name: "Incendiar",
    icon_url: "ignite.png",
    description: "Incendeia o Campeão inimigo, causando Dano Verdadeiro ao longo de 5s.",
    cooldown_secs: 180,
  };
  const teleport: SpellMetadata = {
    id: 12,
    name: "Teleporte",
    icon_url: "teleport.png",
    description: "Após canalizar brevemente, teleporta seu Campeão até uma estrutura aliada.",
    cooldown_secs: 300,
  };
  const heal: SpellMetadata = {
    id: 7,
    name: "Curar",
    icon_url: "heal.png",
    description: "Restaura Vida do Campeão e de um aliado.",
    cooldown_secs: 240,
  };

  const mockSpells: SpellMetadata[] = [flash, ignite];
  const mockOptions: SpellPairOption[] = [
    {
      spell1_id: 4,
      spell2_id: 14,
      play: 84666,
      win: 43679,
      pick_rate: 69.5,
      spells: [flash, ignite],
    },
    {
      spell1_id: 4,
      spell2_id: 12,
      play: 34826,
      win: 17612,
      pick_rate: 28.6,
      spells: [flash, teleport],
    },
  ];

  it("renders top 2 spell pairs with both icons and names, purple active card, D/F badges only on active pair, and swap button", () => {
    const handleInvert = vi.fn();
    const handleSelectPair = vi.fn();

    render(
      <SpellsBar
        orderedSpells={mockSpells}
        spellOptions={mockOptions}
        selectedSpellPairIndex={0}
        onInvertSpells={handleInvert}
        onSelectSpellPair={handleSelectPair}
      />
    );

    expect(screen.getByText("Feitiços")).toBeInTheDocument();
    expect(screen.queryByText("Opções")).not.toBeInTheDocument();
    expect(screen.getByTestId("spell-pair-selector")).toHaveAttribute(
      "aria-label",
      "Escolha de Combinação de Feitiços"
    );

    const btn0 = screen.getByTestId("spell-pair-btn-0");
    expect(btn0.tagName).toBe("BUTTON");
    expect(btn0).toHaveClass("spell-pair-card", "active");
    expect(btn0).toHaveAttribute("aria-pressed", "true");
    expect(within(btn0).getByText("Flash")).toBeInTheDocument();
    expect(within(btn0).getByText("Incendiar")).toBeInTheDocument();
    expect(within(btn0).getByText("69.5%")).toBeInTheDocument();
    expect(within(btn0).getByText("D")).toBeInTheDocument();
    expect(within(btn0).getByText("F")).toBeInTheDocument();

    const btn1 = screen.getByTestId("spell-pair-btn-1");
    expect(btn1.tagName).toBe("BUTTON");
    expect(btn1).toHaveClass("spell-pair-card");
    expect(btn1).not.toHaveClass("active");
    expect(btn1).toHaveAttribute("aria-pressed", "false");
    expect(within(btn1).getByText("Flash")).toBeInTheDocument();
    expect(within(btn1).getByText("Teleporte")).toBeInTheDocument();
    expect(within(btn1).getByText("28.6%")).toBeInTheDocument();
    expect(within(btn1).queryByText("D")).not.toBeInTheDocument();
    expect(within(btn1).queryByText("F")).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId("invert-spells-btn"));
    expect(handleInvert).toHaveBeenCalledTimes(1);
  });

  it("caps displayed options at 2 pairs, allows switching pair selection, and renders rich pair tooltips", () => {
    const handleSelectPair = vi.fn();
    const extraOptions: SpellPairOption[] = [
      ...mockOptions,
      {
        spell1_id: 4,
        spell2_id: 7,
        play: 1200,
        win: 600,
        pick_rate: 1.9,
        spells: [flash, heal],
      },
    ];

    render(
      <SpellsBar
        orderedSpells={mockSpells}
        spellOptions={extraOptions}
        selectedSpellPairIndex={0}
        onInvertSpells={vi.fn()}
        onSelectSpellPair={handleSelectPair}
      />
    );

    expect(screen.getByTestId("spell-pair-btn-0")).toBeInTheDocument();
    const tpPairBtn = screen.getByTestId("spell-pair-btn-1");
    expect(screen.queryByTestId("spell-pair-btn-2")).not.toBeInTheDocument();
    expect(tpPairBtn).toHaveAccessibleName("Flash + Teleporte");
    expect(tpPairBtn.querySelectorAll("img")).toHaveLength(2);

    fireEvent.focus(tpPairBtn);
    const pairTooltip = screen.getByRole("tooltip");
    expect(pairTooltip).toHaveTextContent("Flash + Teleporte");
    expect(pairTooltip).toHaveTextContent("2ª Mais Usada");
    expect(pairTooltip).toHaveTextContent("28.6%");
    expect(pairTooltip).toHaveTextContent(/34\.826 partidas/);
    expect(pairTooltip).toHaveTextContent(/Teleporta seu Campeão/);

    fireEvent.click(tpPairBtn);
    expect(handleSelectPair).toHaveBeenCalledWith(1);
  });

  it("renders non-clickable hover-only spell cards and swap indicator with identical visual layout in readOnly mode", () => {
    const handleSelectPair = vi.fn();
    const handleInvert = vi.fn();

    render(
      <SpellsBar
        orderedSpells={mockSpells}
        spellOptions={mockOptions}
        selectedSpellPairIndex={1}
        onSelectSpellPair={handleSelectPair}
        onInvertSpells={handleInvert}
        readOnly={true}
      />
    );

    expect(screen.getByTestId("spell-pair-selector")).toHaveAttribute(
      "aria-label",
      "Combinações de Feitiços"
    );
    expect(screen.queryByText("Opções")).not.toBeInTheDocument();
    expect(screen.queryByTestId("spell-pair-btn-0")).not.toBeInTheDocument();
    expect(screen.queryByTestId("spell-pair-btn-1")).not.toBeInTheDocument();
    expect(screen.queryByTestId("invert-spells-btn")).not.toBeInTheDocument();

    const info0 = screen.getByTestId("spell-pair-info-0");
    expect(info0.tagName).toBe("DIV");
    expect(info0).toHaveClass("spell-pair-card", "is-readonly", "active");
    expect(within(info0).getByText("Flash")).toBeInTheDocument();
    expect(within(info0).getByText("Incendiar")).toBeInTheDocument();
    expect(within(info0).getByText("69.5%")).toBeInTheDocument();
    expect(within(info0).getByText("D")).toBeInTheDocument();
    expect(within(info0).getByText("F")).toBeInTheDocument();

    const info1 = screen.getByTestId("spell-pair-info-1");
    expect(info1.tagName).toBe("DIV");
    expect(info1).toHaveClass("spell-pair-card", "is-readonly");
    expect(info1).not.toHaveClass("active");
    expect(within(info1).getByText("Flash")).toBeInTheDocument();
    expect(within(info1).getByText("Teleporte")).toBeInTheDocument();
    expect(within(info1).getByText("28.6%")).toBeInTheDocument();
    expect(within(info1).queryByText("D")).not.toBeInTheDocument();
    expect(within(info1).queryByText("F")).not.toBeInTheDocument();

    fireEvent.mouseEnter(info1);
    const pairTooltip = screen.getByRole("tooltip");
    expect(pairTooltip).toHaveTextContent("Flash + Teleporte");
    expect(pairTooltip).toHaveTextContent("Alternativa");
    expect(pairTooltip).toHaveTextContent("28.6%");
    fireEvent.mouseLeave(info1);

    fireEvent.click(info1);
    expect(handleSelectPair).not.toHaveBeenCalled();

    const readonlySwap = screen.getByTestId("invert-spells-readonly");
    expect(readonlySwap.tagName).toBe("SPAN");
    expect(readonlySwap).toHaveClass("invert-spells-btn", "is-readonly");
    expect(readonlySwap).toHaveAttribute("aria-disabled", "true");
    fireEvent.click(readonlySwap);
    expect(handleInvert).not.toHaveBeenCalled();

    fireEvent.mouseEnter(readonlySwap);
    expect(screen.getByRole("tooltip")).toHaveTextContent(
      "Troca de D/F disponível na seleção de campeões"
    );
  });

  it("renders single squircle frames inside pair cards without nested frames", () => {
    render(
      <SpellsBar
        orderedSpells={mockSpells}
        spellOptions={mockOptions}
        selectedSpellPairIndex={0}
        onInvertSpells={vi.fn()}
        onSelectSpellPair={vi.fn()}
      />
    );

    const activeBtn = screen.getByTestId("spell-pair-btn-0");
    expect(activeBtn).toHaveClass("spell-pair-card", "active");
    const frames = activeBtn.querySelectorAll(".dd-icon-frame");
    expect(frames).toHaveLength(2);
    frames.forEach((frame) => {
      expect(frame.querySelector(".dd-icon-frame")).toBeNull();
    });
  });
});

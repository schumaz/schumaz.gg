import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { SpellsBar } from "../components/ui/SpellsBar";
import { SpellMetadata } from "../types";

describe("SpellsBar", () => {
  const mockSpells: SpellMetadata[] = [
    {
      id: 4,
      name: "Flash",
      icon_url: "flash.png",
      description: "Teleporta seu Campeão por uma curta distância na direção do cursor.",
      cooldown_secs: 300,
    },
    {
      id: 14,
      name: "Incendiar",
      icon_url: "ignite.png",
      description: "Incendeia o Campeão inimigo, causando Dano Verdadeiro ao longo de 5s.",
      cooldown_secs: 180,
    },
  ];

  const mockChoices: SpellMetadata[] = [
    {
      id: 14,
      name: "Incendiar",
      icon_url: "ignite.png",
      description: "Incendeia o Campeão inimigo, causando Dano Verdadeiro ao longo de 5s.",
      cooldown_secs: 180,
    },
    {
      id: 12,
      name: "Teleporte",
      icon_url: "teleport.png",
      description: "Após canalizar brevemente, teleporta seu Campeão até uma estrutura ou tropa aliada.",
      cooldown_secs: 300,
    },
  ];

  it("renders ordered spells in D and F slots and triggers inversion", () => {
    const handleInvert = vi.fn();
    const handleSelectSecondary = vi.fn();

    render(
      <SpellsBar
        orderedSpells={mockSpells}
        spellChoices={mockChoices}
        selectedSecondarySpellId={null}
        onInvertSpells={handleInvert}
        onSelectSecondarySpell={handleSelectSecondary}
      />
    );

    expect(screen.getByText("Flash")).toBeInTheDocument();
    expect(screen.getAllByText("Incendiar").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("Escolha de Segundo Feitiço")).toBeInTheDocument();
    expect(screen.getByText("D")).toBeInTheDocument();
    expect(screen.getByText("F")).toBeInTheDocument();

    const invertBtn = screen.getByTestId("invert-spells-btn");
    fireEvent.click(invertBtn);

    expect(handleInvert).toHaveBeenCalledTimes(1);
  });

  it("allows selecting an alternative secondary spell and renders rich spell tooltips", () => {
    const handleInvert = vi.fn();
    const handleSelectSecondary = vi.fn();

    render(
      <SpellsBar
        orderedSpells={mockSpells}
        spellChoices={mockChoices}
        selectedSecondarySpellId={null}
        onInvertSpells={handleInvert}
        onSelectSecondarySpell={handleSelectSecondary}
      />
    );

    const flashTrigger = screen.getByLabelText("Flash");
    fireEvent.mouseEnter(flashTrigger);
    const flashTooltip = screen.getByRole("tooltip");
    expect(flashTooltip).toHaveTextContent("Flash");
    expect(flashTooltip).toHaveTextContent("300s");
    expect(flashTooltip).toHaveTextContent(/Teleporta seu Campeão/);
    fireEvent.mouseLeave(flashTrigger);

    const teleportBtn = screen.getByTestId("sec-spell-btn-12");
    fireEvent.focus(teleportBtn);
    const tpTooltip = screen.getByRole("tooltip");
    expect(tpTooltip).toHaveTextContent("Teleporte");
    expect(tpTooltip).toHaveTextContent("Segundo Mais Usado");
    expect(tpTooltip).toHaveTextContent("300s");

    fireEvent.click(teleportBtn);
    expect(handleSelectSecondary).toHaveBeenCalledWith(12);
  });

  it("renders read-only consultation UI in Ranked mode without D/F keys, invert button, or Runas choice wording", () => {
    const { container } = render(
      <SpellsBar
        orderedSpells={mockSpells}
        spellChoices={mockChoices}
        selectedSecondarySpellId={null}
        onInvertSpells={vi.fn()}
        onSelectSecondarySpell={vi.fn()}
        readOnly={true}
      />
    );

    expect(screen.getByText("Feitiços que Pode Usar")).toBeInTheDocument();
    expect(screen.queryByText("Escolha de Segundo Feitiço")).not.toBeInTheDocument();
    expect(screen.queryByTestId("invert-spells-btn")).not.toBeInTheDocument();
    expect(container.querySelectorAll(".spell-key-indicator")).toHaveLength(0);
    expect(screen.queryByText("D")).not.toBeInTheDocument();
    expect(screen.queryByText("F")).not.toBeInTheDocument();

    expect(screen.getByTestId("sec-spell-info-14")).toBeInTheDocument();
    expect(screen.getByTestId("sec-spell-info-12")).toBeInTheDocument();
    expect(screen.queryByTestId("sec-spell-btn-14")).not.toBeInTheDocument();
    expect(screen.queryByTestId("sec-spell-btn-12")).not.toBeInTheDocument();
  });
});

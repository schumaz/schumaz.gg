import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { SpellsBar } from "../components/SpellsBar";
import { SpellMetadata } from "../types";

describe("SpellsBar", () => {
  const mockSpells: SpellMetadata[] = [
    { id: 4, name: "Flash", icon_url: "flash.png" },
    { id: 14, name: "Incendiar", icon_url: "ignite.png" },
  ];

  const mockChoices: SpellMetadata[] = [
    { id: 14, name: "Incendiar", icon_url: "ignite.png" },
    { id: 12, name: "Teleporte", icon_url: "teleport.png" },
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

  it("allows selecting an alternative secondary spell", () => {
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

    const teleportBtn = screen.getByTestId("sec-spell-btn-12");
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

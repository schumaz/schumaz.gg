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
});

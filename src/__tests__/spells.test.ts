import { describe, it, expect } from "vitest";
import { computeOrderedSpells } from "../utils/spells";
import { SpellMetadata } from "../types";

describe("computeOrderedSpells", () => {
  const flash: SpellMetadata = { id: 4, name: "Flash", icon_url: "flash.png" };
  const ignite: SpellMetadata = { id: 14, name: "Incendiar", icon_url: "ignite.png" };
  const teleport: SpellMetadata = { id: 12, name: "Teleporte", icon_url: "teleport.png" };
  const smite: SpellMetadata = { id: 11, name: "Golpear", icon_url: "smite.png" };

  it("positions Flash on D when flashKey is D", () => {
    const base = [ignite, flash]; // Flash on F initially
    const result = computeOrderedSpells(base, null, undefined, "D", false);

    expect(result[0].id).toBe(4);
    expect(result[1].id).toBe(14);
  });

  it("positions Flash on F when flashKey is F", () => {
    const base = [flash, ignite]; // Flash on D initially
    const result = computeOrderedSpells(base, null, undefined, "F", false);

    expect(result[0].id).toBe(14);
    expect(result[1].id).toBe(4);
  });

  it("replaces secondary spell when secondarySpellId is provided", () => {
    const base = [flash, ignite];
    const choices = [ignite, teleport];

    const result = computeOrderedSpells(base, 12, choices, "D", false);

    expect(result[0].id).toBe(4);
    expect(result[1].id).toBe(12);
  });

  it("inverts spells when inverted flag is true", () => {
    const base = [flash, smite];
    const result = computeOrderedSpells(base, null, undefined, "D", true);

    expect(result[0].id).toBe(11);
    expect(result[1].id).toBe(4);
  });
});

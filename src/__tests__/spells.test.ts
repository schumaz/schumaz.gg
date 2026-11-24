import { describe, it, expect } from "vitest";
import { computeOrderedSpells, orderSpellPair } from "../utils/spells";
import { SpellMetadata, SpellPairOption } from "../types";

describe("computeOrderedSpells & orderSpellPair", () => {
  const flash: SpellMetadata = { id: 4, name: "Flash", icon_url: "flash.png" };
  const ignite: SpellMetadata = { id: 14, name: "Incendiar", icon_url: "ignite.png" };
  const teleport: SpellMetadata = { id: 12, name: "Teleporte", icon_url: "teleport.png" };
  const smite: SpellMetadata = { id: 11, name: "Golpear", icon_url: "smite.png" };
  const heal: SpellMetadata = { id: 7, name: "Curar", icon_url: "heal.png" };
  const barrier: SpellMetadata = { id: 21, name: "Barreira", icon_url: "barrier.png" };
  const ghost: SpellMetadata = { id: 6, name: "Fantasma", icon_url: "ghost.png" };
  const mark: SpellMetadata = { id: 32, name: "Marca", icon_url: "mark.png" };

  it("positions Flash on D when flashKey is D", () => {
    const base = [ignite, flash];
    const result = computeOrderedSpells(base, 0, undefined, "D", false);

    expect(result[0].id).toBe(4);
    expect(result[1].id).toBe(14);
  });

  it("positions Flash on F when flashKey is F", () => {
    const base = [flash, ignite];
    const result = computeOrderedSpells(base, 0, undefined, "F", false);

    expect(result[0].id).toBe(14);
    expect(result[1].id).toBe(4);
  });

  it("handles ARAM Tristana spell pair switching while respecting Flash key preference", () => {
    const base = [flash, heal];
    const options: SpellPairOption[] = [
      { spell1_id: 4, spell2_id: 7, play: 1125, win: 599, pick_rate: 33.6, spells: [flash, heal] },
      { spell1_id: 4, spell2_id: 21, play: 695, win: 404, pick_rate: 20.8, spells: [flash, barrier] },
    ];

    const primaryD = computeOrderedSpells(base, 0, options, "D", false);
    expect(primaryD.map((s) => s.name)).toEqual(["Flash", "Curar"]);

    const secondaryD = computeOrderedSpells(base, 1, options, "D", false);
    expect(secondaryD.map((s) => s.name)).toEqual(["Flash", "Barreira"]);

    const secondaryF = computeOrderedSpells(base, 1, options, "F", false);
    expect(secondaryF.map((s) => s.name)).toEqual(["Barreira", "Flash"]);
  });

  it("switches both spells cleanly when selecting a pair with different primary spell (e.g. Darius)", () => {
    const base = [flash, ghost];
    const options: SpellPairOption[] = [
      { spell1_id: 4, spell2_id: 6, play: 51000, win: 26500, pick_rate: 64.0, spells: [flash, ghost] },
      { spell1_id: 6, spell2_id: 12, play: 18500, win: 9400, pick_rate: 23.0, spells: [ghost, teleport] },
    ];

    const switched = computeOrderedSpells(base, 1, options, "F", false);
    expect(switched.map((s) => s.id)).toEqual([6, 12]);
  });

  it("keeps Flash ahead of Marca when OP.GG returns Marca first and flashKey is D", () => {
    const result = orderSpellPair([mark, flash], "D", false);
    expect(result.map((s) => s.name)).toEqual(["Flash", "Marca"]);
  });

  it("inverts spells when inverted flag is true", () => {
    const base = [flash, smite];
    const result = computeOrderedSpells(base, 0, undefined, "D", true);

    expect(result[0].id).toBe(11);
    expect(result[1].id).toBe(4);
  });
});

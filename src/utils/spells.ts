import { FlashKeyPreference, SpellMetadata } from "../types";

export function computeOrderedSpells(
  baseSpells: SpellMetadata[] | undefined,
  secondarySpellId: number | null,
  spellChoices: SpellMetadata[] | undefined,
  flashKey: FlashKeyPreference,
  inverted: boolean
): SpellMetadata[] {
  if (!baseSpells || baseSpells.length < 2) return baseSpells || [];
  const spellsCopy = [...baseSpells];

  if (secondarySpellId && spellChoices && spellChoices.length > 0) {
    const chosen = spellChoices.find((c) => c.id === secondarySpellId);
    if (chosen) {
      const secondaryIds = new Set(spellChoices.map((c) => c.id));
      const idxToReplace = spellsCopy.findIndex((s) => secondaryIds.has(s.id));
      if (idxToReplace !== -1) {
        spellsCopy[idxToReplace] = chosen;
      } else {
        const nonAnchorIdx = spellsCopy.findIndex((s) => s.id !== 4 && s.id !== 11);
        if (nonAnchorIdx !== -1) {
          spellsCopy[nonAnchorIdx] = chosen;
        } else {
          spellsCopy[1] = chosen;
        }
      }
    }
  }

  const flashIndex = spellsCopy.findIndex((s) => s.id === 4);
  if (flashIndex !== -1) {
    if (flashKey === "F" && flashIndex === 0) {
      spellsCopy.reverse();
    } else if (flashKey === "D" && flashIndex === 1) {
      spellsCopy.reverse();
    }
  }

  if (inverted) {
    spellsCopy.reverse();
  }

  return spellsCopy;
}

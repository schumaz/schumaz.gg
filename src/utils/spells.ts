import { FlashKeyPreference, SpellMetadata, SpellPairOption } from "../types";

export function orderSpellPair(
  pair: SpellMetadata[] | undefined,
  flashKey: FlashKeyPreference,
  inverted = false
): SpellMetadata[] {
  if (!pair || pair.length < 2) return pair || [];
  const ordered = [pair[0], pair[1]];

  const flashIndex = ordered.findIndex((s) => s.id === 4);
  if (flashIndex !== -1) {
    if ((flashKey === "F" && flashIndex === 0) || (flashKey === "D" && flashIndex === 1)) {
      ordered.reverse();
    }
  }

  if (inverted) {
    ordered.reverse();
  }

  return ordered;
}

export function computeOrderedSpells(
  baseSpells: SpellMetadata[] | undefined,
  selectedPairIndex: number,
  spellOptions: SpellPairOption[] | undefined,
  flashKey: FlashKeyPreference,
  inverted: boolean
): SpellMetadata[] {
  const chosenPair =
    spellOptions &&
    selectedPairIndex >= 0 &&
    selectedPairIndex < spellOptions.length &&
    spellOptions[selectedPairIndex]?.spells &&
    spellOptions[selectedPairIndex].spells.length >= 2
      ? spellOptions[selectedPairIndex].spells
      : baseSpells;

  return orderSpellPair(chosenPair, flashKey, inverted);
}

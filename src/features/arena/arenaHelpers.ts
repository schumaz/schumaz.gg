import type { ArenaChampionEntry } from "../../types";

export interface ArenaTierGroup {
  tier: number;
  items: ArenaChampionEntry[];
}

/**
 * Normalizes a champion search query by removing diacritics and whitespace.
 */
export function normalizeArenaSearch(value: string): string {
  return value
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/\s+/g, "");
}

function compareArenaEntries(a: ArenaChampionEntry, b: ArenaChampionEntry): number {
  const aValid = a.rank > 0 ? 0 : 1;
  const bValid = b.rank > 0 ? 0 : 1;
  if (aValid !== bValid) return aValid - bValid;
  if (a.rank !== b.rank) return a.rank - b.rank;

  const aWr = a.win_rate ?? -1;
  const bWr = b.win_rate ?? -1;
  if (aWr !== bWr) return bWr - aWr;

  const aFirst = a.first_place_rate ?? -1;
  const bFirst = b.first_place_rate ?? -1;
  if (aFirst !== bFirst) return bFirst - aFirst;

  const aPr = a.pick_rate ?? -1;
  const bPr = b.pick_rate ?? -1;
  if (aPr !== bPr) return bPr - aPr;

  return a.name.localeCompare(b.name);
}

/**
 * Groups Arena tier list entries by tier (`0` = OP, `1`..`5`) sorted by rank ascending.
 */
export function groupArenaByTier(entries: ArenaChampionEntry[]): ArenaTierGroup[] {
  const tierMap = new Map<number, ArenaChampionEntry[]>();
  for (const item of entries) {
    const normalizedTier = Math.max(0, item.tier);
    const list = tierMap.get(normalizedTier);
    if (list) {
      list.push(item);
    } else {
      tierMap.set(normalizedTier, [item]);
    }
  }

  return Array.from(tierMap.entries())
    .sort((a, b) => a[0] - b[0])
    .map(([tier, items]) => ({
      tier,
      items: [...items].sort(compareArenaEntries),
    }));
}

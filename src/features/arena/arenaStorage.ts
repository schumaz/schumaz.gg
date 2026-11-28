import type { AppLocale, ArenaChampionEntry, ArenaSynergyEntry } from "../../types";

export interface ArenaFilters {
  patch: string | null;
}

export const DEFAULT_ARENA_FILTERS: ArenaFilters = {
  patch: null,
};

const ARENA_PATCH_STORAGE_KEY = "schumaz.arena.filterPatch";
const ARENA_TIER_STORAGE_KEY = "schumaz.arena.championTierFilter";
const VALID_ARENA_TIERS = new Set<string>(["all", "0", "1", "2", "3", "4"]);

/**
 * Builds the in-memory tier list cache key scoped by patch, 48h patch protection, and locale.
 */
export function buildArenaCacheKey(
  filters: ArenaFilters,
  protection: boolean,
  locale: AppLocale
): string {
  return `${filters.patch ?? ""}:${protection ? "1" : "0"}:${locale}`;
}

/**
 * Loads the persisted Arena patch filter from localStorage, falling back to automatic latest patch.
 */
export function loadSavedArenaFilters(): ArenaFilters {
  try {
    if (typeof window === "undefined" || !window.localStorage) {
      return DEFAULT_ARENA_FILTERS;
    }
    const rawPatch = window.localStorage.getItem(ARENA_PATCH_STORAGE_KEY)?.trim() ?? "";
    return { patch: rawPatch || null };
  } catch {
    return DEFAULT_ARENA_FILTERS;
  }
}

/**
 * Persists the selected Arena patch filter to localStorage.
 */
export function saveArenaFilters(filters: ArenaFilters): void {
  try {
    if (typeof window === "undefined" || !window.localStorage) return;
    if (filters.patch) {
      window.localStorage.setItem(ARENA_PATCH_STORAGE_KEY, filters.patch);
    } else {
      window.localStorage.removeItem(ARENA_PATCH_STORAGE_KEY);
    }
  } catch {
    // Ignore storage quota or restricted context errors.
  }
}

/**
 * Loads the persisted champion tier filter (`all`, `0`..`4`) for the Arena tier list.
 */
export function loadSavedArenaTierFilter(): string {
  try {
    if (typeof window === "undefined" || !window.localStorage) return "all";
    const raw = window.localStorage.getItem(ARENA_TIER_STORAGE_KEY)?.trim() ?? "";
    return VALID_ARENA_TIERS.has(raw) ? raw : "all";
  } catch {
    return "all";
  }
}

/**
 * Persists the champion tier filter (`all`, `0`..`4`) for the Arena tier list.
 */
export function saveArenaTierFilter(tier: string): void {
  try {
    if (typeof window === "undefined" || !window.localStorage) return;
    const normalized = VALID_ARENA_TIERS.has(tier) ? tier : "all";
    window.localStorage.setItem(ARENA_TIER_STORAGE_KEY, normalized);
  } catch {
    // Ignore storage quota or restricted context errors.
  }
}

export function synergyToArenaChampionEntry(syn: ArenaSynergyEntry): ArenaChampionEntry {
  return {
    id: syn.champion_id,
    key: syn.key,
    name: syn.name,
    icon_url: syn.icon_url,
    tier: 0,
    rank: 0,
    win_rate: syn.win_rate ?? null,
    first_place_rate: syn.first_place_rate ?? null,
    avg_placement: syn.avg_placement ?? null,
    pick_rate: syn.pick_rate ?? null,
    ban_rate: null,
    play: syn.play ?? null,
  };
}

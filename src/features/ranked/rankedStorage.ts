import {
  ChampionBuild,
  NavigationStatePatch,
  RANKED_FILTERS_STORAGE_KEY,
  RankedChampionEntry,
  RankedLane,
} from "../../types";

export type RankedLaneFilter = "all" | RankedLane;

export interface RankedFilters {
  region: string;
  tier: string;
  lane: RankedLaneFilter;
  patch: string | null;
}

export const DEFAULT_RANKED_FILTERS: RankedFilters = {
  region: "global",
  tier: "emerald_plus",
  lane: "all",
  patch: null,
};

export const VALID_LANES = new Set<string>(["all", "top", "jungle", "mid", "adc", "support"]);

export function loadSavedRankedFilters(): RankedFilters {
  try {
    const raw = localStorage.getItem(RANKED_FILTERS_STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return {
        region:
          typeof parsed.region === "string" && parsed.region.trim()
            ? parsed.region.trim()
            : DEFAULT_RANKED_FILTERS.region,
        tier:
          typeof parsed.tier === "string" && parsed.tier.trim()
            ? parsed.tier.trim()
            : DEFAULT_RANKED_FILTERS.tier,
        lane:
          typeof parsed.lane === "string" && VALID_LANES.has(parsed.lane)
            ? (parsed.lane as RankedLaneFilter)
            : DEFAULT_RANKED_FILTERS.lane,
        patch:
          typeof parsed.patch === "string" && parsed.patch.trim()
            ? parsed.patch.trim()
            : null,
      };
    }
  } catch {
    // Ignore storage access or parse errors
  }
  return { ...DEFAULT_RANKED_FILTERS };
}

export function saveRankedFilters(filters: RankedFilters): void {
  try {
    localStorage.setItem(RANKED_FILTERS_STORAGE_KEY, JSON.stringify(filters));
  } catch {
    // Ignore storage quota errors
  }
}

export interface UseRankedPersistenceOptions {
  iniBacked?: boolean;
  savedFilters?: {
    region?: string | null;
    tier?: string | null;
    lane?: string | null;
    patch?: string | null;
  };
  onNavigationPatch?: (patch: NavigationStatePatch) => void;
}

export function extractIniRankedFilters(
  sf: NonNullable<UseRankedPersistenceOptions["savedFilters"]>
): RankedFilters {
  return {
    region: sf.region?.trim() || DEFAULT_RANKED_FILTERS.region,
    tier: sf.tier?.trim() || DEFAULT_RANKED_FILTERS.tier,
    lane:
      sf.lane && VALID_LANES.has(sf.lane)
        ? (sf.lane as RankedLaneFilter)
        : DEFAULT_RANKED_FILTERS.lane,
    patch: sf.patch?.trim() ? sf.patch.trim() : null,
  };
}

export function buildRankedCacheKey(f: RankedFilters, protection: boolean): string {
  return `${f.region}:${f.tier}:${f.patch ?? ""}:${protection ? "1" : "0"}`;
}

export function applyTierListFilterChange(
  prev: RankedFilters,
  filterType: "position" | "tier" | "region" | "patch",
  value: string
): RankedFilters {
  const normalized = value.trim();
  if (filterType === "position") {
    const nextLane: RankedLaneFilter = VALID_LANES.has(normalized)
      ? (normalized as RankedLaneFilter)
      : "all";
    return { ...prev, lane: nextLane };
  }
  if (filterType === "region") return { ...prev, region: normalized || "global" };
  if (filterType === "tier") return { ...prev, tier: normalized || "emerald_plus" };
  return { ...prev, patch: normalized || null };
}

export function computeNextDetailRankedFilters(
  filters: RankedFilters,
  championBuild: ChampionBuild | null,
  currentChamp: RankedChampionEntry | null,
  filterType: "position" | "tier" | "region" | "patch",
  value: string
): { nextFilters: RankedFilters; nextLane: RankedLane } {
  const normalized = value.trim();
  const currentLane =
    (championBuild?.active_position as RankedLane | undefined) ??
    currentChamp?.lane ??
    "mid";

  const nextFilters: RankedFilters = {
    ...filters,
    region: filterType === "region" ? normalized || "global" : filters.region,
    tier: filterType === "tier" ? normalized || "emerald_plus" : filters.tier,
    lane:
      filterType === "position" && VALID_LANES.has(normalized)
        ? (normalized as RankedLaneFilter)
        : filters.lane,
    patch:
      filterType === "patch"
        ? normalized || null
        : championBuild?.patch_fallback_applied
        ? null
        : filters.patch,
  };

  const nextLane =
    filterType === "position" && VALID_LANES.has(normalized) && normalized !== "all"
      ? (normalized as RankedLane)
      : currentLane;

  return { nextFilters, nextLane };
}

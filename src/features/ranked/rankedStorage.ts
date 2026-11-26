import {
  ChampionBuild,
  NavigationStatePatch,
  RankedChampionEntry,
  RankedLane,
  RankedTierListResponse,
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
const VALID_DETAIL_LANES = new Set<string>(["top", "jungle", "mid", "adc", "support"]);

export interface RankedChampionShortcut {
  championId: number;
  championName: string;
  championIconUrl: string;
}

export interface ResolvedRankedShortcut {
  entry: RankedChampionEntry;
  positionOverride: RankedLane | null;
}

export function extractValidRankedLane(pos?: string | null): RankedLane | null {
  const normalized = pos?.trim().toLowerCase() ?? "";
  return VALID_DETAIL_LANES.has(normalized) ? (normalized as RankedLane) : null;
}

export function resolveRankedShortcutEntry(
  shortcut: RankedChampionShortcut,
  tierData: RankedTierListResponse | null
): ResolvedRankedShortcut {
  const matches = (tierData?.champions ?? []).filter((c) => c.id === shortcut.championId);
  if (matches.length > 0) {
    const primary = [...matches].sort(
      (a, b) =>
        (b.role_rate ?? -1) - (a.role_rate ?? -1) ||
        (a.overall_rank || Number.MAX_SAFE_INTEGER) - (b.overall_rank || Number.MAX_SAFE_INTEGER) ||
        (a.rank || Number.MAX_SAFE_INTEGER) - (b.rank || Number.MAX_SAFE_INTEGER)
    )[0];
    return { entry: primary, positionOverride: primary.lane };
  }

  return {
    entry: {
      id: shortcut.championId,
      key: String(shortcut.championId),
      name: shortcut.championName.trim() || `#${shortcut.championId}`,
      icon_url: shortcut.championIconUrl || "",
      lane: "mid",
      tier: 4,
      rank: 0,
      overall_rank: 0,
      win_rate: null,
      pick_rate: null,
      ban_rate: null,
      role_rate: null,
    },
    positionOverride: null,
  };
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

export function resolveRankedMatchupEntry(
  championId: number,
  tierData: RankedTierListResponse | null,
  championBuild: ChampionBuild | null,
  currentChampion: RankedChampionEntry | null,
  filters: RankedFilters
): RankedChampionEntry | null {
  const rawPos = championBuild?.active_position?.trim().toLowerCase() ?? "";
  const targetLane: RankedLane =
    rawPos !== "all" && VALID_LANES.has(rawPos)
      ? (rawPos as RankedLane)
      : currentChampion?.lane ?? (filters.lane !== "all" ? filters.lane : "mid");

  const allChampions = tierData?.champions ?? [];
  const exactMatch = allChampions.find((c) => c.id === championId && c.lane === targetLane);
  if (exactMatch) return exactMatch;

  const anyLaneMatch = allChampions.find((c) => c.id === championId);
  if (anyLaneMatch) {
    return { ...anyLaneMatch, lane: targetLane, rank: 0, overall_rank: 0 };
  }

  const matchupList = [
    ...(championBuild?.lane_matchups?.hard ?? []),
    ...(championBuild?.lane_matchups?.favorable ?? []),
  ];
  const fromMatchup = matchupList.find((m) => m.champion_id === championId);
  if (!fromMatchup) return null;

  return {
    id: fromMatchup.champion_id,
    key: fromMatchup.key?.trim() || String(fromMatchup.champion_id),
    name: fromMatchup.name?.trim() || `#${fromMatchup.champion_id}`,
    icon_url: fromMatchup.icon_url ?? "",
    lane: targetLane,
    tier: 4,
    rank: 0,
    overall_rank: 0,
    win_rate: null,
    pick_rate: null,
    ban_rate: null,
    role_rate: null,
  };
}

import { AppLocale, ChampionMasteryInfo, PlayerProfile, RecentMatchInfo } from "../../types";
import { resolveCachedAssetUrl } from "../../utils/assets";

const CDRAGON_RANKED_EMBLEM_BASE =
  "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-shared-components/global/default/images";
const CDRAGON_RANKED_MINI_CRESTS_BASE =
  "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/images/ranked-mini-crests";

const VALID_RANK_TIERS = new Set([
  "iron",
  "bronze",
  "silver",
  "gold",
  "platinum",
  "emerald",
  "diamond",
  "master",
  "grandmaster",
  "challenger",
  "unranked",
]);

function normalizeRankTierKey(tier: string): string {
  const normalized = tier.trim().toLowerCase();
  return VALID_RANK_TIERS.has(normalized) ? normalized : "unranked";
}

export function getRankedCrestUrl(tier: string): string {
  return resolveCachedAssetUrl(`${CDRAGON_RANKED_EMBLEM_BASE}/${normalizeRankTierKey(tier)}.png`);
}

export function getRankedMiniCrestUrl(tier: string): string {
  return resolveCachedAssetUrl(
    `${CDRAGON_RANKED_MINI_CRESTS_BASE}/${normalizeRankTierKey(tier)}.svg`
  );
}

export function formatDuration(secs: number): string {
  const mins = Math.floor(Math.max(0, secs) / 60);
  const rem = Math.max(0, secs) % 60;
  return `${mins}m ${rem.toString().padStart(2, "0")}s`;
}

export function getMultiKillBadge(match: RecentMatchInfo): string | null {
  if (match.penta_kills > 0) return "Penta Kill";
  if (match.quadra_kills > 0) return "Quadra Kill";
  if (match.triple_kills > 0) return "Triple Kill";
  if (match.double_kills > 0) return "Double Kill";
  return null;
}

export interface AvailableModeItem {
  key: string;
  label: string;
  count: number;
}

export interface WinLossStats {
  total: number;
  wins: number;
  losses: number;
  winRate: number;
}

export interface HighlightedChampion {
  name: string;
  icon: string;
  games: number;
  wins: number;
  kdaSum: number;
}

export interface ProfileChampionTarget {
  championId: number;
  championName: string;
  championIconUrl: string;
}

export type MasterySortKey = "points" | "level" | "recent" | "name";

export interface MasterySummaryStats {
  totalLevel: number;
  totalPoints: number;
  championCount: number;
}

export function filterValidMatchList(matches: RecentMatchInfo[]): RecentMatchInfo[] {
  return matches.filter((m) => {
    const modeLabelLower = m.game_mode_label.toLowerCase();
    return (
      m.queue_id >= 0 &&
      m.game_mode_key.toLowerCase() !== "practicetool" &&
      !modeLabelLower.includes("treino") &&
      !modeLabelLower.includes("practice")
    );
  });
}

export function filterValidMatches(profile: PlayerProfile | null): RecentMatchInfo[] {
  if (!profile?.recent_matches) return [];
  return filterValidMatchList(profile.recent_matches);
}

export function computeAvailableModes(validMatches: RecentMatchInfo[]): AvailableModeItem[] {
  const modeMap = new Map<string, AvailableModeItem>();
  for (const match of validMatches) {
    const existing = modeMap.get(match.game_mode_key);
    if (existing) {
      existing.count += 1;
    } else {
      modeMap.set(match.game_mode_key, {
        key: match.game_mode_key,
        label: match.game_mode_label,
        count: 1,
      });
    }
  }
  return Array.from(modeMap.values());
}

export function formatArenaPlacement(placement: number, locale: AppLocale): string {
  if (locale === "pt_BR") {
    return `${placement}º`;
  }
  if (placement === 1) return "1st";
  if (placement === 2) return "2nd";
  if (placement === 3) return "3rd";
  return `${placement}th`;
}

export function getArenaPlacementTone(placement: number): "gold" | "top" | "bottom" {
  if (placement === 1) return "gold";
  if (placement <= 4) return "top";
  return "bottom";
}

export function computeWinLossStats(filteredMatches: RecentMatchInfo[]): WinLossStats {
  const counted = filteredMatches.filter((m) => !m.remake);
  const total = counted.length;
  const wins = counted.filter((m) => m.win).length;
  const losses = total - wins;
  const winRate = total > 0 ? Math.round((wins / total) * 100) : 0;
  return { total, wins, losses, winRate };
}

export function computeHighlightedChampion(
  filteredMatches: RecentMatchInfo[]
): HighlightedChampion | null {
  const counted = filteredMatches.filter((m) => !m.remake);
  if (counted.length === 0) return null;
  const byChamp = new Map<number, HighlightedChampion>();
  for (const m of counted) {
    const kda = m.deaths === 0 ? m.kills + m.assists : (m.kills + m.assists) / m.deaths;
    const cur = byChamp.get(m.champion_id);
    if (cur) {
      cur.games += 1;
      cur.wins += m.win ? 1 : 0;
      cur.kdaSum += kda;
    } else {
      byChamp.set(m.champion_id, {
        name: m.champion_name,
        icon: m.champion_icon_url,
        games: 1,
        wins: m.win ? 1 : 0,
        kdaSum: kda,
      });
    }
  }
  const sorted = Array.from(byChamp.values()).sort(
    (a, b) => b.games - a.games || b.wins - a.wins || b.kdaSum - a.kdaSum
  );
  return sorted[0] ?? null;
}

export function resolveAllMasteries(profile: PlayerProfile | null): ChampionMasteryInfo[] {
  if (!profile) return [];
  if (profile.all_masteries && profile.all_masteries.length > 0) {
    return profile.all_masteries;
  }
  return profile.top_masteries ?? [];
}

export function computeMasterySummary(masteries: ChampionMasteryInfo[]): MasterySummaryStats {
  let totalLevel = 0;
  let totalPoints = 0;
  for (const m of masteries) {
    totalLevel += m.champion_level;
    totalPoints += m.champion_points;
  }
  return {
    totalLevel,
    totalPoints,
    championCount: masteries.length,
  };
}

function normalizeSearchText(value: string): string {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .trim();
}

export function filterAndSortMasteries(
  masteries: ChampionMasteryInfo[],
  query: string,
  sortBy: MasterySortKey,
  locale: AppLocale
): ChampionMasteryInfo[] {
  const normalizedQuery = normalizeSearchText(query);
  const filtered = normalizedQuery
    ? masteries.filter((m) => normalizeSearchText(m.champion_name).includes(normalizedQuery))
    : [...masteries];
  const collatorLocale = locale === "pt_BR" ? "pt-BR" : "en-US";

  return filtered.sort((a, b) => {
    const byName = a.champion_name.localeCompare(b.champion_name, collatorLocale);
    if (sortBy === "level") {
      return b.champion_level - a.champion_level || b.champion_points - a.champion_points || byName;
    }
    if (sortBy === "recent") {
      return (
        (b.last_play_time_ms ?? 0) - (a.last_play_time_ms ?? 0) ||
        b.champion_points - a.champion_points ||
        byName
      );
    }
    if (sortBy === "name") {
      return byName || b.champion_points - a.champion_points;
    }
    return b.champion_points - a.champion_points || b.champion_level - a.champion_level || byName;
  });
}

export function computeMasteryProgressPercent(mastery: ChampionMasteryInfo): number | null {
  const untilNext = mastery.champion_points_until_next_level ?? 0;
  const sinceLast = mastery.champion_points_since_last_level ?? 0;
  if (untilNext <= 0) return null;
  const span = sinceLast + untilNext;
  if (span <= 0) return 0;
  return Math.min(100, Math.max(0, Math.round((sinceLast / span) * 100)));
}

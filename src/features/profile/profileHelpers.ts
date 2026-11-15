import { PlayerProfile, RecentMatchInfo } from "../../types";
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

export function filterValidMatches(profile: PlayerProfile | null): RecentMatchInfo[] {
  if (!profile?.recent_matches) return [];
  return profile.recent_matches.filter((m) => {
    const modeLabelLower = m.game_mode_label.toLowerCase();
    return (
      m.queue_id >= 0 &&
      m.game_mode_key.toLowerCase() !== "practicetool" &&
      !modeLabelLower.includes("treino") &&
      !modeLabelLower.includes("practice")
    );
  });
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

export function computeWinLossStats(filteredMatches: RecentMatchInfo[]): WinLossStats {
  const total = filteredMatches.length;
  const wins = filteredMatches.filter((m) => m.win).length;
  const losses = total - wins;
  const winRate = total > 0 ? Math.round((wins / total) * 100) : 0;
  return { total, wins, losses, winRate };
}

export function computeHighlightedChampion(
  filteredMatches: RecentMatchInfo[]
): HighlightedChampion | null {
  if (filteredMatches.length === 0) return null;
  const byChamp = new Map<number, HighlightedChampion>();
  for (const m of filteredMatches) {
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

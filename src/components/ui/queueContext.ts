import type { ChampionBuild, ChampSelectedPayload } from "../../types";

export const QUEUE_ID_ARENA = 1700;
export const ARENA_QUEUE_IDS = new Set<number>([1700, 1710, 1740, 1750]);

/** Returns true for any Riot Arena (`CHERRY` / 3x6) queue ID (`1700`, `1710`, `1740`, `1750`). */
export function isArenaQueueId(queueId?: number | null): boolean {
  return typeof queueId === "number" && ARENA_QUEUE_IDS.has(queueId);
}

/**
 * Returns true when the active Champ Select payload or build represents Arena (`CHERRY` / 3x6).
 * Safely distinguishes Arena (`supports_spells === false`) from ARAM Mayhem (`supports_runes === false`).
 */
export function isArenaBuildContext(
  build?: ChampionBuild | null,
  champSelect?: ChampSelectedPayload | null
): boolean {
  if (champSelect?.queue_mode === "Arena" || isArenaQueueId(champSelect?.queue_id)) {
    return true;
  }
  if (!build) return false;
  return (
    build.supports_spells === false ||
    build.active_tier === "arena" ||
    build.item_blocks.some((block) => block.kind === "prismatic")
  );
}

/**
 * Returns true when the active Champ Select payload or build represents ARAM / ARAM Mayhem
 * (never Arena, even though both Arena and ARAM Mayhem disable runes).
 */
export function isAramBuildContextMode(
  build?: ChampionBuild | null,
  champSelect?: ChampSelectedPayload | null
): boolean {
  if (isArenaBuildContext(build, champSelect)) {
    return false;
  }
  return (
    champSelect?.queue_mode === "Aram" ||
    champSelect?.queue_mode === "AramMayhem" ||
    champSelect?.queue_id === 450 ||
    champSelect?.queue_id === 2400 ||
    build?.supports_runes === false
  );
}

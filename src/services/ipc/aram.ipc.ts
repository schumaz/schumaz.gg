import {
  commands,
  type AramAugmentEntry,
  type AramChampionRank,
  type AramMode,
  type ChampionBuild,
} from "../../types/bindings";

/**
 * Typed IPC adapter for ARAM and ARAM Mayhem tier lists, champion augments, and read-only builds.
 */
export const aramIpc = {
  async getAramTierList(mode: AramMode): Promise<AramChampionRank[]> {
    return commands.getAramTierList(mode);
  },

  async getAramMayhemAugments(championKey: string): Promise<AramAugmentEntry[]> {
    return commands.getAramMayhemAugments(championKey);
  },

  async getAramChampionBuild(
    championId: number,
    mode: AramMode,
    tier: string | null,
    region: string | null,
    patch: string | null
  ): Promise<ChampionBuild> {
    return commands.getAramChampionBuild(championId, mode, tier, region, patch);
  },
};

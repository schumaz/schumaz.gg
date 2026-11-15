import {
  commands,
  type ChampionBuild,
  type RankedTierListResponse,
} from "../../types/bindings";

/**
 * Typed IPC adapter for Ranked Summoner's Rift tier list and read-only build previews.
 */
export const rankedIpc = {
  async getRankedTierList(
    region: string | null,
    tier: string | null,
    patch: string | null
  ): Promise<RankedTierListResponse> {
    return commands.getRankedTierList(region, tier, patch);
  },

  async getRankedChampionBuild(
    championId: number,
    position: string | null,
    tier: string | null,
    region: string | null,
    patch: string | null
  ): Promise<ChampionBuild> {
    return commands.getRankedChampionBuild(championId, position, tier, region, patch);
  },
};

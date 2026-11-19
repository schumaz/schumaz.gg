import {
  commands,
  type ArenaTierListResponse,
  type ChampionBuild,
} from "../../types/bindings";

/**
 * Typed IPC adapter for Arena (`CHERRY`) tier list and read-only build previews.
 */
export const arenaIpc = {
  async getArenaTierList(patch: string | null): Promise<ArenaTierListResponse> {
    return commands.getArenaTierList(patch);
  },

  async getArenaChampionBuild(
    championId: number,
    patch: string | null
  ): Promise<ChampionBuild> {
    return commands.getArenaChampionBuild(championId, patch);
  },
};

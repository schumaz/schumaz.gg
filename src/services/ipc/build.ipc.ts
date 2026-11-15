import {
  commands,
  events,
  type ChampionBuild,
  type RecommendedBuild,
} from "../../types/bindings";
import type { UnlistenFn } from "./lcu.ipc";

/**
 * Typed IPC adapter for active Champ Select builds, filter updates, and recommended build events.
 */
export const buildIpc = {
  async getRecommendedBuildForChampion(
    championId: number,
    queueId: number | null
  ): Promise<ChampionBuild> {
    return commands.getRecommendedBuildForChampion(championId, queueId);
  },

  async updateBuildFilters(
    championId: number,
    queueId: number | null,
    position: string | null,
    tier: string | null,
    region: string | null,
    patch: string | null
  ): Promise<ChampionBuild> {
    return commands.updateBuildFilters(championId, queueId, position, tier, region, patch);
  },

  async onRecommendedBuild(handler: (build: RecommendedBuild) => void): Promise<UnlistenFn> {
    return events.recommendedBuild.listen((event) => handler(event.payload));
  },
};

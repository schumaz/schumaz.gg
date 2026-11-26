import {
  commands,
  events,
  type CachedProfileResult,
  type MatchHistoryPage,
  type MatchScoreboardDetail,
  type PlayerProfile,
} from "../../types/bindings";
import type { UnlistenFn } from "./lcu.ipc";

/**
 * Typed IPC adapter for cached/live player profile fetching, paginated match history, and post-match refresh events.
 */
export const profileIpc = {
  async getCachedPlayerProfile(summonerId: number | null): Promise<CachedProfileResult | null> {
    return commands.getCachedPlayerProfile(summonerId);
  },

  async getPlayerProfile(forceRefresh: boolean | null): Promise<PlayerProfile | null> {
    return commands.getPlayerProfile(forceRefresh);
  },

  async getPlayerMatchHistoryPage(
    begIndex: number,
    count: number | null = 10
  ): Promise<MatchHistoryPage> {
    return commands.getPlayerMatchHistoryPage(begIndex, count);
  },

  async getMatchScoreboardDetail(gameId: number): Promise<MatchScoreboardDetail> {
    return commands.getMatchScoreboardDetail(gameId);
  },

  async onProfileRefreshRequested(handler: () => void): Promise<UnlistenFn> {
    return events.profileRefreshRequested.listen(() => handler());
  },
};

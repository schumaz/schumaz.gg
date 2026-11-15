import {
  commands,
  events,
  type CachedProfileResult,
  type PlayerProfile,
} from "../../types/bindings";
import type { UnlistenFn } from "./lcu.ipc";

/**
 * Typed IPC adapter for cached/live player profile fetching and post-match refresh events.
 */
export const profileIpc = {
  async getCachedPlayerProfile(summonerId: number | null): Promise<CachedProfileResult | null> {
    return commands.getCachedPlayerProfile(summonerId);
  },

  async getPlayerProfile(forceRefresh: boolean | null): Promise<PlayerProfile | null> {
    return commands.getPlayerProfile(forceRefresh);
  },

  async onProfileRefreshRequested(handler: () => void): Promise<UnlistenFn> {
    return events.profileRefreshRequested.listen(() => handler());
  },
};

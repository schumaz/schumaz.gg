import {
  commands,
  type AramAugmentEntry,
  type AramChampionRank,
  type AramMode,
} from "../../types/bindings";

/**
 * Typed IPC adapter for ARAM and ARAM Mayhem tier lists and champion augments.
 */
export const aramIpc = {
  async getAramTierList(mode: AramMode): Promise<AramChampionRank[]> {
    return commands.getAramTierList(mode);
  },

  async getAramMayhemAugments(championKey: string): Promise<AramAugmentEntry[]> {
    return commands.getAramMayhemAugments(championKey);
  },
};

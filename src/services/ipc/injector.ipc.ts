import { commands, events, type BuildInjected } from "../../types/bindings";
import type { UnlistenFn } from "./lcu.ipc";

/**
 * Typed IPC adapter for LCU injections (summoner spells, rune pages, and item sets).
 */
export const injectorIpc = {
  async injectSummonerSpells(
    spell1Id: number,
    spell2Id: number,
    championId: number | null
  ): Promise<null> {
    return commands.injectSummonerSpells(spell1Id, spell2Id, championId);
  },

  async injectRunePage(
    championId: number,
    championName: string,
    pageIndex: number,
    primaryStyleId: number,
    subStyleId: number,
    selectedPerkIds: number[]
  ): Promise<null> {
    return commands.injectRunePage(
      championId,
      championName,
      pageIndex,
      primaryStyleId,
      subStyleId,
      selectedPerkIds
    );
  },

  async injectItemBuild(
    championId: number,
    queueId: number | null,
    buildIndex: number | null
  ): Promise<null> {
    return commands.injectItemBuild(championId, queueId, buildIndex);
  },

  async onBuildInjected(handler: (report: BuildInjected) => void): Promise<UnlistenFn> {
    return events.buildInjected.listen((event) => handler(event.payload));
  },
};

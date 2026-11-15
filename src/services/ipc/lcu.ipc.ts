import {
  commands,
  events,
  type ChampSelectedPayload,
  type ChampSelectReconciledPayload,
  type GameflowPhaseChanged,
  type LcuConnected,
  type SummonerInfo,
} from "../../types/bindings";

export type UnlistenFn = () => void;

/**
 * Typed IPC adapter for League Client (LCU) status, Champ Select, and Gameflow events.
 */
export const lcuIpc = {
  async getSummonerStatus(): Promise<SummonerInfo | null> {
    return commands.getSummonerStatus();
  },

  async syncDisplayedChampion(championId: number | null): Promise<void> {
    return commands.syncDisplayedChampion(championId);
  },

  async onLcuConnected(handler: (summoner: LcuConnected) => void): Promise<UnlistenFn> {
    return events.lcuConnected.listen((event) => handler(event.payload));
  },

  async onLcuDisconnected(handler: () => void): Promise<UnlistenFn> {
    return events.lcuDisconnected.listen(() => handler());
  },

  async onChampSelected(handler: (payload: ChampSelectedPayload) => void): Promise<UnlistenFn> {
    return events.champSelected.listen((event) => handler(event.payload));
  },

  async onChampSelectEnded(handler: () => void): Promise<UnlistenFn> {
    return events.champSelectEnded.listen(() => handler());
  },

  async onChampSelectReconciled(
    handler: (payload: ChampSelectReconciledPayload) => void
  ): Promise<UnlistenFn> {
    return events.champSelectReconciled.listen((event) => handler(event.payload));
  },

  async onGameflowPhaseChanged(
    handler: (phase: GameflowPhaseChanged) => void
  ): Promise<UnlistenFn> {
    return events.gameflowPhaseChanged.listen((event) => handler(event.payload));
  },

  async onMatchEnded(handler: () => void): Promise<UnlistenFn> {
    return events.matchEnded.listen(() => handler());
  },
};

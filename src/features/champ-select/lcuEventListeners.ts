import { Dispatch, MutableRefObject, SetStateAction } from "react";
import {
  ActiveChampSelectSnapshot,
  ChampionBuild,
  ChampSelectedPayload,
  SummonerInfo,
} from "../../types";
import { buildIpc, lcuIpc, type UnlistenFn } from "../../services/ipc";
import { fetchRecommendedPayloadBuild } from "./lcuBuildActions";
import { hydrateActiveChampSelectSnapshot } from "./lcuSnapshotHydration";

export function isInGamePhase(phase: string): boolean {
  return phase === "GameStart" || phase === "InProgress" || phase === "Reconnect";
}

export interface LcuListenersContext {
  activeChampIdRef: MutableRefObject<number | null>;
  buildRef: MutableRefObject<ChampionBuild | null>;
  latestFetchSeqRef: MutableRefObject<number>;
  hasAutoNavigatedRef: MutableRefObject<boolean>;
  gameflowPhaseRef: MutableRefObject<string>;
  onChampionPulledRef: MutableRefObject<(() => void) | undefined>;
  onMatchEndedRef: MutableRefObject<(() => void) | undefined>;
  setSummoner: Dispatch<SetStateAction<SummonerInfo | null>>;
  setChampSelect: Dispatch<SetStateAction<ChampSelectedPayload | null>>;
  setBuild: Dispatch<SetStateAction<ChampionBuild | null>>;
  setGameflowPhase: Dispatch<SetStateAction<string>>;
  setSpellsInverted: Dispatch<SetStateAction<boolean>>;
  setSelectedSecondarySpellId: Dispatch<SetStateAction<number | null>>;
  setSelectedRunePageIndex: Dispatch<SetStateAction<number>>;
  setSelectedBuildIndex: Dispatch<SetStateAction<number>>;
  setErrorMsg: Dispatch<SetStateAction<string | null>>;
}

export interface LcuEventSeqSnapshot {
  sessionSeq: number;
  phaseSeq: number;
}

export interface LcuSubscriptionHandle {
  ready: Promise<void>;
  captureEventSeq: () => LcuEventSeqSnapshot;
  hydrateSnapshot: (
    snapshot: ActiveChampSelectSnapshot | null,
    before: LcuEventSeqSnapshot,
    isActive: () => boolean
  ) => Promise<void>;
  unlisten: () => void;
}

export function subscribeToLcuEvents(ctx: LcuListenersContext): LcuSubscriptionHandle {
  const unlisteners: Array<UnlistenFn | undefined> = [];
  let disposed = false;
  let liveSessionEventSeq = 0;
  let livePhaseEventSeq = 0;

  const resetBuildSelections = () => {
    ctx.setSelectedSecondarySpellId(null);
    ctx.setSelectedRunePageIndex(0);
    ctx.setSelectedBuildIndex(0);
  };

  const clearSessionState = () => {
    ctx.activeChampIdRef.current = null;
    ctx.buildRef.current = null;
    ctx.latestFetchSeqRef.current += 1;
    ctx.hasAutoNavigatedRef.current = false;
    ctx.setChampSelect(null);
    ctx.setBuild(null);
    ctx.setSpellsInverted(false);
    resetBuildSelections();
  };

  const notifyChampionPulledOnce = () => {
    if (!ctx.hasAutoNavigatedRef.current) {
      ctx.hasAutoNavigatedRef.current = true;
      ctx.onChampionPulledRef.current?.();
    }
  };

  const fetchBuildForPayload = async (payload: ChampSelectedPayload, fetchSeq: number) => {
    try {
      const recommended = await fetchRecommendedPayloadBuild(payload);
      if (
        recommended &&
        ctx.latestFetchSeqRef.current === fetchSeq &&
        ctx.activeChampIdRef.current === payload.champion_id
      ) {
        ctx.buildRef.current = recommended;
        ctx.setBuild((prev) => {
          if (prev?.champion_id !== recommended.champion_id) resetBuildSelections();
          return recommended;
        });
        notifyChampionPulledOnce();
      }
    } catch {
      // Ignore background fetch error; ws_listener will emit recommended-build
    }
  };

  const ready = Promise.all([
    lcuIpc.onLcuConnected((payload) => {
      ctx.setSummoner(payload);
      ctx.setErrorMsg(null);
    }),
    lcuIpc.onLcuDisconnected(() => {
      livePhaseEventSeq += 1;
      liveSessionEventSeq += 1;
      ctx.gameflowPhaseRef.current = "";
      ctx.setGameflowPhase("");
      ctx.setSummoner(null);
      clearSessionState();
    }),
    lcuIpc.onChampSelected(async (payload) => {
      liveSessionEventSeq += 1;
      ctx.activeChampIdRef.current = payload.champion_id;
      const fetchSeq = ++ctx.latestFetchSeqRef.current;
      ctx.setChampSelect((prev) => {
        if (prev?.champion_id !== payload.champion_id) {
          ctx.setSpellsInverted(false);
          resetBuildSelections();
        }
        return payload;
      });
      await fetchBuildForPayload(payload, fetchSeq);
    }),
    lcuIpc.onChampSelectEnded(() => {
      liveSessionEventSeq += 1;
      ctx.activeChampIdRef.current = null;
      ctx.latestFetchSeqRef.current += 1;
      if (!isInGamePhase(ctx.gameflowPhaseRef.current)) clearSessionState();
    }),
    buildIpc.onRecommendedBuild((payload) => {
      if (
        ctx.activeChampIdRef.current !== null &&
        ctx.activeChampIdRef.current !== payload.champion_id
      ) {
        return;
      }
      ctx.latestFetchSeqRef.current += 1;
      ctx.buildRef.current = payload;
      ctx.setBuild((prev) => {
        if (prev?.champion_id !== payload.champion_id) resetBuildSelections();
        return payload;
      });
      notifyChampionPulledOnce();
    }),
    lcuIpc.onChampSelectReconciled(async (payload) => {
      liveSessionEventSeq += 1;
      const { champ_selected, build: reconciledBuild } = payload;
      const currentDisplayedChampId =
        ctx.activeChampIdRef.current ?? ctx.buildRef.current?.champion_id ?? null;
      const champDiverged = currentDisplayedChampId !== champ_selected.champion_id;
      ctx.activeChampIdRef.current = champ_selected.champion_id;

      if (champDiverged) {
        const fetchSeq = ++ctx.latestFetchSeqRef.current;
        ctx.setSpellsInverted(false);
        resetBuildSelections();
        ctx.setChampSelect(champ_selected);
        if (reconciledBuild && reconciledBuild.champion_id === champ_selected.champion_id) {
          ctx.buildRef.current = reconciledBuild;
          ctx.setBuild(reconciledBuild);
          notifyChampionPulledOnce();
        } else {
          await fetchBuildForPayload(champ_selected, fetchSeq);
        }
        return;
      }

      ctx.setChampSelect(champ_selected);
      if (
        reconciledBuild &&
        reconciledBuild.champion_id === champ_selected.champion_id &&
        (!ctx.buildRef.current || ctx.buildRef.current.champion_id !== champ_selected.champion_id)
      ) {
        ctx.buildRef.current = reconciledBuild;
        ctx.setBuild(reconciledBuild);
        notifyChampionPulledOnce();
      }
    }),
    lcuIpc.onGameflowPhaseChanged((phase) => {
      livePhaseEventSeq += 1;
      ctx.gameflowPhaseRef.current = phase;
      ctx.setGameflowPhase(phase);
    }),
    lcuIpc.onMatchEnded(() => {
      liveSessionEventSeq += 1;
      clearSessionState();
      ctx.onMatchEndedRef.current?.();
    }),
  ])
    .then((fns) => {
      if (disposed) {
        for (const fn of fns) fn?.();
        return;
      }
      unlisteners.push(...fns);
    })
    .catch((e) => {
      console.warn("Tauri event listener setup skipped or running in test mode:", e);
    });

  const captureEventSeq = (): LcuEventSeqSnapshot => ({
    sessionSeq: liveSessionEventSeq,
    phaseSeq: livePhaseEventSeq,
  });

  return {
    ready,
    captureEventSeq,
    hydrateSnapshot: (snapshot, before, isActive) =>
      hydrateActiveChampSelectSnapshot(snapshot, {
        ctx,
        before,
        getCurrentEventSeq: captureEventSeq,
        isActive,
        resetBuildSelections,
        notifyChampionPulledOnce,
        fetchBuildForPayload,
      }),
    unlisten: () => {
      disposed = true;
      for (const fn of unlisteners) fn?.();
    },
  };
}

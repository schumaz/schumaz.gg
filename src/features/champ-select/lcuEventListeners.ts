import { Dispatch, MutableRefObject, SetStateAction } from "react";
import {
  ActiveChampSelectSnapshot,
  ChampionBuild,
  ChampSelectedPayload,
  SummonerInfo,
} from "../../types";
import { buildIpc, lcuIpc, type UnlistenFn } from "../../services/ipc";
import {
  fetchRecommendedPayloadBuild,
  isInGamePhase,
  isSpellsLockedPhase,
} from "./lcuBuildActions";
import {
  applyChampSelectReconciledPayload,
  hydrateActiveChampSelectSnapshot,
} from "./lcuSnapshotHydration";

export { isInGamePhase, isSpellsLockedPhase };

export interface LcuListenersContext {
  activeChampIdRef: MutableRefObject<number | null>;
  buildRef: MutableRefObject<ChampionBuild | null>;
  latestFetchSeqRef: MutableRefObject<number>;
  activeLoadingSeqRef: MutableRefObject<number>;
  lobbySlotSeqRef: MutableRefObject<number>;
  expectedLobbySlotRef: MutableRefObject<number | null>;
  hasAutoNavigatedRef: MutableRefObject<boolean>;
  gameflowPhaseRef: MutableRefObject<string>;
  onChampionPulledRef: MutableRefObject<(() => void) | undefined>;
  onMatchEndedRef: MutableRefObject<(() => void) | undefined>;
  setSummoner: Dispatch<SetStateAction<SummonerInfo | null>>;
  setChampSelect: Dispatch<SetStateAction<ChampSelectedPayload | null>>;
  setBuild: Dispatch<SetStateAction<ChampionBuild | null>>;
  setGameflowPhase: Dispatch<SetStateAction<string>>;
  setSpellsInverted: Dispatch<SetStateAction<boolean>>;
  setSelectedSpellPairIndex: Dispatch<SetStateAction<number>>;
  setSelectedRunePageIndex: Dispatch<SetStateAction<number>>;
  setSelectedBuildIndex: Dispatch<SetStateAction<number>>;
  setLoading: Dispatch<SetStateAction<boolean>>;
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
    ctx.setSelectedSpellPairIndex(0);
    ctx.setSelectedRunePageIndex(0);
    ctx.setSelectedBuildIndex(0);
  };

  const cancelActiveLoading = () => {
    ctx.activeLoadingSeqRef.current += 1;
    ctx.setLoading(false);
  };

  const invalidateLobbySlotSeq = () => {
    ctx.lobbySlotSeqRef.current += 1;
    ctx.expectedLobbySlotRef.current = null;
  };

  const clearSessionState = () => {
    ctx.activeChampIdRef.current = null;
    ctx.buildRef.current = null;
    ctx.latestFetchSeqRef.current += 1;
    ctx.hasAutoNavigatedRef.current = false;
    cancelActiveLoading();
    invalidateLobbySlotSeq();
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
      const prevChampId = ctx.activeChampIdRef.current ?? ctx.buildRef.current?.champion_id ?? null;
      ctx.activeChampIdRef.current = payload.champion_id;
      const fetchSeq = ++ctx.latestFetchSeqRef.current;
      if (
        !payload.is_lobby_preselect ||
        (ctx.expectedLobbySlotRef.current !== null &&
          payload.active_slot_index !== ctx.expectedLobbySlotRef.current)
      ) {
        invalidateLobbySlotSeq();
      }
      if (prevChampId !== payload.champion_id) {
        cancelActiveLoading();
        ctx.setSpellsInverted(false);
        resetBuildSelections();
      }
      ctx.setChampSelect(payload);
      if (payload.is_lobby_preselect) return;
      await fetchBuildForPayload(payload, fetchSeq);
    }),
    lcuIpc.onChampSelectEnded(() => {
      liveSessionEventSeq += 1;
      ctx.activeChampIdRef.current = null;
      ctx.latestFetchSeqRef.current += 1;
      invalidateLobbySlotSeq();
      cancelActiveLoading();
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
      cancelActiveLoading();
      ctx.buildRef.current = payload;
      ctx.setBuild((prev) => {
        if (prev?.champion_id !== payload.champion_id) resetBuildSelections();
        return payload;
      });
      notifyChampionPulledOnce();
    }),
    lcuIpc.onChampSelectReconciled(async (payload) => {
      liveSessionEventSeq += 1;
      await applyChampSelectReconciledPayload(payload, {
        ctx,
        cancelActiveLoading,
        invalidateLobbySlotSeq,
        resetBuildSelections,
        notifyChampionPulledOnce,
        fetchBuildForPayload,
      });
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

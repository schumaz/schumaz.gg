import { Dispatch, MutableRefObject, SetStateAction } from "react";
import {
  ChampionBuild,
  ChampSelectedPayload,
  SummonerInfo,
} from "../../types";
import { buildIpc, lcuIpc, type UnlistenFn } from "../../services/ipc";
import { fetchRecommendedPayloadBuild } from "./lcuBuildActions";

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

export function subscribeToLcuEvents(ctx: LcuListenersContext): () => void {
  const unlisteners: Array<UnlistenFn | undefined> = [];

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

  const setup = async () => {
    try {
      unlisteners.push(
        await lcuIpc.onLcuConnected((payload) => {
          ctx.setSummoner(payload);
          ctx.setErrorMsg(null);
        })
      );
      unlisteners.push(
        await lcuIpc.onLcuDisconnected(() => {
          ctx.gameflowPhaseRef.current = "";
          ctx.setGameflowPhase("");
          ctx.setSummoner(null);
          clearSessionState();
        })
      );
      unlisteners.push(
        await lcuIpc.onChampSelected(async (payload) => {
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
        })
      );
      unlisteners.push(
        await lcuIpc.onChampSelectEnded(() => {
          ctx.activeChampIdRef.current = null;
          ctx.latestFetchSeqRef.current += 1;
          if (!isInGamePhase(ctx.gameflowPhaseRef.current)) clearSessionState();
        })
      );
      unlisteners.push(
        await buildIpc.onRecommendedBuild((payload) => {
          if (
            ctx.activeChampIdRef.current !== null &&
            ctx.activeChampIdRef.current !== payload.champion_id
          ) {
            return;
          }
          ctx.buildRef.current = payload;
          ctx.setBuild((prev) => {
            if (prev?.champion_id !== payload.champion_id) resetBuildSelections();
            return payload;
          });
          notifyChampionPulledOnce();
        })
      );
      unlisteners.push(
        await lcuIpc.onChampSelectReconciled(async (payload) => {
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
        })
      );
      unlisteners.push(
        await lcuIpc.onGameflowPhaseChanged((phase) => {
          ctx.gameflowPhaseRef.current = phase;
          ctx.setGameflowPhase(phase);
        })
      );
      unlisteners.push(
        await lcuIpc.onMatchEnded(() => {
          clearSessionState();
          ctx.onMatchEndedRef.current?.();
        })
      );
    } catch (e) {
      console.warn("Tauri event listener setup skipped or running in test mode:", e);
    }
  };

  setup();

  return () => {
    for (const unlisten of unlisteners) {
      unlisten?.();
    }
  };
}

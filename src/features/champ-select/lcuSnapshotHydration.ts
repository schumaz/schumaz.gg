import { ActiveChampSelectSnapshot, ChampSelectedPayload } from "../../types";
import type { LcuEventSeqSnapshot, LcuListenersContext } from "./lcuEventListeners";

export interface SnapshotHydrationDeps {
  ctx: LcuListenersContext;
  before: LcuEventSeqSnapshot;
  getCurrentEventSeq: () => LcuEventSeqSnapshot;
  isActive: () => boolean;
  resetBuildSelections: () => void;
  notifyChampionPulledOnce: () => void;
  fetchBuildForPayload: (payload: ChampSelectedPayload, fetchSeq: number) => Promise<void>;
}

/**
 * Hydrates initial LCU gameflow phase, Champ Select payload, and cached build from the
 * consolidated backend bootstrap snapshot without duplicating fetches or resurrecting ended sessions.
 */
export async function hydrateActiveChampSelectSnapshot(
  snapshot: ActiveChampSelectSnapshot | null,
  deps: SnapshotHydrationDeps
): Promise<void> {
  const { ctx, before, getCurrentEventSeq, isActive } = deps;
  if (!snapshot || !isActive()) return;

  const currentSeq = getCurrentEventSeq();
  if (
    snapshot.gameflow_phase &&
    (currentSeq.phaseSeq === before.phaseSeq || !ctx.gameflowPhaseRef.current)
  ) {
    ctx.gameflowPhaseRef.current = snapshot.gameflow_phase;
    ctx.setGameflowPhase(snapshot.gameflow_phase);
  }

  const payload = snapshot.champ_selected;
  if (!payload) return;

  const liveSessionChanged = currentSeq.sessionSeq !== before.sessionSeq;
  if (liveSessionChanged && ctx.activeChampIdRef.current === null) {
    return;
  }

  const sameLiveChamp = liveSessionChanged && ctx.activeChampIdRef.current === payload.champion_id;
  const prevChampId = ctx.activeChampIdRef.current ?? ctx.buildRef.current?.champion_id ?? null;
  ctx.activeChampIdRef.current = payload.champion_id;
  ctx.setChampSelect((prev) => {
    if (prev && prev.champion_id !== payload.champion_id) {
      ctx.setSpellsInverted(false);
      deps.resetBuildSelections();
    }
    return payload;
  });

  if (snapshot.build && snapshot.build.champion_id === payload.champion_id) {
    if (ctx.buildRef.current?.champion_id !== snapshot.build.champion_id) {
      if (prevChampId !== null && prevChampId !== snapshot.build.champion_id) {
        ctx.setSpellsInverted(false);
      }
      deps.resetBuildSelections();
    }
    ctx.latestFetchSeqRef.current += 1;
    ctx.buildRef.current = snapshot.build;
    ctx.setBuild(snapshot.build);
    deps.notifyChampionPulledOnce();
    return;
  }

  if (sameLiveChamp || ctx.buildRef.current?.champion_id === payload.champion_id) {
    return;
  }

  const fetchSeq = ++ctx.latestFetchSeqRef.current;
  await deps.fetchBuildForPayload(payload, fetchSeq);
}

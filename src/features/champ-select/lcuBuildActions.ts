import {
  ChampionBuild,
  ChampSelectedPayload,
  FlashKeyPreference,
} from "../../types";
import { buildIpc, injectorIpc } from "../../services/ipc";
import { computeOrderedSpells } from "../../utils/spells";

export function isAramBuildContext(
  champSelect: ChampSelectedPayload | null,
  currentBuild: ChampionBuild
): { champId: number; qId: number; isAram: boolean } {
  const champId = champSelect?.champion_id || currentBuild.champion_id;
  const qId = champSelect?.queue_id || (currentBuild.supports_runes === false ? 2400 : 420);
  const isAram =
    champSelect?.queue_mode === "Aram" ||
    champSelect?.queue_mode === "AramMayhem" ||
    qId === 450 ||
    qId === 2400 ||
    currentBuild.supports_runes === false;
  return { champId, qId, isAram };
}

export async function fetchFilteredChampSelectBuild(
  currentBuild: ChampionBuild,
  champSelect: ChampSelectedPayload | null,
  filterType?: "position" | "tier" | "region" | "patch",
  value?: string
): Promise<ChampionBuild> {
  const { champId, qId, isAram } = isAramBuildContext(champSelect, currentBuild);
  const patchArg =
    filterType === "patch" && value !== undefined
      ? value.trim() || null
      : currentBuild.patch_fallback_applied
      ? null
      : currentBuild.active_patch || null;

  return buildIpc.updateBuildFilters(
    champId,
    qId,
    isAram
      ? null
      : filterType === "position" && value !== undefined
      ? value
      : currentBuild.active_position || null,
    filterType === "tier" && value !== undefined ? value : currentBuild.active_tier || null,
    filterType === "region" && value !== undefined ? value : currentBuild.active_region || null,
    patchArg
  );
}

export async function fetchRecommendedPayloadBuild(
  payload: ChampSelectedPayload
): Promise<ChampionBuild> {
  const qId = payload.queue_id || 420;
  const isAram =
    payload.queue_mode === "Aram" ||
    payload.queue_mode === "AramMayhem" ||
    qId === 450 ||
    qId === 2400;

  return !isAram && payload.assigned_position
    ? buildIpc.updateBuildFilters(
        payload.champion_id,
        qId,
        payload.assigned_position,
        null,
        null,
        null
      )
    : buildIpc.getRecommendedBuildForChampion(payload.champion_id, qId);
}

export async function injectSecondarySpellAction(
  build: ChampionBuild | null,
  spellId: number,
  flashKey: FlashKeyPreference,
  spellsInverted: boolean
) {
  const reordered = computeOrderedSpells(
    build?.spells,
    spellId,
    build?.spell_choices,
    flashKey,
    spellsInverted
  );
  if (reordered.length >= 2) {
    try {
      await injectorIpc.injectSummonerSpells(
        reordered[0].id,
        reordered[1].id,
        build?.champion_id ?? null,
        spellId
      );
    } catch (e) {
      console.warn("Failed to inject secondary spell change:", e);
    }
  }
}

export async function injectRunePageAction(build: ChampionBuild, pageIndex: number) {
  if (!build.rune_pages || pageIndex >= build.rune_pages.length) return;
  const page = build.rune_pages[pageIndex];
  try {
    await injectorIpc.injectRunePage(
      build.champion_id,
      build.champion_name,
      pageIndex,
      page.primary_style_id,
      page.sub_style_id,
      page.selected_perk_ids
    );
  } catch (e) {
    console.warn("Failed to inject chosen rune page:", e);
  }
}

export async function injectItemBuildAction(
  build: ChampionBuild,
  champSelect: ChampSelectedPayload | null,
  index: number
) {
  const activeQueueId =
    champSelect?.queue_id ?? (build.supports_runes === false ? 2400 : null);
  try {
    await injectorIpc.injectItemBuild(build.champion_id, activeQueueId, index);
  } catch (e) {
    console.warn("Failed to inject chosen unified build:", e);
  }
}

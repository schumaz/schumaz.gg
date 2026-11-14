import { useState, useEffect, useRef } from "react";
import {
  ChampionBuild,
  ChampSelectedPayload,
  FlashKeyPreference,
  SummonerInfo,
  commands,
  events,
} from "../types";
import { computeOrderedSpells } from "../utils/spells";

export function useLcuEvents(flashKey: FlashKeyPreference) {
  const [summoner, setSummoner] = useState<SummonerInfo | null>(null);
  const [champSelect, setChampSelect] = useState<ChampSelectedPayload | null>(null);
  const [build, setBuild] = useState<ChampionBuild | null>(null);
  const [spellsInverted, setSpellsInverted] = useState(false);
  const [selectedSecondarySpellId, setSelectedSecondarySpellId] = useState<number | null>(null);
  const [selectedRunePageIndex, setSelectedRunePageIndex] = useState<number>(0);
  const [selectedBuildIndex, setSelectedBuildIndex] = useState<number>(0);
  const [loading, setLoading] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const activeChampIdRef = useRef<number | null>(null);
  const latestFetchSeqRef = useRef(0);

  const handleFilterChange = async (
    filterType: "position" | "tier" | "region" | "patch",
    value: string
  ) => {
    if (!build) return;
    const champId = champSelect?.champion_id || build.champion_id;
    const qId = champSelect?.queue_id || (build.supports_runes === false ? 2400 : 420);
    const isAram =
      champSelect?.queue_mode === "Aram" ||
      champSelect?.queue_mode === "AramMayhem" ||
      qId === 450 ||
      qId === 2400 ||
      build.supports_runes === false;

    const fetchSeq = ++latestFetchSeqRef.current;
    setLoading(true);
    setErrorMsg(null);
    try {
      const updatedBuild = await commands.updateBuildFilters(
        champId,
        qId,
        isAram ? null : filterType === "position" ? value : (build.active_position || null),
        filterType === "tier" ? value : (build.active_tier || null),
        filterType === "region" ? value : (build.active_region || null),
        filterType === "patch" ? value : (build.active_patch || null)
      );
      if (updatedBuild && latestFetchSeqRef.current === fetchSeq) {
        setBuild(updatedBuild);
      }
    } catch (err: unknown) {
      console.error("Falha ao atualizar filtros da build:", err);
      if (latestFetchSeqRef.current === fetchSeq) {
        setErrorMsg(
          typeof err === "string"
            ? err
            : "Erro ao carregar build com os filtros selecionados."
        );
      }
    } finally {
      if (latestFetchSeqRef.current === fetchSeq) {
        setLoading(false);
      }
    }
  };

  useEffect(() => {
    // Initial check for summoner status
    commands
      .getSummonerStatus()
      .then((data) => {
        if (data) setSummoner(data);
      })
      .catch(() => {});

    // Listen to real-time events emitted from Tauri backend
    let unlistenConnected: (() => void) | undefined;
    let unlistenDisconnected: (() => void) | undefined;
    let unlistenChampSelected: (() => void) | undefined;
    let unlistenChampSelectEnded: (() => void) | undefined;
    let unlistenBuild: (() => void) | undefined;

    const setupListeners = async () => {
      try {
        unlistenConnected = await events.lcuConnected.listen((event) => {
          setSummoner(event.payload);
          setErrorMsg(null);
        });

        unlistenDisconnected = await events.lcuDisconnected.listen(() => {
          activeChampIdRef.current = null;
          latestFetchSeqRef.current += 1;
          setSummoner(null);
          setChampSelect(null);
          setBuild(null);
        });

        unlistenChampSelected = await events.champSelected.listen(async (event) => {
          activeChampIdRef.current = event.payload.champion_id;
          const fetchSeq = ++latestFetchSeqRef.current;

          setChampSelect((prev) => {
            if (prev?.champion_id !== event.payload.champion_id) {
              setSpellsInverted(false);
              setSelectedSecondarySpellId(null);
              setSelectedRunePageIndex(0);
              setSelectedBuildIndex(0);
            }
            return event.payload;
          });

          // Fetch recommended build for selected champion if not already received
          try {
            const qId = event.payload.queue_id || 420;
            const isAram =
              event.payload.queue_mode === "Aram" ||
              event.payload.queue_mode === "AramMayhem" ||
              qId === 450 ||
              qId === 2400;
            if (!isAram && event.payload.assigned_position) {
              const recommended = await commands.updateBuildFilters(
                event.payload.champion_id,
                qId,
                event.payload.assigned_position,
                null,
                null,
                null
              );
              if (
                recommended &&
                latestFetchSeqRef.current === fetchSeq &&
                activeChampIdRef.current === event.payload.champion_id
              ) {
                setBuild((prev) => {
                  if (prev?.champion_id !== recommended.champion_id) {
                    setSelectedSecondarySpellId(null);
                    setSelectedRunePageIndex(0);
                    setSelectedBuildIndex(0);
                  }
                  return recommended;
                });
              }
            } else {
              const recommended = await commands.getRecommendedBuildForChampion(
                event.payload.champion_id,
                qId
              );
              if (
                recommended &&
                latestFetchSeqRef.current === fetchSeq &&
                activeChampIdRef.current === event.payload.champion_id
              ) {
                setBuild((prev) => {
                  if (prev?.champion_id !== recommended.champion_id) {
                    setSelectedSecondarySpellId(null);
                    setSelectedRunePageIndex(0);
                    setSelectedBuildIndex(0);
                  }
                  return recommended;
                });
              }
            }
          } catch {
            // Ignore background fetch error, ws_listener will emit recommended-build
          }
        });

        unlistenChampSelectEnded = await events.champSelectEnded.listen(() => {
          activeChampIdRef.current = null;
          latestFetchSeqRef.current += 1;
          setChampSelect(null);
          setBuild(null);
          setSpellsInverted(false);
          setSelectedSecondarySpellId(null);
          setSelectedRunePageIndex(0);
          setSelectedBuildIndex(0);
        });

        unlistenBuild = await events.recommendedBuild.listen((event) => {
          if (
            activeChampIdRef.current !== null &&
            activeChampIdRef.current !== event.payload.champion_id
          ) {
            return;
          }
          setBuild((prev) => {
            if (prev?.champion_id !== event.payload.champion_id) {
              setSelectedSecondarySpellId(null);
              setSelectedRunePageIndex(0);
              setSelectedBuildIndex(0);
            }
            return event.payload;
          });
        });
      } catch (e) {
        console.warn("Tauri event listener setup skipped or running in test mode:", e);
      }
    };

    setupListeners();

    return () => {
      if (unlistenConnected) unlistenConnected();
      if (unlistenDisconnected) unlistenDisconnected();
      if (unlistenChampSelected) unlistenChampSelected();
      if (unlistenChampSelectEnded) unlistenChampSelectEnded();
      if (unlistenBuild) unlistenBuild();
    };
  }, []);

  const orderedSpells = computeOrderedSpells(
    build?.spells,
    selectedSecondarySpellId,
    build?.spell_choices,
    flashKey,
    spellsInverted
  );

  const handleSelectSecondarySpell = async (spellId: number) => {
    setSelectedSecondarySpellId(spellId);
    const reordered = computeOrderedSpells(
      build?.spells,
      spellId,
      build?.spell_choices,
      flashKey,
      spellsInverted
    );
    if (reordered.length >= 2) {
      try {
        await commands.injectSummonerSpells(
          reordered[0].id,
          reordered[1].id,
          build?.champion_id ?? null,
          spellId
        );
      } catch (e) {
        console.warn("Failed to inject secondary spell change:", e);
      }
    }
  };

  const handleSelectRunePage = async (pageIndex: number) => {
    if (!build?.rune_pages || pageIndex >= build.rune_pages.length) return;
    setSelectedRunePageIndex(pageIndex);
    const page = build.rune_pages[pageIndex];
    try {
      await commands.injectRunePage(
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
  };

  const handleSelectBuild = async (index: number) => {
    if (!build?.build_options || index >= build.build_options.length) return;
    setSelectedBuildIndex(index);
    const activeQueueId =
      champSelect?.queue_id ?? (build.supports_runes === false ? 2400 : null);
    try {
      await commands.injectItemBuild(build.champion_id, activeQueueId, index, index, index);
    } catch (e) {
      console.warn("Failed to inject chosen unified build:", e);
    }
  };

  return {
    summoner,
    champSelect,
    build,
    spellsInverted,
    setSpellsInverted,
    selectedSecondarySpellId,
    selectedRunePageIndex,
    selectedBuildIndex,
    loading,
    errorMsg,
    orderedSpells,
    handleFilterChange,
    handleSelectSecondarySpell,
    handleSelectRunePage,
    handleSelectBuild,
  };
}

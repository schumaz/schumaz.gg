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

export function isInGamePhase(phase: string): boolean {
  return phase === "GameStart" || phase === "InProgress" || phase === "Reconnect";
}

export interface UseLcuEventsOptions {
  onChampionPulled?: () => void;
  onMatchEnded?: () => void;
}

export function useLcuEvents(
  flashKey: FlashKeyPreference,
  options?: UseLcuEventsOptions
) {
  const [summoner, setSummoner] = useState<SummonerInfo | null>(null);
  const [champSelect, setChampSelect] = useState<ChampSelectedPayload | null>(null);
  const [build, setBuild] = useState<ChampionBuild | null>(null);
  const [gameflowPhase, setGameflowPhase] = useState<string>("");
  const [spellsInverted, setSpellsInverted] = useState(false);
  const [selectedSecondarySpellId, setSelectedSecondarySpellId] = useState<number | null>(null);
  const [selectedRunePageIndex, setSelectedRunePageIndex] = useState<number>(0);
  const [selectedBuildIndex, setSelectedBuildIndex] = useState<number>(0);
  const [loading, setLoading] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const activeChampIdRef = useRef<number | null>(null);
  const buildRef = useRef<ChampionBuild | null>(null);
  const latestFetchSeqRef = useRef(0);
  const prevSyncedChampRef = useRef<number | null>(null);
  const gameflowPhaseRef = useRef<string>("");
  const hasAutoNavigatedRef = useRef<boolean>(false);
  const onChampionPulledRef = useRef(options?.onChampionPulled);
  const onMatchEndedRef = useRef(options?.onMatchEnded);

  useEffect(() => {
    onChampionPulledRef.current = options?.onChampionPulled;
    onMatchEndedRef.current = options?.onMatchEnded;
  }, [options?.onChampionPulled, options?.onMatchEnded]);

  useEffect(() => {
    buildRef.current = build;
  }, [build]);

  useEffect(() => {
    const displayedId = champSelect?.champion_id ?? build?.champion_id ?? null;
    if (prevSyncedChampRef.current !== displayedId) {
      prevSyncedChampRef.current = displayedId;
      commands.syncDisplayedChampion(displayedId).catch(() => {});
    }
  }, [champSelect?.champion_id, build?.champion_id]);

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
        buildRef.current = updatedBuild;
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
    let unlistenReconciled: (() => void) | undefined;
    let unlistenGameflowPhase: (() => void) | undefined;
    let unlistenMatchEnded: (() => void) | undefined;

    const notifyChampionPulledOnce = () => {
      if (!hasAutoNavigatedRef.current) {
        hasAutoNavigatedRef.current = true;
        onChampionPulledRef.current?.();
      }
    };

    const fetchBuildForPayload = async (payload: ChampSelectedPayload, fetchSeq: number) => {
      try {
        const qId = payload.queue_id || 420;
        const isAram =
          payload.queue_mode === "Aram" ||
          payload.queue_mode === "AramMayhem" ||
          qId === 450 ||
          qId === 2400;

        const recommended =
          !isAram && payload.assigned_position
            ? await commands.updateBuildFilters(
                payload.champion_id,
                qId,
                payload.assigned_position,
                null,
                null,
                null
              )
            : await commands.getRecommendedBuildForChampion(payload.champion_id, qId);

        if (
          recommended &&
          latestFetchSeqRef.current === fetchSeq &&
          activeChampIdRef.current === payload.champion_id
        ) {
          buildRef.current = recommended;
          setBuild((prev) => {
            if (prev?.champion_id !== recommended.champion_id) {
              setSelectedSecondarySpellId(null);
              setSelectedRunePageIndex(0);
              setSelectedBuildIndex(0);
            }
            return recommended;
          });
          notifyChampionPulledOnce();
        }
      } catch {
        // Ignore background fetch error, ws_listener will emit recommended-build
      }
    };

    const setupListeners = async () => {
      try {
        unlistenConnected = await events.lcuConnected.listen((event) => {
          setSummoner(event.payload);
          setErrorMsg(null);
        });

        unlistenDisconnected = await events.lcuDisconnected.listen(() => {
          activeChampIdRef.current = null;
          buildRef.current = null;
          latestFetchSeqRef.current += 1;
          hasAutoNavigatedRef.current = false;
          gameflowPhaseRef.current = "";
          setGameflowPhase("");
          setSummoner(null);
          setChampSelect(null);
          setBuild(null);
          setSpellsInverted(false);
          setSelectedSecondarySpellId(null);
          setSelectedRunePageIndex(0);
          setSelectedBuildIndex(0);
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

          await fetchBuildForPayload(event.payload, fetchSeq);
        });

        unlistenChampSelectEnded = await events.champSelectEnded.listen(() => {
          const inGame = isInGamePhase(gameflowPhaseRef.current);
          activeChampIdRef.current = null;
          latestFetchSeqRef.current += 1;

          if (inGame) {
            return;
          }

          buildRef.current = null;
          hasAutoNavigatedRef.current = false;
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
          buildRef.current = event.payload;
          setBuild((prev) => {
            if (prev?.champion_id !== event.payload.champion_id) {
              setSelectedSecondarySpellId(null);
              setSelectedRunePageIndex(0);
              setSelectedBuildIndex(0);
            }
            return event.payload;
          });
          notifyChampionPulledOnce();
        });

        unlistenReconciled = await events.champSelectReconciled.listen(async (event) => {
          const { champ_selected, build: reconciledBuild } = event.payload;
          const currentDisplayedChampId =
            activeChampIdRef.current ?? buildRef.current?.champion_id ?? null;
          const champDiverged = currentDisplayedChampId !== champ_selected.champion_id;

          activeChampIdRef.current = champ_selected.champion_id;

          if (champDiverged) {
            const fetchSeq = ++latestFetchSeqRef.current;
            setSpellsInverted(false);
            setSelectedSecondarySpellId(null);
            setSelectedRunePageIndex(0);
            setSelectedBuildIndex(0);
            setChampSelect(champ_selected);

            if (
              reconciledBuild &&
              reconciledBuild.champion_id === champ_selected.champion_id
            ) {
              buildRef.current = reconciledBuild;
              setBuild(reconciledBuild);
              notifyChampionPulledOnce();
            } else {
              await fetchBuildForPayload(champ_selected, fetchSeq);
            }
            return;
          }

          // Same champion: keep manual UI selections intact and only fill build if missing.
          setChampSelect(champ_selected);
          if (
            reconciledBuild &&
            reconciledBuild.champion_id === champ_selected.champion_id &&
            (!buildRef.current || buildRef.current.champion_id !== champ_selected.champion_id)
          ) {
            buildRef.current = reconciledBuild;
            setBuild(reconciledBuild);
            notifyChampionPulledOnce();
          }
        });

        unlistenGameflowPhase = await events.gameflowPhaseChanged.listen((event) => {
          gameflowPhaseRef.current = event.payload;
          setGameflowPhase(event.payload);
        });

        unlistenMatchEnded = await events.matchEnded.listen(() => {
          activeChampIdRef.current = null;
          buildRef.current = null;
          latestFetchSeqRef.current += 1;
          hasAutoNavigatedRef.current = false;
          setChampSelect(null);
          setBuild(null);
          setSpellsInverted(false);
          setSelectedSecondarySpellId(null);
          setSelectedRunePageIndex(0);
          setSelectedBuildIndex(0);
          onMatchEndedRef.current?.();
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
      if (unlistenReconciled) unlistenReconciled();
      if (unlistenGameflowPhase) unlistenGameflowPhase();
      if (unlistenMatchEnded) unlistenMatchEnded();
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
    gameflowPhase,
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

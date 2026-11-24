import { useState, useEffect, useRef } from "react";
import {
  AppLocale,
  ChampionBuild,
  ChampSelectedPayload,
  FlashKeyPreference,
  SummonerInfo,
} from "../../types";
import { lcuIpc } from "../../services/ipc";
import { translate } from "../../i18n";
import { computeOrderedSpells } from "../../utils/spells";
import { isArenaBuildContext } from "../../components/ui/queueContext";
import {
  fetchFilteredChampSelectBuild,
  injectItemBuildAction,
  injectRunePageAction,
  injectSpellPairAction,
} from "./lcuBuildActions";
import { isInGamePhase, subscribeToLcuEvents } from "./lcuEventListeners";

export { isInGamePhase };

export interface UseLcuEventsOptions {
  onChampionPulled?: () => void;
  onMatchEnded?: () => void;
  locale?: AppLocale;
  localeReady?: boolean;
}

export function useLcuEvents(
  flashKey: FlashKeyPreference,
  options?: UseLcuEventsOptions
) {
  const activeLocale: AppLocale = options?.locale ?? "pt_BR";
  const localeReady = options?.localeReady ?? true;
  const [summoner, setSummoner] = useState<SummonerInfo | null>(null);
  const [lcuInitialChecked, setLcuInitialChecked] = useState<boolean>(false);
  const [champSelect, setChampSelect] = useState<ChampSelectedPayload | null>(null);
  const [build, setBuild] = useState<ChampionBuild | null>(null);
  const [gameflowPhase, setGameflowPhase] = useState<string>("");
  const [spellsInverted, setSpellsInverted] = useState(false);
  const [selectedSpellPairIndex, setSelectedSpellPairIndex] = useState<number>(0);
  const [selectedRunePageIndex, setSelectedRunePageIndex] = useState<number>(0);
  const [selectedBuildIndex, setSelectedBuildIndex] = useState<number>(0);
  const [loading, setLoading] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const activeChampIdRef = useRef<number | null>(null);
  const buildRef = useRef<ChampionBuild | null>(null);
  const champSelectRef = useRef<ChampSelectedPayload | null>(null);
  const latestFetchSeqRef = useRef(0);
  const prevSyncedChampRef = useRef<number | null>(null);
  const gameflowPhaseRef = useRef<string>("");
  const hasAutoNavigatedRef = useRef<boolean>(false);
  const onChampionPulledRef = useRef(options?.onChampionPulled);
  const onMatchEndedRef = useRef(options?.onMatchEnded);
  const activeLocaleRef = useRef<AppLocale>(activeLocale);
  const prevLocaleRef = useRef<AppLocale | null>(localeReady ? activeLocale : null);

  useEffect(() => {
    onChampionPulledRef.current = options?.onChampionPulled;
    onMatchEndedRef.current = options?.onMatchEnded;
    activeLocaleRef.current = activeLocale;
  }, [options?.onChampionPulled, options?.onMatchEnded, activeLocale]);

  useEffect(() => {
    buildRef.current = build;
  }, [build]);

  useEffect(() => {
    champSelectRef.current = champSelect;
  }, [champSelect]);

  useEffect(() => {
    const displayedId = champSelect?.champion_id ?? build?.champion_id ?? null;
    if (prevSyncedChampRef.current !== displayedId) {
      prevSyncedChampRef.current = displayedId;
      lcuIpc.syncDisplayedChampion(displayedId).catch(() => {});
    }
  }, [champSelect?.champion_id, build?.champion_id]);

  const handleFilterChange = async (
    filterType: "position" | "tier" | "region" | "patch",
    value: string
  ) => {
    const currentBuild = buildRef.current ?? build;
    if (!currentBuild) return;
    if (isArenaBuildContext(currentBuild, champSelectRef.current ?? champSelect) && filterType !== "patch") {
      return;
    }
    const fetchSeq = ++latestFetchSeqRef.current;
    setLoading(true);
    setErrorMsg(null);
    try {
      const updatedBuild = await fetchFilteredChampSelectBuild(
        currentBuild,
        champSelectRef.current ?? champSelect,
        filterType,
        value
      );
      if (updatedBuild && latestFetchSeqRef.current === fetchSeq) {
        buildRef.current = updatedBuild;
        setBuild(updatedBuild);
      }
    } catch (err: unknown) {
      console.error("Failed to update build filters:", err);
      if (latestFetchSeqRef.current === fetchSeq) {
        setErrorMsg(
          typeof err === "string"
            ? err
            : translate(activeLocaleRef.current, "idle.fetchFilterError")
        );
      }
    } finally {
      if (latestFetchSeqRef.current === fetchSeq) setLoading(false);
    }
  };

  useEffect(() => {
    if (!localeReady) return;
    if (prevLocaleRef.current === null) {
      prevLocaleRef.current = activeLocale;
      return;
    }
    if (prevLocaleRef.current === activeLocale) return;
    prevLocaleRef.current = activeLocale;
    const currentBuild = buildRef.current;
    if (!currentBuild) return;
    const fetchSeq = ++latestFetchSeqRef.current;
    fetchFilteredChampSelectBuild(currentBuild, champSelectRef.current)
      .then((updatedBuild) => {
        if (updatedBuild && latestFetchSeqRef.current === fetchSeq) {
          buildRef.current = updatedBuild;
          setBuild(updatedBuild);
        }
      })
      .catch(() => {});
  }, [activeLocale, localeReady]);

  useEffect(() => {
    let active = true;
    const subscription = subscribeToLcuEvents({
      activeChampIdRef,
      buildRef,
      latestFetchSeqRef,
      hasAutoNavigatedRef,
      gameflowPhaseRef,
      onChampionPulledRef,
      onMatchEndedRef,
      setSummoner,
      setChampSelect,
      setBuild,
      setGameflowPhase,
      setSpellsInverted,
      setSelectedSpellPairIndex,
      setSelectedRunePageIndex,
      setSelectedBuildIndex,
      setErrorMsg,
    });

    const bootstrapLcu = async () => {
      try {
        const summonerPromise = lcuIpc.getSummonerStatus().catch(() => null);
        await subscription.ready;
        if (!active) return;

        const seqBeforeSnapshot = subscription.captureEventSeq();
        const [summonerData, snapshot] = await Promise.all([
          summonerPromise,
          lcuIpc.getActiveChampSelectSnapshot().catch(() => null),
        ]);
        if (!active) return;

        if (summonerData) setSummoner(summonerData);
        await subscription.hydrateSnapshot(snapshot, seqBeforeSnapshot, () => active);
      } finally {
        if (active) setLcuInitialChecked(true);
      }
    };

    void bootstrapLcu();

    return () => {
      active = false;
      subscription.unlisten();
    };
  }, []);

  const isArena = isArenaBuildContext(build, champSelect);

  const orderedSpells = isArena
    ? []
    : computeOrderedSpells(
        build?.spells,
        selectedSpellPairIndex,
        build?.spell_options,
        flashKey,
        spellsInverted
      );

  const handleSelectSpellPair = async (pairIndex: number) => {
    if (isArena || build?.supports_spells === false || isInGamePhase(gameflowPhaseRef.current)) return;
    if (!build?.spell_options || pairIndex < 0 || pairIndex >= build.spell_options.length) return;
    setSelectedSpellPairIndex(pairIndex);
    await injectSpellPairAction(build, pairIndex, flashKey, spellsInverted);
  };

  const handleSelectRunePage = async (pageIndex: number) => {
    if (isArena || build?.supports_runes === false) return;
    if (!build?.rune_pages || pageIndex >= build.rune_pages.length) return;
    setSelectedRunePageIndex(pageIndex);
    await injectRunePageAction(build, pageIndex);
  };

  const handleSelectBuild = async (index: number) => {
    if (!build?.build_options || index >= build.build_options.length) return;
    setSelectedBuildIndex(index);
    await injectItemBuildAction(build, champSelect, index);
  };

  return {
    summoner,
    lcuInitialChecked,
    champSelect,
    build,
    gameflowPhase,
    spellsInverted,
    setSpellsInverted,
    selectedSpellPairIndex,
    selectedRunePageIndex,
    selectedBuildIndex,
    loading,
    errorMsg,
    orderedSpells,
    handleFilterChange,
    handleSelectSpellPair,
    handleSelectRunePage,
    handleSelectBuild,
  };
}

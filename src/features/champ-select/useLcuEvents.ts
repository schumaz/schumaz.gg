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
import {
  fetchFilteredChampSelectBuild,
  injectItemBuildAction,
  injectRunePageAction,
  injectSecondarySpellAction,
} from "./lcuBuildActions";
import { isInGamePhase, subscribeToLcuEvents } from "./lcuEventListeners";

export { isInGamePhase };

export interface UseLcuEventsOptions {
  onChampionPulled?: () => void;
  onMatchEnded?: () => void;
  locale?: AppLocale;
}

export function useLcuEvents(
  flashKey: FlashKeyPreference,
  options?: UseLcuEventsOptions
) {
  const activeLocale: AppLocale = options?.locale ?? "pt_BR";
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
  const champSelectRef = useRef<ChampSelectedPayload | null>(null);
  const latestFetchSeqRef = useRef(0);
  const prevSyncedChampRef = useRef<number | null>(null);
  const gameflowPhaseRef = useRef<string>("");
  const hasAutoNavigatedRef = useRef<boolean>(false);
  const onChampionPulledRef = useRef(options?.onChampionPulled);
  const onMatchEndedRef = useRef(options?.onMatchEnded);
  const activeLocaleRef = useRef<AppLocale>(activeLocale);
  const prevLocaleRef = useRef<AppLocale>(activeLocale);

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
    const fetchSeq = ++latestFetchSeqRef.current;
    setLoading(true);
    setErrorMsg(null);
    try {
      const updatedBuild = await fetchFilteredChampSelectBuild(
        currentBuild,
        champSelect,
        filterType,
        value
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
            : translate(activeLocaleRef.current, "idle.fetchFilterError")
        );
      }
    } finally {
      if (latestFetchSeqRef.current === fetchSeq) setLoading(false);
    }
  };

  useEffect(() => {
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
  }, [activeLocale]);

  useEffect(() => {
    lcuIpc
      .getSummonerStatus()
      .then((data) => {
        if (data) setSummoner(data);
      })
      .catch(() => {});

    return subscribeToLcuEvents({
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
      setSelectedSecondarySpellId,
      setSelectedRunePageIndex,
      setSelectedBuildIndex,
      setErrorMsg,
    });
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
    await injectSecondarySpellAction(build, spellId, flashKey, spellsInverted);
  };

  const handleSelectRunePage = async (pageIndex: number) => {
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

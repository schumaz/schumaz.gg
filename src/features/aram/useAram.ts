import { useCallback, useEffect, useRef, useState } from "react";
import {
  AppLocale,
  AramAugmentEntry,
  AramChampionRank,
  AramMode,
} from "../../types";
import { aramIpc } from "../../services/ipc";
import { translate } from "../../i18n";
import { UseAramPersistenceOptions } from "./aramStorage";

export * from "./aramStorage";

export function useAram(
  isActive: boolean,
  currentChampionId: number | null,
  locale: AppLocale = "pt_BR",
  persistence?: UseAramPersistenceOptions
) {
  const onNavigationPatch = persistence?.onNavigationPatch;

  const [savedMode, setSavedMode] = useState<AramMode | null>(
    () => persistence?.savedAramMode ?? null
  );
  const [showChooser, setShowChooser] = useState<boolean>(
    () => (persistence?.savedAramMode ?? null) === null
  );

  const [tierLists, setTierLists] = useState<Partial<Record<AramMode, AramChampionRank[]>>>({});
  const [tierLoading, setTierLoading] = useState<Partial<Record<AramMode, boolean>>>({});
  const [tierErrors, setTierErrors] = useState<Partial<Record<AramMode, string | null>>>({});

  const [selectedChampionKey, setSelectedChampionKey] = useState<string | null>(
    () => persistence?.savedLastChampion ?? null
  );
  const [augmentsByChampion, setAugmentsByChampion] = useState<Record<string, AramAugmentEntry[]>>(
    {}
  );
  const [augmentsLoading, setAugmentsLoading] = useState(false);
  const [augmentsError, setAugmentsError] = useState<string | null>(null);

  const tierCacheRef = useRef<Partial<Record<AramMode, AramChampionRank[]>>>({});
  const tierFetchingRef = useRef<Partial<Record<AramMode, boolean>>>({});
  const augmentsCacheRef = useRef<Record<string, AramAugmentEntry[]>>({});
  const augmentsSeqRef = useRef(0);
  const wasMayhemVisibleRef = useRef(false);
  const lastAutoChampionIdRef = useRef<number | null>(null);
  const prevLocaleRef = useRef<AppLocale>(locale);
  const selectedChampionKeyRef = useRef<string | null>(selectedChampionKey);
  selectedChampionKeyRef.current = selectedChampionKey;

  const fetchTierList = useCallback(
    async (mode: AramMode, force = false) => {
      if (!force && tierCacheRef.current[mode]?.length) return;
      if (tierFetchingRef.current[mode]) return;

      tierFetchingRef.current[mode] = true;
      setTierLoading((prev) => ({ ...prev, [mode]: true }));
      setTierErrors((prev) => ({ ...prev, [mode]: null }));

      try {
        const data = await aramIpc.getAramTierList(mode);
        tierCacheRef.current[mode] = data;
        setTierLists((prev) => ({ ...prev, [mode]: data }));
      } catch (err: unknown) {
        const msg =
          typeof err === "string" ? err : translate(locale, "aram.fetchTierListError");
        setTierErrors((prev) => ({ ...prev, [mode]: msg }));
      } finally {
        tierFetchingRef.current[mode] = false;
        setTierLoading((prev) => ({ ...prev, [mode]: false }));
      }
    },
    [locale]
  );

  const fetchAugmentsForChampion = useCallback(
    async (championKey: string, force = false, persistSelection = true) => {
      const normalizedKey = championKey.trim().toLowerCase();
      if (!normalizedKey) return;

      setSelectedChampionKey(normalizedKey);
      setAugmentsError(null);
      if (persistSelection) {
        onNavigationPatch?.({ aramMayhemLastChampion: normalizedKey });
      }

      if (!force && augmentsCacheRef.current[normalizedKey]) {
        setAugmentsLoading(false);
        return;
      }

      const seq = ++augmentsSeqRef.current;
      setAugmentsLoading(true);

      try {
        const data = await aramIpc.getAramMayhemAugments(normalizedKey);
        augmentsCacheRef.current[normalizedKey] = data;
        if (augmentsSeqRef.current === seq) {
          setAugmentsByChampion((prev) => ({ ...prev, [normalizedKey]: data }));
        }
      } catch (err: unknown) {
        if (augmentsSeqRef.current === seq) {
          const msg =
            typeof err === "string" ? err : translate(locale, "aram.fetchAugmentsError");
          setAugmentsError(msg);
        }
      } finally {
        if (augmentsSeqRef.current === seq) setAugmentsLoading(false);
      }
    },
    [locale, onNavigationPatch]
  );

  useEffect(() => {
    if (persistence?.savedAramMode !== undefined) {
      const mode = persistence.savedAramMode ?? null;
      setSavedMode(mode);
      setShowChooser(mode === null);
    }
    if (persistence?.savedLastChampion && !selectedChampionKeyRef.current) {
      fetchAugmentsForChampion(persistence.savedLastChampion, false, false);
    }
  }, [persistence?.savedAramMode, persistence?.savedLastChampion, fetchAugmentsForChampion]);

  useEffect(() => {
    if (prevLocaleRef.current === locale) return;
    prevLocaleRef.current = locale;
    augmentsCacheRef.current = {};
    setAugmentsByChampion({});
    if (selectedChampionKeyRef.current) {
      fetchAugmentsForChampion(selectedChampionKeyRef.current, true, false);
    }
  }, [locale, fetchAugmentsForChampion]);

  useEffect(() => {
    if (!isActive || showChooser || !savedMode) return;
    if (!tierLists[savedMode] && !tierLoading[savedMode] && !tierErrors[savedMode]) {
      fetchTierList(savedMode);
    }
  }, [isActive, showChooser, savedMode, tierLists, tierLoading, tierErrors, fetchTierList]);

  useEffect(() => {
    const isMayhemVisible = isActive && !showChooser && savedMode === "aram_mayhem";
    if (!isMayhemVisible) {
      wasMayhemVisibleRef.current = false;
      return;
    }

    const mayhemList = tierLists.aram_mayhem;
    if (!mayhemList || mayhemList.length === 0) return;

    const justEnteredMayhem = !wasMayhemVisibleRef.current;
    const championChanged = currentChampionId !== lastAutoChampionIdRef.current;
    wasMayhemVisibleRef.current = true;

    if (currentChampionId !== null && (justEnteredMayhem || championChanged)) {
      const matched = mayhemList.find((c) => c.id === currentChampionId);
      if (matched) {
        lastAutoChampionIdRef.current = currentChampionId;
        fetchAugmentsForChampion(matched.key);
      }
    }
  }, [
    isActive,
    showChooser,
    savedMode,
    tierLists.aram_mayhem,
    currentChampionId,
    fetchAugmentsForChampion,
  ]);

  const handleAramDockClick = useCallback(
    (wasAlreadyOnAram: boolean) => {
      if (!wasAlreadyOnAram) {
        setShowChooser(savedMode === null);
        return;
      }
      if (savedMode === null) {
        setShowChooser(true);
      } else {
        setShowChooser((prev) => !prev);
      }
    },
    [savedMode]
  );

  const selectMode = useCallback(
    (mode: AramMode) => {
      onNavigationPatch?.({ savedAramMode: mode });
      setSavedMode(mode);
      setShowChooser(false);
    },
    [onNavigationPatch]
  );

  const retryTierList = useCallback((mode: AramMode) => fetchTierList(mode, true), [fetchTierList]);

  const retryAugments = useCallback(() => {
    if (selectedChampionKey) fetchAugmentsForChampion(selectedChampionKey, true, false);
  }, [selectedChampionKey, fetchAugmentsForChampion]);

  return {
    savedMode,
    showChooser,
    tierLists,
    tierLoading,
    tierErrors,
    selectedChampionKey,
    augmentsByChampion,
    augmentsLoading,
    augmentsError,
    handleAramDockClick,
    selectMode,
    selectChampion: fetchAugmentsForChampion,
    retryTierList,
    retryAugments,
  };
}

import { useCallback, useEffect, useRef, useState } from "react";
import {
  ARAM_SUBMODE_STORAGE_KEY,
  AramAugmentEntry,
  AramChampionRank,
  AramMode,
  commands,
} from "../types";

export function loadSavedAramMode(): AramMode | null {
  try {
    const raw = localStorage.getItem(ARAM_SUBMODE_STORAGE_KEY);
    if (raw === "aram" || raw === "aram_mayhem") {
      return raw;
    }
  } catch {
    // Ignore storage access errors
  }
  return null;
}

export function saveAramMode(mode: AramMode): void {
  try {
    localStorage.setItem(ARAM_SUBMODE_STORAGE_KEY, mode);
  } catch {
    // Ignore storage quota errors
  }
}

export function useAram(isActive: boolean, currentChampionId: number | null) {
  const [savedMode, setSavedMode] = useState<AramMode | null>(loadSavedAramMode);
  const [showChooser, setShowChooser] = useState<boolean>(() => loadSavedAramMode() === null);

  const [tierLists, setTierLists] = useState<Partial<Record<AramMode, AramChampionRank[]>>>({});
  const [tierLoading, setTierLoading] = useState<Partial<Record<AramMode, boolean>>>({});
  const [tierErrors, setTierErrors] = useState<Partial<Record<AramMode, string | null>>>({});

  const [selectedChampionKey, setSelectedChampionKey] = useState<string | null>(null);
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

  const fetchTierList = useCallback(async (mode: AramMode, force = false) => {
    if (!force && tierCacheRef.current[mode]?.length) {
      return;
    }
    if (tierFetchingRef.current[mode]) {
      return;
    }

    tierFetchingRef.current[mode] = true;
    setTierLoading((prev) => ({ ...prev, [mode]: true }));
    setTierErrors((prev) => ({ ...prev, [mode]: null }));

    try {
      const data = await commands.getAramTierList(mode);
      tierCacheRef.current[mode] = data;
      setTierLists((prev) => ({ ...prev, [mode]: data }));
    } catch (err: unknown) {
      const msg =
        typeof err === "string" ? err : "Não foi possível carregar o ranking no momento.";
      setTierErrors((prev) => ({ ...prev, [mode]: msg }));
    } finally {
      tierFetchingRef.current[mode] = false;
      setTierLoading((prev) => ({ ...prev, [mode]: false }));
    }
  }, []);

  const fetchAugmentsForChampion = useCallback(async (championKey: string, force = false) => {
    const normalizedKey = championKey.trim().toLowerCase();
    if (!normalizedKey) return;

    setSelectedChampionKey(normalizedKey);
    setAugmentsError(null);

    if (!force && augmentsCacheRef.current[normalizedKey]) {
      setAugmentsLoading(false);
      return;
    }

    const seq = ++augmentsSeqRef.current;
    setAugmentsLoading(true);

    try {
      const data = await commands.getAramMayhemAugments(normalizedKey);
      augmentsCacheRef.current[normalizedKey] = data;
      if (augmentsSeqRef.current === seq) {
        setAugmentsByChampion((prev) => ({ ...prev, [normalizedKey]: data }));
      }
    } catch (err: unknown) {
      if (augmentsSeqRef.current === seq) {
        const msg =
          typeof err === "string"
            ? err
            : "Não foi possível carregar os aprimoramentos desse campeão.";
        setAugmentsError(msg);
      }
    } finally {
      if (augmentsSeqRef.current === seq) {
        setAugmentsLoading(false);
      }
    }
  }, []);

  // Trigger tier list fetch when viewing a sub-tab that hasn't loaded yet
  useEffect(() => {
    if (!isActive || showChooser || !savedMode) return;
    if (!tierLists[savedMode] && !tierLoading[savedMode] && !tierErrors[savedMode]) {
      fetchTierList(savedMode);
    }
  }, [isActive, showChooser, savedMode, tierLists, tierLoading, tierErrors, fetchTierList]);

  // Auto-select the active Champ Select / displayed champion when entering ARAM Mayhem
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

  const selectMode = useCallback((mode: AramMode) => {
    saveAramMode(mode);
    setSavedMode(mode);
    setShowChooser(false);
  }, []);

  const retryTierList = useCallback(
    (mode: AramMode) => {
      fetchTierList(mode, true);
    },
    [fetchTierList]
  );

  const retryAugments = useCallback(() => {
    if (selectedChampionKey) {
      fetchAugmentsForChampion(selectedChampionKey, true);
    }
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

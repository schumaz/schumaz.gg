import { useCallback, useEffect, useRef, useState } from "react";
import type {
  AppLocale,
  ArenaChampionEntry,
  ArenaTierListResponse,
  ChampionBuild,
} from "../../types";
import { arenaIpc } from "../../services/ipc";
import { translate } from "../../i18n";
import {
  ArenaFilters,
  buildArenaCacheKey,
  loadSavedArenaFilters,
  saveArenaFilters,
  synergyToArenaChampionEntry,
} from "./arenaStorage";

export * from "./arenaStorage";

/**
 * State and IPC controller for the Arena (`CHERRY`) Tier List and Champion Detail tab.
 */
export function useArena(
  isActive: boolean,
  patch48hProtection: boolean,
  locale: AppLocale = "pt_BR"
) {
  const [filters, setFilters] = useState<ArenaFilters>(() => loadSavedArenaFilters());
  const [tierData, setTierData] = useState<ArenaTierListResponse | null>(null);
  const [tierLoading, setTierLoading] = useState(false);
  const [tierError, setTierError] = useState<string | null>(null);
  const [selectedChampion, setSelectedChampion] = useState<ArenaChampionEntry | null>(null);
  const [championBuild, setChampionBuild] = useState<ChampionBuild | null>(null);
  const [buildLoading, setBuildLoading] = useState(false);
  const [buildError, setBuildError] = useState<string | null>(null);
  const [selectedBuildIndex, setSelectedBuildIndex] = useState<number>(0);

  const tierCacheRef = useRef<Record<string, ArenaTierListResponse>>({});
  const tierSeqRef = useRef(0);
  const buildSeqRef = useRef(0);
  const loadedCacheKeyRef = useRef<string | null>(null);
  const staleBuildRef = useRef(false);
  const prevLocaleRef = useRef<AppLocale>(locale);
  const selectedChampionRef = useRef<ArenaChampionEntry | null>(selectedChampion);
  selectedChampionRef.current = selectedChampion;
  const filtersRef = useRef<ArenaFilters>(filters);
  filtersRef.current = filters;
  const tierDataRef = useRef<ArenaTierListResponse | null>(tierData);
  tierDataRef.current = tierData;
  const championBuildRef = useRef<ChampionBuild | null>(championBuild);
  championBuildRef.current = championBuild;

  const fetchTierList = useCallback(
    async (targetFilters: ArenaFilters, force = false) => {
      const cacheKey = buildArenaCacheKey(targetFilters, patch48hProtection, locale);
      if (!force && tierCacheRef.current[cacheKey]) {
        loadedCacheKeyRef.current = cacheKey;
        setTierData(tierCacheRef.current[cacheKey]);
        setTierError(null);
        setTierLoading(false);
        return;
      }

      const seq = ++tierSeqRef.current;
      setTierLoading(true);
      setTierError(null);
      try {
        const res = await arenaIpc.getArenaTierList(targetFilters.patch);
        tierCacheRef.current[cacheKey] = res;
        if (tierSeqRef.current === seq) {
          loadedCacheKeyRef.current = cacheKey;
          setTierData(res);
        }
      } catch (err: unknown) {
        if (tierSeqRef.current === seq) {
          setTierError(
            typeof err === "string" ? err : translate(locale, "arenaTab.fetchTierListError")
          );
        }
      } finally {
        if (tierSeqRef.current === seq) setTierLoading(false);
      }
    },
    [patch48hProtection, locale]
  );

  const fetchChampionBuild = useCallback(
    async (champion: ArenaChampionEntry, targetFilters: ArenaFilters) => {
      staleBuildRef.current = false;
      const seq = ++buildSeqRef.current;
      setBuildLoading(true);
      setBuildError(null);
      try {
        const build = await arenaIpc.getArenaChampionBuild(champion.id, targetFilters.patch);
        if (buildSeqRef.current === seq) {
          setChampionBuild(build);
        }
      } catch (err: unknown) {
        if (buildSeqRef.current === seq) {
          setBuildError(
            typeof err === "string" ? err : translate(locale, "arenaTab.fetchBuildError")
          );
        }
      } finally {
        if (buildSeqRef.current === seq) setBuildLoading(false);
      }
    },
    [locale]
  );

  useEffect(() => {
    if (prevLocaleRef.current === locale) return;
    prevLocaleRef.current = locale;
    if (selectedChampionRef.current) {
      void fetchChampionBuild(selectedChampionRef.current, filtersRef.current);
    }
  }, [locale, fetchChampionBuild]);

  useEffect(() => {
    if (!isActive) return;
    const key = buildArenaCacheKey(filters, patch48hProtection, locale);
    if (loadedCacheKeyRef.current !== key && !tierError) {
      void fetchTierList(filters);
    }
    if (staleBuildRef.current && selectedChampionRef.current) {
      staleBuildRef.current = false;
      void fetchChampionBuild(selectedChampionRef.current, filters);
    }
  }, [isActive, filters, patch48hProtection, locale, tierError, fetchTierList, fetchChampionBuild]);

  const handlePatchChange = useCallback((value: string) => {
    const normalized = value.trim();
    const next: ArenaFilters = { patch: normalized || null };
    saveArenaFilters(next);
    setFilters(next);
    setTierError(null);
  }, []);

  const selectChampion = useCallback(
    (entry: ArenaChampionEntry) => {
      setSelectedChampion(entry);
      setChampionBuild(null);
      setBuildError(null);
      setSelectedBuildIndex(0);
      void fetchChampionBuild(entry, filtersRef.current);
    },
    [fetchChampionBuild]
  );

  const selectChampionById = useCallback(
    (championId: number) => {
      const fromTierList = tierDataRef.current?.champions.find((c) => c.id === championId);
      if (fromTierList) {
        selectChampion(fromTierList);
        return;
      }

      const fromSynergy = championBuildRef.current?.arena_synergies?.find(
        (syn) => syn.champion_id === championId
      );
      if (fromSynergy) {
        selectChampion(synergyToArenaChampionEntry(fromSynergy));
      }
    },
    [selectChampion]
  );

  const handleDetailFilterChange = useCallback(
    (filterType: "position" | "tier" | "region" | "patch", value: string) => {
      if (filterType !== "patch") return;
      const normalized = value.trim();
      const nextFilters: ArenaFilters = { patch: normalized || null };
      saveArenaFilters(nextFilters);
      setFilters(nextFilters);
      setTierError(null);

      if (selectedChampionRef.current) {
        void fetchChampionBuild(selectedChampionRef.current, nextFilters);
      }
    },
    [fetchChampionBuild]
  );

  const clearSelectedChampion = useCallback(() => {
    buildSeqRef.current += 1;
    staleBuildRef.current = false;
    setSelectedChampion(null);
    setChampionBuild(null);
    setBuildError(null);
    setBuildLoading(false);
  }, []);

  const invalidateCache = useCallback(() => {
    tierCacheRef.current = {};
    loadedCacheKeyRef.current = null;
    tierSeqRef.current += 1;
    buildSeqRef.current += 1;
    staleBuildRef.current = Boolean(selectedChampionRef.current);
    setTierData(null);
    setTierError(null);
    setChampionBuild(null);
    setBuildError(null);
  }, []);

  const retryTierList = useCallback(
    () => fetchTierList(filters, true),
    [filters, fetchTierList]
  );

  const retryChampionBuild = useCallback(() => {
    if (selectedChampion) {
      void fetchChampionBuild(selectedChampion, filters);
    }
  }, [selectedChampion, filters, fetchChampionBuild]);

  const handlePatchProtectionChange = useCallback(() => {
    const nextFilters: ArenaFilters = { patch: null };
    saveArenaFilters(nextFilters);
    setFilters(nextFilters);
    loadedCacheKeyRef.current = null;
    setTierError(null);
    if (isActive) {
      void fetchTierList(nextFilters, true);
      if (selectedChampionRef.current) {
        void fetchChampionBuild(selectedChampionRef.current, nextFilters);
      }
    }
  }, [isActive, fetchTierList, fetchChampionBuild]);

  return {
    filters, tierData, tierLoading, tierError, selectedChampion, championBuild, buildLoading, buildError,
    selectedBuildIndex, setSelectedBuildIndex, handlePatchChange, handleDetailFilterChange, selectChampion,
    selectChampionById, clearSelectedChampion, retryTierList, retryChampionBuild, handlePatchProtectionChange,
    invalidateCache,
  };
}

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  AppLocale, ChampionBuild, FlashKeyPreference, RankedChampionEntry, RankedLane, RankedTierListResponse,
} from "../../types";
import { rankedIpc } from "../../services/ipc";
import { translate } from "../../i18n";
import {
  applyTierListFilterChange, buildRankedCacheKey, computeNextDetailRankedFilters,
  DEFAULT_RANKED_FILTERS, extractIniRankedFilters, extractValidRankedLane,
  RankedChampionShortcut, RankedFilters, resolveRankedMatchupEntry,
  resolveRankedShortcutEntry, UseRankedPersistenceOptions,
} from "./rankedStorage";
import { computeOrderedSpells } from "../../utils/spells";

export * from "./rankedStorage";

export function useRanked(
  isActive: boolean,
  flashKey: FlashKeyPreference,
  patch48hProtection: boolean,
  locale: AppLocale = "pt_BR",
  persistence?: UseRankedPersistenceOptions
) {
  const onNavigationPatch = persistence?.onNavigationPatch;
  const [filters, setFilters] = useState<RankedFilters>(() =>
    persistence?.savedFilters ? extractIniRankedFilters(persistence.savedFilters) : DEFAULT_RANKED_FILTERS
  );
  const [tierData, setTierData] = useState<RankedTierListResponse | null>(null);
  const [tierLoading, setTierLoading] = useState(false);
  const [tierError, setTierError] = useState<string | null>(null);
  const [selectedChampion, setSelectedChampion] = useState<RankedChampionEntry | null>(null);
  const [championBuild, setChampionBuild] = useState<ChampionBuild | null>(null);
  const [buildLoading, setBuildLoading] = useState(false);
  const [buildError, setBuildError] = useState<string | null>(null);
  const [selectedRunePageIndex, setSelectedRunePageIndex] = useState<number>(0);
  const [selectedBuildIndex, setSelectedBuildIndex] = useState<number>(0);

  const tierCacheRef = useRef<Record<string, RankedTierListResponse>>({});
  const tierSeqRef = useRef(0);
  const buildSeqRef = useRef(0);
  const loadedCacheKeyRef = useRef<string | null>(null);
  const prevLocaleRef = useRef<AppLocale>(locale);
  const autoLaneRef = useRef(false);
  const hydratedIniRef = useRef(Boolean(persistence?.savedFilters));
  const latestStateRef = useRef({ selectedChampion, filters, tierData, championBuild });
  latestStateRef.current = { selectedChampion, filters, tierData, championBuild };

  const persistFilters = useCallback(
    (next: RankedFilters) => onNavigationPatch?.({
      rankedFilterRegion: next.region,
      rankedFilterTier: next.tier,
      rankedFilterLane: next.lane,
      rankedFilterPatch: next.patch ?? "",
    }),
    [onNavigationPatch]
  );

  useEffect(() => {
    if (hydratedIniRef.current || !persistence?.savedFilters) return;
    hydratedIniRef.current = true;
    setFilters(extractIniRankedFilters(persistence.savedFilters));
  }, [persistence?.savedFilters]);

  const fetchTierList = useCallback(
    async (targetFilters: RankedFilters, force = false) => {
      const cacheKey = buildRankedCacheKey(targetFilters, patch48hProtection);
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
        const res = await rankedIpc.getRankedTierList(targetFilters.region, targetFilters.tier, targetFilters.patch);
        tierCacheRef.current[cacheKey] = res;
        if (tierSeqRef.current === seq) {
          loadedCacheKeyRef.current = cacheKey;
          setTierData(res);
        }
      } catch (err: unknown) {
        if (tierSeqRef.current === seq) {
          setTierError(typeof err === "string" ? err : translate(locale, "ranked.fetchTierListError"));
        }
      } finally {
        if (tierSeqRef.current === seq) setTierLoading(false);
      }
    },
    [patch48hProtection, locale]
  );

  const fetchChampionBuild = useCallback(
    async (champion: RankedChampionEntry, laneOverride: RankedLane | null, targetFilters: RankedFilters) => {
      const seq = ++buildSeqRef.current;
      setBuildLoading(true);
      setBuildError(null);
      try {
        const build = await rankedIpc.getRankedChampionBuild(champion.id, laneOverride, targetFilters.tier, targetFilters.region, targetFilters.patch);
        if (buildSeqRef.current === seq) {
          setChampionBuild(build);
          const resolved = laneOverride === null ? extractValidRankedLane(build.active_position) : null;
          if (resolved) {
            autoLaneRef.current = false;
            setSelectedChampion((prev) => (prev && prev.id === champion.id ? { ...prev, lane: resolved } : prev));
          }
        }
      } catch (err: unknown) {
        if (buildSeqRef.current === seq) {
          setBuildError(typeof err === "string" ? err : translate(locale, "ranked.fetchBuildError"));
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
    const { selectedChampion: sc, filters: f } = latestStateRef.current;
    if (sc) fetchChampionBuild(sc, autoLaneRef.current ? null : sc.lane, f);
  }, [locale, fetchChampionBuild]);

  useEffect(() => {
    if (!isActive) return;
    const key = buildRankedCacheKey(filters, patch48hProtection);
    if (loadedCacheKeyRef.current !== key && !tierError) fetchTierList(filters);
  }, [isActive, filters, patch48hProtection, tierError, fetchTierList]);

  const handleTierListFilterChange = useCallback(
    (filterType: "position" | "tier" | "region" | "patch", value: string) => {
      if (filterType !== "position") setTierError(null);
      setFilters((prev) => {
        const next = applyTierListFilterChange(prev, filterType, value);
        persistFilters(next);
        return next;
      });
    },
    [persistFilters]
  );

  const selectChampion = useCallback(
    (entry: RankedChampionEntry) => {
      autoLaneRef.current = false;
      setSelectedChampion(entry);
      setChampionBuild(null);
      setBuildError(null);
      setSelectedRunePageIndex(0);
      setSelectedBuildIndex(0);
      fetchChampionBuild(entry, entry.lane, filters);
    },
    [filters, fetchChampionBuild]
  );

  const openChampionShortcut = useCallback(
    (shortcut: RankedChampionShortcut) => {
      const { entry, positionOverride } = resolveRankedShortcutEntry(shortcut, latestStateRef.current.tierData);
      autoLaneRef.current = positionOverride === null;
      setSelectedChampion(entry);
      setChampionBuild(null);
      setBuildError(null);
      setSelectedRunePageIndex(0);
      setSelectedBuildIndex(0);
      fetchChampionBuild(entry, positionOverride, latestStateRef.current.filters);
    },
    [fetchChampionBuild]
  );

  const selectChampionById = useCallback(
    (championId: number) => {
      const { tierData: td, championBuild: cb, selectedChampion: sc, filters: f } = latestStateRef.current;
      const entry = resolveRankedMatchupEntry(championId, td, cb, sc, f);
      if (entry) selectChampion(entry);
    },
    [selectChampion]
  );

  const handleDetailFilterChange = useCallback(
    (filterType: "position" | "tier" | "region" | "patch", value: string) => {
      if (filterType === "position") autoLaneRef.current = false;
      const currentChamp = selectedChampion;
      const { nextFilters, nextLane } = computeNextDetailRankedFilters(filters, championBuild, currentChamp, filterType, value);
      persistFilters(nextFilters);
      setFilters(nextFilters);
      setTierError(null);
      if (currentChamp) {
        const updatedChamp: RankedChampionEntry = nextLane !== currentChamp.lane ? { ...currentChamp, lane: nextLane } : currentChamp;
        setSelectedChampion(updatedChamp);
        fetchChampionBuild(updatedChamp, autoLaneRef.current ? null : nextLane, nextFilters);
      }
    },
    [selectedChampion, championBuild, filters, fetchChampionBuild, persistFilters]
  );

  const clearSelectedChampion = useCallback(() => {
    buildSeqRef.current += 1;
    autoLaneRef.current = false;
    setSelectedChampion(null);
    setChampionBuild(null);
    setBuildError(null);
    setBuildLoading(false);
  }, []);

  const retryTierList = useCallback(() => fetchTierList(filters, true), [filters, fetchTierList]);

  const retryChampionBuild = useCallback(() => {
    if (selectedChampion) fetchChampionBuild(selectedChampion, autoLaneRef.current ? null : selectedChampion.lane, filters);
  }, [selectedChampion, filters, fetchChampionBuild]);

  const handlePatchProtectionChange = useCallback(() => {
    const nextFilters: RankedFilters = { ...filters, patch: null };
    persistFilters(nextFilters);
    setFilters(nextFilters);
    loadedCacheKeyRef.current = null;
    setTierError(null);
    if (isActive) {
      fetchTierList(nextFilters, true);
      if (selectedChampion) fetchChampionBuild(selectedChampion, autoLaneRef.current ? null : selectedChampion.lane, nextFilters);
    }
  }, [filters, isActive, selectedChampion, fetchTierList, fetchChampionBuild, persistFilters]);

  const orderedSpells = useMemo(
    () => computeOrderedSpells(championBuild?.spells, 0, championBuild?.spell_options, flashKey, false),
    [championBuild?.spells, championBuild?.spell_options, flashKey]
  );

  return {
    filters, tierData, tierLoading, tierError, selectedChampion, championBuild, buildLoading, buildError,
    selectedRunePageIndex, selectedBuildIndex, orderedSpells, setSelectedRunePageIndex, setSelectedBuildIndex,
    handleTierListFilterChange, handleDetailFilterChange, selectChampion, openChampionShortcut,
    selectChampionById, clearSelectedChampion, retryTierList, retryChampionBuild, handlePatchProtectionChange,
  };
}

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  AppLocale,
  ChampionBuild,
  FlashKeyPreference,
  NavigationStatePatch,
  RANKED_FILTERS_STORAGE_KEY,
  RankedChampionEntry,
  RankedLane,
  RankedTierListResponse,
} from "../types";
import { rankedIpc } from "../services/ipc";
import { translate } from "../i18n";
import { computeOrderedSpells } from "../utils/spells";

export type RankedLaneFilter = "all" | RankedLane;

export interface RankedFilters {
  region: string;
  tier: string;
  lane: RankedLaneFilter;
  patch: string | null;
}

export const DEFAULT_RANKED_FILTERS: RankedFilters = {
  region: "global",
  tier: "emerald_plus",
  lane: "all",
  patch: null,
};

const VALID_LANES = new Set<string>(["all", "top", "jungle", "mid", "adc", "support"]);

export function loadSavedRankedFilters(): RankedFilters {
  try {
    const raw = localStorage.getItem(RANKED_FILTERS_STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return {
        region:
          typeof parsed.region === "string" && parsed.region.trim()
            ? parsed.region.trim()
            : DEFAULT_RANKED_FILTERS.region,
        tier:
          typeof parsed.tier === "string" && parsed.tier.trim()
            ? parsed.tier.trim()
            : DEFAULT_RANKED_FILTERS.tier,
        lane:
          typeof parsed.lane === "string" && VALID_LANES.has(parsed.lane)
            ? (parsed.lane as RankedLaneFilter)
            : DEFAULT_RANKED_FILTERS.lane,
        patch:
          typeof parsed.patch === "string" && parsed.patch.trim()
            ? parsed.patch.trim()
            : null,
      };
    }
  } catch {
    // Ignore storage access or parse errors
  }
  return { ...DEFAULT_RANKED_FILTERS };
}

export function saveRankedFilters(filters: RankedFilters): void {
  try {
    localStorage.setItem(RANKED_FILTERS_STORAGE_KEY, JSON.stringify(filters));
  } catch {
    // Ignore storage quota errors
  }
}

export interface UseRankedPersistenceOptions {
  iniBacked?: boolean;
  savedFilters?: {
    region?: string | null;
    tier?: string | null;
    lane?: string | null;
    patch?: string | null;
  };
  onNavigationPatch?: (patch: NavigationStatePatch) => void;
}

export function useRanked(
  isActive: boolean,
  flashKey: FlashKeyPreference,
  patch48hProtection: boolean,
  locale: AppLocale = "pt_BR",
  persistence?: UseRankedPersistenceOptions
) {
  const iniBacked = Boolean(persistence?.iniBacked);
  const onNavigationPatch = persistence?.onNavigationPatch;

  const [filters, setFilters] = useState<RankedFilters>(loadSavedRankedFilters);
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
  const hydratedIniRef = useRef(false);
  const selectedChampionRef = useRef<RankedChampionEntry | null>(selectedChampion);
  selectedChampionRef.current = selectedChampion;
  const filtersRef = useRef<RankedFilters>(filters);
  filtersRef.current = filters;

  const persistFilters = useCallback(
    (next: RankedFilters) => {
      if (iniBacked) {
        onNavigationPatch?.({
          rankedFilterRegion: next.region,
          rankedFilterTier: next.tier,
          rankedFilterLane: next.lane,
          rankedFilterPatch: next.patch ?? "",
        });
      } else {
        saveRankedFilters(next);
      }
    },
    [iniBacked, onNavigationPatch]
  );

  useEffect(() => {
    if (!iniBacked || hydratedIniRef.current || !persistence?.savedFilters) return;
    hydratedIniRef.current = true;
    const sf = persistence.savedFilters;
    setFilters({
      region: sf.region?.trim() || DEFAULT_RANKED_FILTERS.region,
      tier: sf.tier?.trim() || DEFAULT_RANKED_FILTERS.tier,
      lane:
        sf.lane && VALID_LANES.has(sf.lane)
          ? (sf.lane as RankedLaneFilter)
          : DEFAULT_RANKED_FILTERS.lane,
      patch: sf.patch?.trim() ? sf.patch.trim() : null,
    });
  }, [iniBacked, persistence?.savedFilters]);

  const buildCacheKey = useCallback(
    (f: RankedFilters, protection: boolean) =>
      `${f.region}:${f.tier}:${f.patch ?? ""}:${protection ? "1" : "0"}`,
    []
  );

  const fetchTierList = useCallback(
    async (targetFilters: RankedFilters, force = false) => {
      const cacheKey = buildCacheKey(targetFilters, patch48hProtection);
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
        const res = await rankedIpc.getRankedTierList(
          targetFilters.region,
          targetFilters.tier,
          targetFilters.patch
        );
        tierCacheRef.current[cacheKey] = res;
        if (tierSeqRef.current === seq) {
          loadedCacheKeyRef.current = cacheKey;
          setTierData(res);
        }
      } catch (err: unknown) {
        if (tierSeqRef.current === seq) {
          const msg =
            typeof err === "string" ? err : translate(locale, "ranked.fetchTierListError");
          setTierError(msg);
        }
      } finally {
        if (tierSeqRef.current === seq) {
          setTierLoading(false);
        }
      }
    },
    [buildCacheKey, patch48hProtection, locale]
  );

  const fetchChampionBuild = useCallback(
    async (
      champion: RankedChampionEntry,
      laneOverride: RankedLane,
      targetFilters: RankedFilters
    ) => {
      const seq = ++buildSeqRef.current;
      setBuildLoading(true);
      setBuildError(null);

      try {
        const build = await rankedIpc.getRankedChampionBuild(
          champion.id,
          laneOverride,
          targetFilters.tier,
          targetFilters.region,
          targetFilters.patch
        );
        if (buildSeqRef.current === seq) {
          setChampionBuild(build);
        }
      } catch (err: unknown) {
        if (buildSeqRef.current === seq) {
          const msg =
            typeof err === "string" ? err : translate(locale, "ranked.fetchBuildError");
          setBuildError(msg);
        }
      } finally {
        if (buildSeqRef.current === seq) {
          setBuildLoading(false);
        }
      }
    },
    [locale]
  );

  // Refresh localized champion build if user switches language while inspecting a champion
  useEffect(() => {
    if (prevLocaleRef.current === locale) return;
    prevLocaleRef.current = locale;
    if (selectedChampionRef.current) {
      fetchChampionBuild(
        selectedChampionRef.current,
        selectedChampionRef.current.lane,
        filtersRef.current
      );
    }
  }, [locale, fetchChampionBuild]);

  // Trigger tier list fetch whenever the tab is active and filters/protection key changed
  useEffect(() => {
    if (!isActive) return;
    const key = buildCacheKey(filters, patch48hProtection);
    if (loadedCacheKeyRef.current !== key && !tierError) {
      fetchTierList(filters);
    }
  }, [isActive, filters, patch48hProtection, buildCacheKey, tierError, fetchTierList]);

  const updateFilters = useCallback(
    (updater: (prev: RankedFilters) => RankedFilters) => {
      setFilters((prev) => {
        const next = updater(prev);
        persistFilters(next);
        return next;
      });
    },
    [persistFilters]
  );

  const handleTierListFilterChange = useCallback(
    (filterType: "position" | "tier" | "region" | "patch", value: string) => {
      const normalized = value.trim();
      if (filterType === "position") {
        const nextLane: RankedLaneFilter = VALID_LANES.has(normalized)
          ? (normalized as RankedLaneFilter)
          : "all";
        updateFilters((prev) => ({ ...prev, lane: nextLane }));
        return;
      }

      setTierError(null);
      updateFilters((prev) => {
        if (filterType === "region") {
          return { ...prev, region: normalized || "global" };
        }
        if (filterType === "tier") {
          return { ...prev, tier: normalized || "emerald_plus" };
        }
        return { ...prev, patch: normalized || null };
      });
    },
    [updateFilters]
  );

  const selectChampion = useCallback(
    (entry: RankedChampionEntry) => {
      setSelectedChampion(entry);
      setChampionBuild(null);
      setBuildError(null);
      setSelectedRunePageIndex(0);
      setSelectedBuildIndex(0);
      fetchChampionBuild(entry, entry.lane, filters);
    },
    [filters, fetchChampionBuild]
  );

  const handleDetailFilterChange = useCallback(
    (filterType: "position" | "tier" | "region" | "patch", value: string) => {
      const normalized = value.trim();
      const currentChamp = selectedChampion;
      const currentLane =
        (championBuild?.active_position as RankedLane | undefined) ??
        currentChamp?.lane ??
        "mid";

      const nextFilters: RankedFilters = {
        ...filters,
        region: filterType === "region" ? normalized || "global" : filters.region,
        tier: filterType === "tier" ? normalized || "emerald_plus" : filters.tier,
        lane:
          filterType === "position" && VALID_LANES.has(normalized)
            ? (normalized as RankedLaneFilter)
            : filters.lane,
        patch:
          filterType === "patch"
            ? normalized || null
            : championBuild?.patch_fallback_applied
            ? null
            : filters.patch,
      };

      persistFilters(nextFilters);
      setFilters(nextFilters);
      setTierError(null);

      if (currentChamp) {
        const nextLane =
          filterType === "position" && VALID_LANES.has(normalized) && normalized !== "all"
            ? (normalized as RankedLane)
            : currentLane;
        const updatedChamp: RankedChampionEntry =
          nextLane !== currentChamp.lane ? { ...currentChamp, lane: nextLane } : currentChamp;
        setSelectedChampion(updatedChamp);
        fetchChampionBuild(updatedChamp, nextLane, nextFilters);
      }
    },
    [selectedChampion, championBuild, filters, fetchChampionBuild, persistFilters]
  );

  const clearSelectedChampion = useCallback(() => {
    buildSeqRef.current += 1;
    setSelectedChampion(null);
    setChampionBuild(null);
    setBuildError(null);
    setBuildLoading(false);
  }, []);

  const retryTierList = useCallback(() => {
    fetchTierList(filters, true);
  }, [filters, fetchTierList]);

  const retryChampionBuild = useCallback(() => {
    if (selectedChampion) {
      fetchChampionBuild(selectedChampion, selectedChampion.lane, filters);
    }
  }, [selectedChampion, filters, fetchChampionBuild]);

  const handlePatchProtectionChange = useCallback(() => {
    const nextFilters: RankedFilters = { ...filters, patch: null };
    persistFilters(nextFilters);
    setFilters(nextFilters);
    loadedCacheKeyRef.current = null;
    setTierError(null);
    if (isActive) {
      fetchTierList(nextFilters, true);
      if (selectedChampion) {
        fetchChampionBuild(selectedChampion, selectedChampion.lane, nextFilters);
      }
    }
  }, [filters, isActive, selectedChampion, fetchTierList, fetchChampionBuild, persistFilters]);

  const orderedSpells = useMemo(
    () =>
      computeOrderedSpells(
        championBuild?.spells,
        null,
        championBuild?.spell_choices,
        flashKey,
        false
      ),
    [championBuild?.spells, championBuild?.spell_choices, flashKey]
  );

  return {
    filters,
    tierData,
    tierLoading,
    tierError,
    selectedChampion,
    championBuild,
    buildLoading,
    buildError,
    selectedRunePageIndex,
    selectedBuildIndex,
    orderedSpells,
    setSelectedRunePageIndex,
    setSelectedBuildIndex,
    handleTierListFilterChange,
    handleDetailFilterChange,
    selectChampion,
    clearSelectedChampion,
    retryTierList,
    retryChampionBuild,
    handlePatchProtectionChange,
  };
}

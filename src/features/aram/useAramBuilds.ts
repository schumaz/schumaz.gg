import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AppLocale, AramChampionRank, AramMode, ChampionBuild, FlashKeyPreference } from "../../types";
import { aramIpc } from "../../services/ipc";
import { translate } from "../../i18n";
import { computeOrderedSpells } from "../../utils/spells";
import { AramClassicBuildFilters, AramMayhemPanel, DEFAULT_ARAM_CLASSIC_FILTERS } from "./aramHelpers";

interface UseAramBuildsParams {
  isActive: boolean;
  savedMode: AramMode | null;
  showChooser: boolean;
  selectedMayhemChampion: AramChampionRank | null;
  flashKey: FlashKeyPreference;
  locale: AppLocale;
}

/**
 * Manages Classic ARAM read-only build detail state and lazy-fetched ARAM Mayhem build previews.
 */
export function useAramBuilds({
  isActive, savedMode, showChooser, selectedMayhemChampion, flashKey, locale,
}: UseAramBuildsParams) {
  const [selectedClassicChampion, setSelectedClassicChampion] = useState<AramChampionRank | null>(null);
  const [classicFilters, setClassicFilters] = useState<AramClassicBuildFilters>(DEFAULT_ARAM_CLASSIC_FILTERS);
  const [classicBuild, setClassicBuild] = useState<ChampionBuild | null>(null);
  const [classicBuildLoading, setClassicBuildLoading] = useState(false);
  const [classicBuildError, setClassicBuildError] = useState<string | null>(null);
  const [classicRunePageIndex, setClassicRunePageIndex] = useState(0);
  const [classicBuildIndex, setClassicBuildIndex] = useState(0);

  const [mayhemPanel, setMayhemPanel] = useState<AramMayhemPanel>("augments");
  const [mayhemBuildsByChampion, setMayhemBuildsByChampion] = useState<Record<string, ChampionBuild>>({});
  const [mayhemBuildLoading, setMayhemBuildLoading] = useState(false);
  const [mayhemBuildError, setMayhemBuildError] = useState<string | null>(null);
  const [mayhemBuildIndex, setMayhemBuildIndex] = useState(0);

  const classicBuildSeqRef = useRef(0);
  const classicFiltersRef = useRef<AramClassicBuildFilters>(classicFilters);
  classicFiltersRef.current = classicFilters;
  const selectedClassicChampionRef = useRef<AramChampionRank | null>(selectedClassicChampion);
  selectedClassicChampionRef.current = selectedClassicChampion;

  const mayhemBuildCacheRef = useRef<Record<string, ChampionBuild>>({});
  const mayhemBuildSeqRef = useRef(0);
  const staleClassicBuildRef = useRef(false);
  const mayhemPanelRef = useRef<AramMayhemPanel>(mayhemPanel);
  mayhemPanelRef.current = mayhemPanel;
  const selectedMayhemChampionRef = useRef<AramChampionRank | null>(selectedMayhemChampion);
  selectedMayhemChampionRef.current = selectedMayhemChampion;
  const prevLocaleRef = useRef<AppLocale>(locale);

  const fetchClassicBuild = useCallback(
    async (champion: AramChampionRank, targetFilters: AramClassicBuildFilters) => {
      staleClassicBuildRef.current = false;
      const seq = ++classicBuildSeqRef.current;
      setClassicBuildLoading(true);
      setClassicBuildError(null);
      try {
        const build = await aramIpc.getAramChampionBuild(
          champion.id, "aram", targetFilters.tier, targetFilters.region, targetFilters.patch
        );
        if (classicBuildSeqRef.current === seq) setClassicBuild(build);
      } catch (err: unknown) {
        if (classicBuildSeqRef.current === seq) {
          setClassicBuildError(typeof err === "string" ? err : translate(locale, "aram.fetchBuildError"));
        }
      } finally {
        if (classicBuildSeqRef.current === seq) setClassicBuildLoading(false);
      }
    },
    [locale]
  );

  const fetchMayhemBuild = useCallback(
    async (champion: AramChampionRank, force = false) => {
      const normalizedKey = champion.key.trim().toLowerCase();
      if (!normalizedKey) return;

      setMayhemBuildError(null);
      if (!force && mayhemBuildCacheRef.current[normalizedKey]) {
        mayhemBuildSeqRef.current += 1;
        setMayhemBuildLoading(false);
        return;
      }

      const seq = ++mayhemBuildSeqRef.current;
      setMayhemBuildLoading(true);
      try {
        const build = await aramIpc.getAramChampionBuild(champion.id, "aram_mayhem", null, "global", null);
        mayhemBuildCacheRef.current[normalizedKey] = build;
        if (mayhemBuildSeqRef.current === seq) {
          setMayhemBuildsByChampion((prev) => ({ ...prev, [normalizedKey]: build }));
        }
      } catch (err: unknown) {
        if (mayhemBuildSeqRef.current === seq) {
          setMayhemBuildError(typeof err === "string" ? err : translate(locale, "aram.fetchBuildError"));
        }
      } finally {
        if (mayhemBuildSeqRef.current === seq) setMayhemBuildLoading(false);
      }
    },
    [locale]
  );

  useEffect(() => {
    setMayhemBuildIndex(0);
  }, [selectedMayhemChampion?.key]);

  useEffect(() => {
    if (!isActive || showChooser) return;
    if (savedMode === "aram" && staleClassicBuildRef.current && selectedClassicChampion) {
      staleClassicBuildRef.current = false;
      void fetchClassicBuild(selectedClassicChampion, classicFiltersRef.current);
    } else if (savedMode === "aram_mayhem" && mayhemPanel === "build" && selectedMayhemChampion) {
      void fetchMayhemBuild(selectedMayhemChampion);
    }
  }, [isActive, showChooser, savedMode, mayhemPanel, selectedMayhemChampion, selectedClassicChampion, fetchClassicBuild, fetchMayhemBuild]);

  useEffect(() => {
    if (prevLocaleRef.current === locale) return;
    prevLocaleRef.current = locale;
    mayhemBuildCacheRef.current = {};
    setMayhemBuildsByChampion({});
    if (selectedClassicChampionRef.current) {
      void fetchClassicBuild(selectedClassicChampionRef.current, classicFiltersRef.current);
    }
    if (mayhemPanelRef.current === "build" && selectedMayhemChampionRef.current) {
      void fetchMayhemBuild(selectedMayhemChampionRef.current, true);
    }
  }, [locale, fetchClassicBuild, fetchMayhemBuild]);

  const selectClassicChampion = useCallback((champ: AramChampionRank) => {
    setSelectedClassicChampion(champ);
    setClassicBuild(null);
    setClassicBuildError(null);
    setClassicRunePageIndex(0);
    setClassicBuildIndex(0);
    void fetchClassicBuild(champ, classicFiltersRef.current);
  }, [fetchClassicBuild]);

  const clearSelectedClassicChampion = useCallback(() => {
    classicBuildSeqRef.current += 1;
    staleClassicBuildRef.current = false;
    setSelectedClassicChampion(null);
    setClassicBuild(null);
    setClassicBuildError(null);
    setClassicBuildLoading(false);
  }, []);

  const invalidateBuildsCache = useCallback(() => {
    mayhemBuildCacheRef.current = {};
    classicBuildSeqRef.current += 1;
    mayhemBuildSeqRef.current += 1;
    staleClassicBuildRef.current = Boolean(selectedClassicChampionRef.current);
    setClassicBuild(null);
    setClassicBuildError(null);
    setMayhemBuildsByChampion({});
    setMayhemBuildError(null);
  }, []);

  const handleClassicDetailFilterChange = useCallback(
    (filterType: "position" | "tier" | "region" | "patch", value: string) => {
      if (filterType === "position") return;
      const trimmed = value.trim();
      const prev = classicFiltersRef.current;
      const nextFilters: AramClassicBuildFilters =
        filterType === "tier" ? { ...prev, tier: trimmed || null }
          : filterType === "region" ? { ...prev, region: trimmed || "global" }
          : { ...prev, patch: trimmed || null };
      classicFiltersRef.current = nextFilters;
      setClassicFilters(nextFilters);
      if (selectedClassicChampionRef.current) {
        void fetchClassicBuild(selectedClassicChampionRef.current, nextFilters);
      }
    },
    [fetchClassicBuild]
  );

  const retryClassicBuild = useCallback(() => {
    if (selectedClassicChampionRef.current) {
      void fetchClassicBuild(selectedClassicChampionRef.current, classicFiltersRef.current);
    }
  }, [fetchClassicBuild]);

  const retryMayhemBuild = useCallback(() => {
    if (selectedMayhemChampionRef.current) {
      void fetchMayhemBuild(selectedMayhemChampionRef.current, true);
    }
  }, [fetchMayhemBuild]);

  const handlePatchProtectionChange = useCallback(() => {
    const nextFilters: AramClassicBuildFilters = { ...classicFiltersRef.current, patch: null };
    classicFiltersRef.current = nextFilters;
    setClassicFilters(nextFilters);
    mayhemBuildCacheRef.current = {};
    setMayhemBuildsByChampion({});
    if (isActive) {
      if (selectedClassicChampionRef.current) {
        void fetchClassicBuild(selectedClassicChampionRef.current, nextFilters);
      }
      if (mayhemPanelRef.current === "build" && selectedMayhemChampionRef.current) {
        void fetchMayhemBuild(selectedMayhemChampionRef.current, true);
      }
    }
  }, [isActive, fetchClassicBuild, fetchMayhemBuild]);

  const classicOrderedSpells = useMemo(
    () => computeOrderedSpells(classicBuild?.spells, 0, classicBuild?.spell_options, flashKey, false),
    [classicBuild?.spells, classicBuild?.spell_options, flashKey]
  );

  const activeMayhemBuild = selectedMayhemChampion
    ? mayhemBuildsByChampion[selectedMayhemChampion.key.trim().toLowerCase()] ?? null
    : null;

  const mayhemOrderedSpells = useMemo(
    () => computeOrderedSpells(activeMayhemBuild?.spells, 0, activeMayhemBuild?.spell_options, flashKey, false),
    [activeMayhemBuild?.spells, activeMayhemBuild?.spell_options, flashKey]
  );

  return {
    selectedClassicChampion, classicBuild, classicBuildLoading, classicBuildError,
    classicRunePageIndex, classicBuildIndex, classicOrderedSpells,
    setClassicRunePageIndex, setClassicBuildIndex, selectClassicChampion,
    clearSelectedClassicChampion, handleClassicDetailFilterChange, retryClassicBuild,
    mayhemPanel, setMayhemPanel, mayhemBuildsByChampion, mayhemBuildLoading,
    mayhemBuildError, mayhemBuildIndex, mayhemOrderedSpells, setMayhemBuildIndex,
    retryMayhemBuild, handlePatchProtectionChange, invalidateBuildsCache,
  };
}

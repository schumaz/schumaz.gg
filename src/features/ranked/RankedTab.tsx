import "./RankedTab.css";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "../../i18n";
import {
  ChampionBuild,
  RankedChampionEntry,
  RankedTierListResponse,
  SpellMetadata,
} from "../../types";
import { getChampionTierLabel } from "../aram/aramHelpers";
import { RankedFilters } from "./rankedStorage";
import {
  getLaneMeta,
  groupByTier,
  normalizeRankedSearch,
} from "./rankedHelpers";
import { RankedToolbarCard } from "./RankedToolbarCard";
import { RankedTierListCard } from "./RankedTierListCard";
import { RankedChampionDetail } from "./RankedChampionDetail";

export * from "./rankedHelpers";

interface RankedTabProps {
  isActive: boolean;
  filters: RankedFilters;
  tierData: RankedTierListResponse | null;
  tierLoading: boolean;
  tierError: string | null;
  selectedChampion: RankedChampionEntry | null;
  championBuild: ChampionBuild | null;
  buildLoading: boolean;
  buildError: string | null;
  orderedSpells: SpellMetadata[];
  selectedRunePageIndex: number;
  selectedBuildIndex: number;
  onTierListFilterChange: (
    filterType: "position" | "tier" | "region" | "patch",
    value: string
  ) => void;
  onDetailFilterChange: (
    filterType: "position" | "tier" | "region" | "patch",
    value: string
  ) => void;
  onSelectChampion: (entry: RankedChampionEntry) => void;
  onBackToTierList: () => void;
  onRetryTierList: () => void;
  onRetryBuild: () => void;
  onSelectRunePage: (pageIndex: number) => void;
  onSelectBuild: (index: number) => void;
  savedChampionTierFilter?: string;
  onChampionTierFilterChange?: (tier: string) => void;
}

export function RankedTab({
  isActive,
  filters,
  tierData,
  tierLoading,
  tierError,
  selectedChampion,
  championBuild,
  buildLoading,
  buildError,
  orderedSpells,
  selectedRunePageIndex,
  selectedBuildIndex,
  onTierListFilterChange,
  onDetailFilterChange,
  onSelectChampion,
  onBackToTierList,
  onRetryTierList,
  onRetryBuild,
  onSelectRunePage,
  onSelectBuild,
  savedChampionTierFilter,
  onChampionTierFilterChange,
}: RankedTabProps) {
  const { t } = useTranslation();
  const [search, setSearch] = useState("");
  const [tierFilter, setTierFilter] = useState<string>(savedChampionTierFilter || "all");

  useEffect(() => {
    if (savedChampionTierFilter) {
      setTierFilter(savedChampionTierFilter);
    }
  }, [savedChampionTierFilter]);

  const handleSelectTierFilter = (next: string) => {
    setTierFilter(next);
    onChampionTierFilterChange?.(next);
  };

  const laneMeta = useMemo(() => getLaneMeta(t), [t]);
  const isAllLanes = filters.lane === "all";

  const filteredChampions = useMemo(() => {
    const raw = tierData?.champions ?? [];
    const query = normalizeRankedSearch(search);

    return raw.filter((c) => {
      if (filters.lane !== "all" && c.lane !== filters.lane) return false;
      if (tierFilter !== "all" && String(Math.max(0, c.tier)) !== tierFilter) return false;
      if (query && !normalizeRankedSearch(c.name).includes(query)) return false;
      return true;
    });
  }, [tierData?.champions, filters.lane, tierFilter, search]);

  const tierGroups = useMemo(() => {
    return groupByTier(filteredChampions, isAllLanes ? "all" : "lane");
  }, [filteredChampions, isAllLanes]);

  const activeLaneMeta = filters.lane !== "all" ? laneMeta[filters.lane] : null;

  const detailStatusTag = selectedChampion
    ? isAllLanes && (selectedChampion.overall_rank ?? 0) > 0
      ? t("ranked.detailStatusGeneral", {
          tier: getChampionTierLabel(selectedChampion.tier),
          rank: selectedChampion.overall_rank ?? 0,
        })
      : t("ranked.detailStatusLane", {
          tier: getChampionTierLabel(selectedChampion.tier),
          rank: selectedChampion.rank,
          lane: laneMeta[selectedChampion.lane].shortLabel,
        })
    : t("ranked.detailStatusFallback");

  return (
    <div
      className={`tab-content ${isActive ? "active-tab" : "inactive-tab"}`}
      data-testid="view-ranked"
      aria-hidden={!isActive}
    >
      {selectedChampion ? (
        <RankedChampionDetail
          selectedChampion={selectedChampion}
          championBuild={championBuild}
          buildLoading={buildLoading}
          buildError={buildError}
          detailStatusTag={detailStatusTag}
          orderedSpells={orderedSpells}
          selectedRunePageIndex={selectedRunePageIndex}
          selectedBuildIndex={selectedBuildIndex}
          onDetailFilterChange={onDetailFilterChange}
          onBackToTierList={onBackToTierList}
          onRetryBuild={onRetryBuild}
          onSelectRunePage={onSelectRunePage}
          onSelectBuild={onSelectBuild}
        />
      ) : (
        <div className="ranked-layout" data-testid="ranked-tierlist-view">
          <RankedToolbarCard
            filters={filters}
            tierData={tierData}
            tierLoading={tierLoading}
            search={search}
            onSearchChange={setSearch}
            tierFilter={tierFilter}
            onSelectTierFilter={handleSelectTierFilter}
            laneMeta={laneMeta}
            onTierListFilterChange={onTierListFilterChange}
          />

          <RankedTierListCard
            tierLoading={tierLoading}
            tierData={tierData}
            tierError={tierError}
            tierGroups={tierGroups}
            search={search}
            tierFilter={tierFilter}
            laneFilter={filters.lane}
            isAllLanes={isAllLanes}
            activeLaneMeta={activeLaneMeta}
            laneMeta={laneMeta}
            onSelectChampion={onSelectChampion}
            onRetryTierList={onRetryTierList}
          />
        </div>
      )}
    </div>
  );
}

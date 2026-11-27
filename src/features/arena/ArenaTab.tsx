import "./ArenaTab.css";
import { memo, useMemo, useState } from "react";
import { useTranslation } from "../../i18n";
import type {
  ArenaChampionEntry,
  ArenaTierListResponse,
  ChampionBuild,
} from "../../types";
import { getChampionTierLabel } from "../aram/aramHelpers";
import {
  ArenaFilters,
  loadSavedArenaTierFilter,
  saveArenaTierFilter,
} from "./arenaStorage";
import { groupArenaByTier, normalizeArenaSearch } from "./arenaHelpers";
import { ArenaToolbarCard } from "./ArenaToolbarCard";
import { ArenaTierListCard } from "./ArenaTierListCard";
import { ArenaChampionDetail } from "./ArenaChampionDetail";

export * from "./arenaHelpers";

interface ArenaTabProps {
  isActive: boolean;
  filters: ArenaFilters;
  tierData: ArenaTierListResponse | null;
  tierLoading: boolean;
  tierError: string | null;
  selectedChampion: ArenaChampionEntry | null;
  championBuild: ChampionBuild | null;
  buildLoading: boolean;
  buildError: string | null;
  selectedBuildIndex: number;
  onPatchChange: (patch: string) => void;
  onDetailFilterChange: (
    filterType: "position" | "tier" | "region" | "patch",
    value: string
  ) => void;
  onSelectChampion: (entry: ArenaChampionEntry) => void;
  onSelectSynergyChampion: (championId: number) => void;
  onBackToTierList: () => void;
  onRetryTierList: () => void;
  onRetryBuild: () => void;
  onSelectBuild: (index: number) => void;
}

/**
 * Top-level Arena (`CHERRY`) tab switching between the tier-grouped ranking list and champion detail.
 */
export const ArenaTab = memo(function ArenaTab({
  isActive,
  filters,
  tierData,
  tierLoading,
  tierError,
  selectedChampion,
  championBuild,
  buildLoading,
  buildError,
  selectedBuildIndex,
  onPatchChange,
  onDetailFilterChange,
  onSelectChampion,
  onSelectSynergyChampion,
  onBackToTierList,
  onRetryTierList,
  onRetryBuild,
  onSelectBuild,
}: ArenaTabProps) {
  const { t } = useTranslation();
  const [search, setSearch] = useState("");
  const [tierFilter, setTierFilter] = useState<string>(() => loadSavedArenaTierFilter());

  const handleSelectTierFilter = (next: string) => {
    setTierFilter(next);
    saveArenaTierFilter(next);
  };

  const filteredChampions = useMemo(() => {
    const raw = tierData?.champions ?? [];
    const query = normalizeArenaSearch(search);

    return raw.filter((c) => {
      if (tierFilter !== "all" && String(Math.max(0, c.tier)) !== tierFilter) return false;
      if (query && !normalizeArenaSearch(c.name).includes(query)) return false;
      return true;
    });
  }, [tierData?.champions, tierFilter, search]);

  const tierGroups = useMemo(() => groupArenaByTier(filteredChampions), [filteredChampions]);

  const detailStatusTag =
    selectedChampion && selectedChampion.rank > 0
      ? t("arenaTab.detailStatus", {
          tier: getChampionTierLabel(selectedChampion.tier),
          rank: selectedChampion.rank,
        })
      : t("arenaTab.detailStatusFallback");

  return (
    <div
      className={`tab-content ${isActive ? "active-tab" : "inactive-tab"}`}
      data-testid="view-arena"
      aria-hidden={!isActive}
    >
      {selectedChampion ? (
        <ArenaChampionDetail
          selectedChampion={selectedChampion}
          championBuild={championBuild}
          buildLoading={buildLoading}
          buildError={buildError}
          detailStatusTag={detailStatusTag}
          selectedBuildIndex={selectedBuildIndex}
          onDetailFilterChange={onDetailFilterChange}
          onBackToTierList={onBackToTierList}
          onRetryBuild={onRetryBuild}
          onSelectBuild={onSelectBuild}
          onSelectSynergyChampion={onSelectSynergyChampion}
        />
      ) : (
        <div className="arena-layout" data-testid="arena-tierlist-view">
          <ArenaToolbarCard
            filters={filters}
            tierData={tierData}
            tierLoading={tierLoading}
            search={search}
            onSearchChange={setSearch}
            tierFilter={tierFilter}
            onSelectTierFilter={handleSelectTierFilter}
            onPatchChange={onPatchChange}
          />

          <ArenaTierListCard
            tierLoading={tierLoading}
            tierData={tierData}
            tierError={tierError}
            tierGroups={tierGroups}
            search={search}
            tierFilter={tierFilter}
            onSelectChampion={onSelectChampion}
            onRetryTierList={onRetryTierList}
          />
        </div>
      )}
    </div>
  );
});

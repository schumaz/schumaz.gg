import "./RankedTab.css";
import { useMemo, useState } from "react";
import {
  ChampionBuild,
  RankedChampionEntry,
  RankedLane,
  RankedTierListResponse,
  SpellMetadata,
} from "../types";
import { RankedFilters } from "../hooks/useRanked";
import { getChampionTierClass, getChampionTierLabel } from "./AramTab";
import { FilterDropdown, FilterDropdownOption } from "./FilterDropdown";
import {
  FiltersHeader,
  REGION_OPTIONS,
  TIER_OPTIONS,
  buildIconImg,
  computePatchUiMeta,
} from "./FiltersHeader";
import { ItemBlocks } from "./ItemBlocks";
import { RuneTree } from "./RuneTree";
import { SearchInput } from "./SearchInput";
import { SpellsBar } from "./SpellsBar";

const CDRAGON_POSITION_SVG_BASE =
  "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/svg";

export const LANE_ORDER: RankedLane[] = ["top", "jungle", "mid", "adc", "support"];

export const LANE_META: Record<RankedLane, { label: string; shortLabel: string; iconUrl: string }> =
  {
    top: {
      label: "Topo",
      shortLabel: "Topo",
      iconUrl: `${CDRAGON_POSITION_SVG_BASE}/position-top-light.svg`,
    },
    jungle: {
      label: "Selva",
      shortLabel: "Selva",
      iconUrl: `${CDRAGON_POSITION_SVG_BASE}/position-jungle-light.svg`,
    },
    mid: {
      label: "Meio",
      shortLabel: "Meio",
      iconUrl: `${CDRAGON_POSITION_SVG_BASE}/position-middle-light.svg`,
    },
    adc: {
      label: "Atirador (Bot)",
      shortLabel: "Atirador",
      iconUrl: `${CDRAGON_POSITION_SVG_BASE}/position-bottom-light.svg`,
    },
    support: {
      label: "Suporte",
      shortLabel: "Suporte",
      iconUrl: `${CDRAGON_POSITION_SVG_BASE}/position-utility-light.svg`,
    },
  };

export const CHAMPION_TIER_FILTER_OPTIONS: FilterDropdownOption[] = [
  {
    value: "all",
    label: "Todos",
    icon: <span className="ranked-tier-filter-dot tier-all" />,
  },
  {
    value: "0",
    label: "OP",
    icon: <span className="ranked-tier-filter-dot tier-op" />,
  },
  {
    value: "1",
    label: "Tier 1",
    icon: <span className="ranked-tier-filter-dot tier-1" />,
  },
  {
    value: "2",
    label: "Tier 2",
    icon: <span className="ranked-tier-filter-dot tier-2" />,
  },
  {
    value: "3",
    label: "Tier 3",
    icon: <span className="ranked-tier-filter-dot tier-3" />,
  },
  {
    value: "4",
    label: "Tier 4",
    icon: <span className="ranked-tier-filter-dot tier-4" />,
  },
  {
    value: "5",
    label: "Tier 5",
    icon: <span className="ranked-tier-filter-dot tier-5" />,
  },
];

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
}

function normalizeSearch(value: string): string {
  return value
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/\s+/g, "");
}

function compareAllLanesEntries(a: RankedChampionEntry, b: RankedChampionEntry): number {
  const aOverall = a.overall_rank ?? 0;
  const bOverall = b.overall_rank ?? 0;
  const aHasOverall = aOverall > 0;
  const bHasOverall = bOverall > 0;
  if (aHasOverall && bHasOverall && aOverall !== bOverall) {
    return aOverall - bOverall;
  }
  if (aHasOverall !== bHasOverall) {
    return aHasOverall ? -1 : 1;
  }

  const aWr = a.win_rate ?? -1;
  const bWr = b.win_rate ?? -1;
  if (aWr !== bWr) return bWr - aWr;

  const aPr = a.pick_rate ?? -1;
  const bPr = b.pick_rate ?? -1;
  if (aPr !== bPr) return bPr - aPr;

  const aValid = a.rank > 0 ? 0 : 1;
  const bValid = b.rank > 0 ? 0 : 1;
  if (aValid !== bValid) return aValid - bValid;
  if (a.rank !== b.rank) return a.rank - b.rank;

  return a.name.localeCompare(b.name);
}

function compareSingleLaneEntries(a: RankedChampionEntry, b: RankedChampionEntry): number {
  const aValid = a.rank > 0 ? 0 : 1;
  const bValid = b.rank > 0 ? 0 : 1;
  if (aValid !== bValid) return aValid - bValid;
  if (a.rank !== b.rank) return a.rank - b.rank;
  return compareAllLanesEntries(a, b);
}

function groupByTier(
  entries: RankedChampionEntry[],
  mode: "all" | "lane"
): Array<{ tier: number; items: RankedChampionEntry[] }> {
  const tierMap = new Map<number, RankedChampionEntry[]>();
  for (const item of entries) {
    const normalizedTier = Math.max(0, item.tier);
    const list = tierMap.get(normalizedTier);
    if (list) {
      list.push(item);
    } else {
      tierMap.set(normalizedTier, [item]);
    }
  }

  return Array.from(tierMap.entries())
    .sort((a, b) => a[0] - b[0])
    .map(([tier, items]) => ({
      tier,
      items: [...items].sort(mode === "all" ? compareAllLanesEntries : compareSingleLaneEntries),
    }));
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
}: RankedTabProps) {
  const [search, setSearch] = useState("");
  const [tierFilter, setTierFilter] = useState<string>("all");
  const [showProtectionTooltip, setShowProtectionTooltip] = useState(false);

  const {
    patchOptions,
    patchDropdownOptions,
    isPatchProtectionActive,
    remainingHoursShort,
    protectionTooltipText,
  } = computePatchUiMeta({
    activePatch: tierData?.active_patch ?? filters.patch,
    availablePatches: tierData?.available_patches,
    patchFallbackApplied: tierData?.patch_fallback_applied,
    remainingHours: tierData?.patch_protection_remaining_hours,
  });

  const activePatchValue = tierData?.active_patch || filters.patch || patchOptions[0] || "";

  const isAllLanes = filters.lane === "all";

  const filteredChampions = useMemo(() => {
    const raw = tierData?.champions ?? [];
    const query = normalizeSearch(search);

    return raw.filter((c) => {
      if (filters.lane !== "all" && c.lane !== filters.lane) {
        return false;
      }
      if (tierFilter !== "all" && String(Math.max(0, c.tier)) !== tierFilter) {
        return false;
      }
      if (query && !normalizeSearch(c.name).includes(query)) {
        return false;
      }
      return true;
    });
  }, [tierData?.champions, filters.lane, tierFilter, search]);

  const tierGroups = useMemo(() => {
    return groupByTier(filteredChampions, isAllLanes ? "all" : "lane");
  }, [filteredChampions, isAllLanes]);

  const activeLaneMeta = filters.lane !== "all" ? LANE_META[filters.lane] : null;

  const detailStatusTag = selectedChampion
    ? isAllLanes && (selectedChampion.overall_rank ?? 0) > 0
      ? `${getChampionTierLabel(selectedChampion.tier)} · #${selectedChampion.overall_rank} Geral`
      : `${getChampionTierLabel(selectedChampion.tier)} · #${selectedChampion.rank} ${LANE_META[selectedChampion.lane].shortLabel}`
    : "Consulta Ranqueada";

  const renderTierGroups = (
    groups: Array<{ tier: number; items: RankedChampionEntry[] }>,
    scopeKey: string
  ) => {
    let runningGlobalRank = 0;
    return groups.map((group) => (
      <div
        key={`${scopeKey}-tier-${group.tier}`}
        className="ranked-tier-group"
        data-testid={`ranked-tier-group-${scopeKey}-${group.tier}`}
      >
        <div className="ranked-tier-group-header">
          <span className={`aram-tier-pill ${getChampionTierClass(group.tier)}`}>
            {getChampionTierLabel(group.tier)}
          </span>
          <span className="ranked-tier-group-count">
            {group.items.length} {group.items.length === 1 ? "campeão" : "campeões"}
          </span>
        </div>

        <div className="ranked-tier-rows">
          {group.items.map((champ) => {
            runningGlobalRank += 1;
            const laneInfo = LANE_META[champ.lane];
            const displayRank = isAllLanes
              ? (champ.overall_rank ?? 0) > 0
                ? champ.overall_rank
                : runningGlobalRank
              : champ.rank;

            return (
              <button
                key={`${champ.lane}-${champ.id}`}
                type="button"
                className="ranked-champ-row"
                onClick={() => onSelectChampion(champ)}
                data-testid={`ranked-champ-${champ.lane}-${champ.key}`}
              >
                <span className="aram-rank-num">
                  {displayRank > 0 ? `#${displayRank}` : "—"}
                </span>
                <div className="dd-icon-frame aram-champ-frame">
                  <img
                    src={champ.icon_url}
                    alt={champ.name}
                    className="aram-champ-avatar"
                    loading="lazy"
                  />
                </div>
                <span className="aram-champ-name">{champ.name}</span>

                <div className="aram-classic-stats">
                  {isAllLanes && (
                    <span
                      className="ranked-champ-lane-pill"
                      title={
                        champ.rank > 0
                          ? `Rank #${champ.rank} em ${laneInfo.shortLabel}`
                          : laneInfo.shortLabel
                      }
                      data-testid={`ranked-champ-lane-${champ.lane}-${champ.key}`}
                    >
                      <span className="ranked-lane-icon-box" aria-hidden="true">
                        {buildIconImg(
                          laneInfo.iconUrl,
                          `ranked-champ-lane-icon lane-${champ.lane}`
                        )}
                      </span>
                      <span className="ranked-champ-lane-text">{laneInfo.shortLabel}</span>
                    </span>
                  )}

                  {champ.win_rate !== null && champ.win_rate !== undefined && (
                    <span className="aram-stat-pill">
                      <span className="aram-stat-label">WR</span>
                      <span
                        className={`aram-stat-val ${
                          champ.win_rate >= 52
                            ? "is-high"
                            : champ.win_rate < 48
                              ? "is-low"
                              : ""
                        }`}
                      >
                        {champ.win_rate.toFixed(1)}%
                      </span>
                    </span>
                  )}
                  {champ.pick_rate !== null && champ.pick_rate !== undefined && (
                    <span className="aram-stat-pill">
                      <span className="aram-stat-label">PR</span>
                      <span className="aram-stat-val">{champ.pick_rate.toFixed(1)}%</span>
                    </span>
                  )}
                  {champ.ban_rate !== null && champ.ban_rate !== undefined && (
                    <span className="aram-stat-pill">
                      <span className="aram-stat-label">BR</span>
                      <span className="aram-stat-val">{champ.ban_rate.toFixed(1)}%</span>
                    </span>
                  )}
                  <span className={`aram-tier-pill ${getChampionTierClass(champ.tier)}`}>
                    {getChampionTierLabel(champ.tier)}
                  </span>
                </div>
              </button>
            );
          })}
        </div>
      </div>
    ));
  };

  return (
    <div
      className={`tab-content ${isActive ? "active-tab" : "inactive-tab"}`}
      data-testid="view-ranked"
      aria-hidden={!isActive}
    >
      {selectedChampion ? (
        <div className="ranked-detail-layout" data-testid="ranked-champion-detail">
          {championBuild ? (
            <div className="dashboard-layout">
              <FiltersHeader
                build={championBuild}
                champSelect={null}
                loading={buildLoading}
                onFilterChange={onDetailFilterChange}
                onBackToTierList={onBackToTierList}
                statusTagText={detailStatusTag}
                queueModeLabel="Summoner's Rift (Ranqueada)"
              />

              <div className="dashboard-split-grid">
                <div className="dashboard-col dashboard-col-left">
                  <RuneTree
                    build={championBuild}
                    selectedRunePageIndex={selectedRunePageIndex}
                    onSelectRunePage={onSelectRunePage}
                    readOnly={true}
                  />

                  <SpellsBar
                    orderedSpells={orderedSpells}
                    spellChoices={championBuild.spell_choices}
                    selectedSecondarySpellId={null}
                    readOnly={true}
                  />
                </div>

                <div className="dashboard-col dashboard-col-right">
                  <ItemBlocks
                    build={championBuild}
                    selectedBuildIndex={selectedBuildIndex}
                    onSelectBuild={onSelectBuild}
                    readOnly={true}
                  />
                </div>
              </div>
            </div>
          ) : (
            <div className="card ranked-detail-state-card">
              <div className="ranked-detail-fallback-top">
                <button
                  type="button"
                  className="ranked-back-btn"
                  data-testid="ranked-back-btn"
                  onClick={onBackToTierList}
                >
                  <svg
                    viewBox="0 0 24 24"
                    width="16"
                    height="16"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2.2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    aria-hidden="true"
                  >
                    <polyline points="15 18 9 12 15 6" />
                  </svg>
                  <span>Voltar</span>
                </button>
                <div className="ranked-detail-champ-summary">
                  <div className="dd-icon-frame aram-champ-frame">
                    <img
                      src={selectedChampion.icon_url}
                      alt={selectedChampion.name}
                      className="aram-champ-avatar"
                    />
                  </div>
                  <span className="ranked-detail-champ-name">{selectedChampion.name}</span>
                </div>
              </div>

              {buildError ? (
                <div className="aram-state-box">
                  <span className="aram-error-text">{buildError}</span>
                  <button
                    type="button"
                    className="aram-retry-btn"
                    onClick={onRetryBuild}
                  >
                    Tentar novamente
                  </button>
                </div>
              ) : (
                <div className="aram-state-box">
                  <div className="spinner aram-spinner" />
                  <span>Carregando runas e build de {selectedChampion.name}...</span>
                </div>
              )}
            </div>
          )}
        </div>
      ) : (
        <div className="ranked-layout" data-testid="ranked-tierlist-view">
          {/* Header card: Big Branding Title + Right Accessories + Elo/Servidor/Patch + Lane & Tier Filters */}
          <section className="card ranked-toolbar-card" data-testid="ranked-filters-card">
            <div className="ranked-toolbar-top">
              <div className="ranked-hero-left">
                <h2 className="page-hero-title">Tier List</h2>
              </div>

              <div className="ranked-hero-right">
                <div className="ranked-hero-badges">
                  {isPatchProtectionActive && (
                    <span
                      className="badge badge-warning badge-stat patch-fallback-badge"
                      title={protectionTooltipText}
                      data-testid="ranked-patch-fallback-badge"
                    >
                      🛡️ Volume Protegido ({activePatchValue})
                      {remainingHoursShort ? ` · ${remainingHoursShort}` : ""}
                    </span>
                  )}
                  <span className="ranked-mode-badge">Ranqueada</span>
                </div>

                <SearchInput
                  value={search}
                  onChange={setSearch}
                  placeholder="Buscar campeão..."
                  ariaLabel="Buscar campeão na Ranqueada"
                  clearAriaLabel="Limpar busca na Ranqueada"
                  testId="ranked-toolbar-search-wrap"
                />
              </div>
            </div>

            <div className="champ-filters-bar ranked-filters-bar" data-testid="ranked-filters-bar">
              <div className="filter-group">
                <label
                  id="ranked-filter-tier-label"
                  htmlFor="ranked-filter-tier"
                  className="filter-label"
                >
                  Elo:
                </label>
                <FilterDropdown
                  id="ranked-filter-tier"
                  labelId="ranked-filter-tier-label"
                  testId="ranked-filter-tier"
                  value={filters.tier}
                  options={TIER_OPTIONS}
                  disabled={tierLoading}
                  onChange={(val) => onTierListFilterChange("tier", val)}
                />
              </div>

              <div className="filter-group">
                <label
                  id="ranked-filter-region-label"
                  htmlFor="ranked-filter-region"
                  className="filter-label"
                >
                  Servidor:
                </label>
                <FilterDropdown
                  id="ranked-filter-region"
                  labelId="ranked-filter-region-label"
                  testId="ranked-filter-region"
                  value={filters.region}
                  options={REGION_OPTIONS}
                  disabled={tierLoading}
                  onChange={(val) => onTierListFilterChange("region", val)}
                />
              </div>

              <div className="filter-group filter-group-patch">
                <label
                  id="ranked-filter-patch-label"
                  htmlFor="ranked-filter-patch"
                  className="filter-label"
                >
                  Patch:
                </label>
                <FilterDropdown
                  id="ranked-filter-patch"
                  labelId="ranked-filter-patch-label"
                  testId="ranked-filter-patch"
                  value={activePatchValue}
                  options={patchDropdownOptions}
                  disabled={tierLoading}
                  onChange={(val) => onTierListFilterChange("patch", val)}
                />

                {isPatchProtectionActive && (
                  <div
                    className="patch-protection-wrap"
                    onMouseEnter={() => setShowProtectionTooltip(true)}
                    onMouseLeave={() => setShowProtectionTooltip(false)}
                  >
                    <button
                      type="button"
                      className="patch-protection-indicator"
                      data-testid="ranked-patch-protection-indicator"
                      aria-label={protectionTooltipText}
                      aria-describedby={
                        showProtectionTooltip ? "ranked-patch-protection-tooltip" : undefined
                      }
                      onFocus={() => setShowProtectionTooltip(true)}
                      onBlur={() => setShowProtectionTooltip(false)}
                    >
                      !
                    </button>
                    {remainingHoursShort && (
                      <span
                        className="patch-protection-hours"
                        data-testid="ranked-patch-protection-hours"
                      >
                        {remainingHoursShort}
                      </span>
                    )}
                    {showProtectionTooltip && (
                      <div
                        id="ranked-patch-protection-tooltip"
                        role="tooltip"
                        className="patch-protection-tooltip"
                        data-testid="ranked-patch-protection-tooltip"
                      >
                        {protectionTooltipText}
                      </div>
                    )}
                  </div>
                )}
              </div>
            </div>

            {/* Lane Separation Tabs + Pipe + Tier Filter Menu */}
            <div className="ranked-lane-tabs-row">
              <div
                className="ranked-lane-tabs"
                role="group"
                aria-label="Filtrar por rota"
                data-testid="ranked-lane-tabs"
              >
                <button
                  type="button"
                  className={`ranked-lane-pill ${filters.lane === "all" ? "active" : ""}`}
                  onClick={() => onTierListFilterChange("position", "all")}
                  data-testid="ranked-lane-tab-all"
                >
                  <span>Todas as Rotas</span>
                </button>
                {LANE_ORDER.map((lane) => {
                  const meta = LANE_META[lane];
                  const isSelected = filters.lane === lane;
                  return (
                    <button
                      key={lane}
                      type="button"
                      className={`ranked-lane-pill ${isSelected ? "active" : ""}`}
                      onClick={() => onTierListFilterChange("position", lane)}
                      data-testid={`ranked-lane-tab-${lane}`}
                    >
                      <span className="ranked-lane-icon-box" aria-hidden="true">
                        {buildIconImg(meta.iconUrl, `ranked-lane-pill-icon lane-${lane}`)}
                      </span>
                      <span>{meta.shortLabel}</span>
                    </button>
                  );
                })}
              </div>

              <span className="ranked-tabs-pipe" aria-hidden="true">
                |
              </span>

              <div className="filter-group ranked-tier-filter-group">
                <label
                  id="ranked-filter-champ-tier-label"
                  htmlFor="ranked-filter-champ-tier"
                  className="filter-label"
                >
                  Tier:
                </label>
                <FilterDropdown
                  id="ranked-filter-champ-tier"
                  labelId="ranked-filter-champ-tier-label"
                  testId="ranked-filter-champ-tier"
                  value={tierFilter}
                  options={CHAMPION_TIER_FILTER_OPTIONS}
                  disabled={tierLoading && !tierData}
                  onChange={setTierFilter}
                />
              </div>
            </div>
          </section>

          {/* Main Tier List Container */}
          <div className="card ranked-list-card">

            {tierLoading && !tierData ? (
              <div className="aram-state-box">
                <div className="spinner aram-spinner" />
                <span>Carregando tier list ranqueada...</span>
              </div>
            ) : tierError && !tierData ? (
              <div className="aram-state-box">
                <span className="aram-error-text">{tierError}</span>
                <button
                  type="button"
                  className="aram-retry-btn"
                  onClick={onRetryTierList}
                >
                  Tentar novamente
                </button>
              </div>
            ) : tierGroups.length === 0 ? (
              <div className="aram-state-box">
                <span>
                  {search.trim()
                    ? `Nenhum campeão encontrado para "${search}".`
                    : tierFilter !== "all"
                      ? "Nenhum campeão encontrado para o tier selecionado."
                      : "Nenhum dado ranqueado disponível para esses filtros."}
                </span>
              </div>
            ) : (
              <div className="ranked-list-scroll" data-testid="ranked-tierlist-scroll">
                {isAllLanes ? (
                  <div
                    className="ranked-tier-groups"
                    data-testid="ranked-all-lanes-list"
                  >
                    {renderTierGroups(tierGroups, "all")}
                  </div>
                ) : (
                  activeLaneMeta && (
                    <section
                      className="ranked-lane-section"
                      data-testid={`ranked-lane-section-${filters.lane}`}
                    >
                      <header className="ranked-lane-section-header">
                        <div className="ranked-lane-section-title-wrap">
                          {buildIconImg(activeLaneMeta.iconUrl, "ranked-lane-section-icon")}
                          <h3 className="ranked-lane-section-title">{activeLaneMeta.label}</h3>
                        </div>
                      </header>

                      <div className="ranked-tier-groups">
                        {renderTierGroups(tierGroups, filters.lane)}
                      </div>
                    </section>
                  )
                )}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

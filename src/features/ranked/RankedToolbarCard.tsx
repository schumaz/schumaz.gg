import { useMemo, useState } from "react";
import { useTranslation } from "../../i18n";
import { RankedLane, RankedTierListResponse } from "../../types";
import { FilterDropdown } from "../../components/ui/FilterDropdown";
import {
  buildIconImg,
  getRegionOptions,
  getTierOptions,
} from "../../components/ui/filterOptions";
import { LoadingBar } from "../../components/ui/LoadingIndicators";
import { computePatchUiMeta } from "../../components/ui/patchUiMeta";
import { SearchInput } from "../../components/ui/SearchInput";
import { RankedFilters } from "./rankedStorage";
import { LANE_ORDER, LaneMetaItem, getChampionTierFilterOptions } from "./rankedHelpers";

interface RankedToolbarCardProps {
  filters: RankedFilters;
  tierData: RankedTierListResponse | null;
  tierLoading: boolean;
  search: string;
  onSearchChange: (val: string) => void;
  tierFilter: string;
  onSelectTierFilter: (val: string) => void;
  laneMeta: Record<RankedLane, LaneMetaItem>;
  onTierListFilterChange: (
    filterType: "position" | "tier" | "region" | "patch",
    value: string
  ) => void;
}

export function RankedToolbarCard({
  filters,
  tierData,
  tierLoading,
  search,
  onSearchChange,
  tierFilter,
  onSelectTierFilter,
  laneMeta,
  onTierListFilterChange,
}: RankedToolbarCardProps) {
  const { t } = useTranslation();
  const [showProtectionTooltip, setShowProtectionTooltip] = useState(false);

  const tierOptions = useMemo(() => getTierOptions(t), [t]);
  const regionOptions = useMemo(() => getRegionOptions(t), [t]);
  const championTierFilterOptions = useMemo(() => getChampionTierFilterOptions(t), [t]);

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
    t,
  });

  const activePatchValue = tierData?.active_patch || filters.patch || patchOptions[0] || "";

  return (
    <section className="card ranked-toolbar-card" data-testid="ranked-filters-card">
      <LoadingBar active={Boolean(tierLoading && tierData)} testId="ranked-loading-bar" />

      <div className="ranked-toolbar-top">
        <div className="ranked-hero-left">
          <h2 className="page-hero-title">{t("ranked.title")}</h2>
        </div>

        <div className="ranked-hero-right">
          <div className="ranked-hero-badges">
            {isPatchProtectionActive && (
              <span
                className="badge badge-warning badge-stat patch-fallback-badge"
                title={protectionTooltipText}
                data-testid="ranked-patch-fallback-badge"
              >
                🛡️ {t("filters.protectedVolume", { patch: activePatchValue })}
                {remainingHoursShort ? ` · ${remainingHoursShort}` : ""}
              </span>
            )}
            <span className="ranked-mode-badge">{t("ranked.modeBadge")}</span>
          </div>

          <SearchInput
            value={search}
            onChange={onSearchChange}
            placeholder={t("common.searchChampionPlaceholder")}
            ariaLabel={t("ranked.searchAria")}
            clearAriaLabel={t("ranked.clearSearchAria")}
            testId="ranked-toolbar-search-wrap"
          />
        </div>
      </div>

      <div className="champ-filters-bar ranked-filters-bar" data-testid="ranked-filters-bar">
        <div className="filter-group">
          <label id="ranked-filter-tier-label" htmlFor="ranked-filter-tier" className="filter-label">
            {t("filters.eloLabel")}
          </label>
          <FilterDropdown
            id="ranked-filter-tier"
            labelId="ranked-filter-tier-label"
            testId="ranked-filter-tier"
            value={filters.tier}
            options={tierOptions}
            disabled={tierLoading}
            onChange={(val) => onTierListFilterChange("tier", val)}
          />
        </div>

        <div className="filter-group">
          <label id="ranked-filter-region-label" htmlFor="ranked-filter-region" className="filter-label">
            {t("filters.regionLabel")}
          </label>
          <FilterDropdown
            id="ranked-filter-region"
            labelId="ranked-filter-region-label"
            testId="ranked-filter-region"
            value={filters.region}
            options={regionOptions}
            disabled={tierLoading}
            onChange={(val) => onTierListFilterChange("region", val)}
          />
        </div>

        <div className="filter-group filter-group-patch">
          <label id="ranked-filter-patch-label" htmlFor="ranked-filter-patch" className="filter-label">
            {t("filters.patchLabel")}
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
                aria-describedby={showProtectionTooltip ? "ranked-patch-protection-tooltip" : undefined}
                onFocus={() => setShowProtectionTooltip(true)}
                onBlur={() => setShowProtectionTooltip(false)}
              >
                !
              </button>
              {remainingHoursShort && (
                <span className="patch-protection-hours" data-testid="ranked-patch-protection-hours">
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

      <div className="ranked-lane-tabs-row">
        <div
          className="ranked-lane-tabs"
          role="group"
          aria-label={t("ranked.filterByLaneAria")}
          data-testid="ranked-lane-tabs"
        >
          <button
            type="button"
            className={`ranked-lane-pill ${filters.lane === "all" ? "active" : ""}`}
            onClick={() => onTierListFilterChange("position", "all")}
            data-testid="ranked-lane-tab-all"
          >
            <span>{t("ranked.allLanes")}</span>
          </button>
          {LANE_ORDER.map((lane) => {
            const meta = laneMeta[lane];
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

        <span className="ranked-tabs-pipe" aria-hidden="true">|</span>

        <div className="filter-group ranked-tier-filter-group">
          <label
            id="ranked-filter-champ-tier-label"
            htmlFor="ranked-filter-champ-tier"
            className="filter-label"
          >
            {t("filters.tierLabel")}
          </label>
          <FilterDropdown
            id="ranked-filter-champ-tier"
            labelId="ranked-filter-champ-tier-label"
            testId="ranked-filter-champ-tier"
            value={tierFilter}
            options={championTierFilterOptions}
            disabled={tierLoading && !tierData}
            onChange={onSelectTierFilter}
          />
        </div>
      </div>
    </section>
  );
}

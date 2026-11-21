import { useMemo, useState } from "react";
import { useTranslation } from "../../i18n";
import type { ArenaTierListResponse } from "../../types";
import { FilterDropdown, FilterDropdownOption } from "../../components/ui/FilterDropdown";
import { LoadingBar } from "../../components/ui/LoadingIndicators";
import { computePatchUiMeta } from "../../components/ui/patchUiMeta";
import { SearchInput } from "../../components/ui/SearchInput";
import type { ArenaFilters } from "./arenaStorage";

interface ArenaToolbarCardProps {
  filters: ArenaFilters;
  tierData: ArenaTierListResponse | null;
  tierLoading: boolean;
  search: string;
  onSearchChange: (val: string) => void;
  tierFilter: string;
  onSelectTierFilter: (val: string) => void;
  onPatchChange: (patch: string) => void;
}

/**
 * Top toolbar card for the Arena (`3x6`) tier list with search, champion tier filter, and patch controls.
 */
export function ArenaToolbarCard({
  filters,
  tierData,
  tierLoading,
  search,
  onSearchChange,
  tierFilter,
  onSelectTierFilter,
  onPatchChange,
}: ArenaToolbarCardProps) {
  const { t } = useTranslation();
  const [showProtectionTooltip, setShowProtectionTooltip] = useState(false);

  const championTierFilterOptions = useMemo<FilterDropdownOption[]>(
    () => [
      {
        value: "all",
        label: t("arenaTab.allTiers"),
        icon: <span className="arena-tier-filter-dot tier-all" />,
      },
      {
        value: "0",
        label: "OP",
        icon: <span className="arena-tier-filter-dot tier-op" />,
      },
      {
        value: "1",
        label: "Tier 1",
        icon: <span className="arena-tier-filter-dot tier-1" />,
      },
      {
        value: "2",
        label: "Tier 2",
        icon: <span className="arena-tier-filter-dot tier-2" />,
      },
      {
        value: "3",
        label: "Tier 3",
        icon: <span className="arena-tier-filter-dot tier-3" />,
      },
      {
        value: "4",
        label: "Tier 4",
        icon: <span className="arena-tier-filter-dot tier-4" />,
      },
    ],
    [t]
  );

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
    <section className="card arena-toolbar-card" data-testid="arena-filters-card">
      <LoadingBar active={Boolean(tierLoading && tierData)} testId="arena-loading-bar" />

      <div className="arena-toolbar-top">
        <div className="arena-hero-left">
          <h2 className="page-hero-title">{t("arenaTab.title")}</h2>
        </div>

        <div className="arena-hero-right">
          <div className="arena-hero-badges">
            {isPatchProtectionActive && (
              <span
                className="badge badge-warning badge-stat patch-fallback-badge"
                title={protectionTooltipText}
                data-testid="arena-patch-fallback-badge"
              >
                🛡️ {t("filters.protectedVolume", { patch: activePatchValue })}
                {remainingHoursShort ? ` · ${remainingHoursShort}` : ""}
              </span>
            )}
            <span className="arena-mode-badge">{t("arenaTab.modeBadge")}</span>
          </div>

          <SearchInput
            value={search}
            onChange={onSearchChange}
            placeholder={t("common.searchChampionPlaceholder")}
            ariaLabel={t("arenaTab.searchAria")}
            clearAriaLabel={t("arenaTab.clearSearchAria")}
            testId="arena-toolbar-search-wrap"
          />
        </div>
      </div>

      <div className="champ-filters-bar arena-filters-bar" data-testid="arena-filters-bar">
        <div className="filter-group">
          <label
            id="arena-filter-champ-tier-label"
            htmlFor="arena-filter-champ-tier"
            className="filter-label"
          >
            {t("filters.tierLabel")}
          </label>
          <FilterDropdown
            id="arena-filter-champ-tier"
            labelId="arena-filter-champ-tier-label"
            testId="arena-filter-champ-tier"
            value={tierFilter}
            options={championTierFilterOptions}
            disabled={tierLoading && !tierData}
            onChange={onSelectTierFilter}
          />
        </div>

        <div className="filter-group filter-group-patch">
          <label id="arena-filter-patch-label" htmlFor="arena-filter-patch" className="filter-label">
            {t("filters.patchLabel")}
          </label>
          <FilterDropdown
            id="arena-filter-patch"
            labelId="arena-filter-patch-label"
            testId="arena-filter-patch"
            value={activePatchValue}
            options={patchDropdownOptions}
            disabled={tierLoading}
            onChange={onPatchChange}
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
                data-testid="arena-patch-protection-indicator"
                aria-label={protectionTooltipText}
                aria-describedby={showProtectionTooltip ? "arena-patch-protection-tooltip" : undefined}
                onFocus={() => setShowProtectionTooltip(true)}
                onBlur={() => setShowProtectionTooltip(false)}
              >
                !
              </button>
              {remainingHoursShort && (
                <span className="patch-protection-hours" data-testid="arena-patch-protection-hours">
                  {remainingHoursShort}
                </span>
              )}
              {showProtectionTooltip && (
                <div
                  id="arena-patch-protection-tooltip"
                  role="tooltip"
                  className="patch-protection-tooltip"
                  data-testid="arena-patch-protection-tooltip"
                >
                  {protectionTooltipText}
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </section>
  );
}

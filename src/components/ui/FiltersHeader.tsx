import { useState } from "react";
import { useTranslation } from "../../i18n";
import { ChampionBuild, ChampSelectedPayload } from "../../types";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { FilterDropdown } from "./FilterDropdown";
import { LoadingBar } from "./LoadingIndicators";
import { getPositionOptions, getRegionOptions, getTierOptions } from "./filterOptions";
import { computePatchUiMeta } from "./patchUiMeta";

export * from "./filterOptions";
export * from "./patchUiMeta";

interface FiltersHeaderProps {
  build: ChampionBuild;
  champSelect: ChampSelectedPayload | null;
  loading: boolean;
  onFilterChange: (
    filterType: "position" | "tier" | "region" | "patch",
    value: string
  ) => void;
  onBackToTierList?: () => void;
  statusTagText?: string;
  queueModeLabel?: string;
}

export function FiltersHeader({
  build,
  champSelect,
  loading,
  onFilterChange,
  onBackToTierList,
  statusTagText,
  queueModeLabel,
}: FiltersHeaderProps) {
  const { t } = useTranslation();
  const [showProtectionTooltip, setShowProtectionTooltip] = useState(false);

  const isAram =
    champSelect?.queue_mode === "Aram" ||
    champSelect?.queue_mode === "AramMayhem" ||
    champSelect?.queue_id === 450 ||
    champSelect?.queue_id === 2400 ||
    build.supports_runes === false;

  const tierOptions = getTierOptions(t);
  const positionOptions = getPositionOptions(t);
  const regionOptions = getRegionOptions(t);

  const {
    patchOptions,
    patchDropdownOptions,
    isPatchProtectionActive,
    remainingHoursShort,
    protectionTooltipText,
  } = computePatchUiMeta({
    activePatch: build?.active_patch,
    availablePatches: build?.available_patches,
    patchFallbackApplied: build?.patch_fallback_applied,
    remainingHours: build?.patch_protection_remaining_hours,
    t,
  });

  const resolvedTagText =
    statusTagText ?? (champSelect?.locked_in ? t("filters.lockedIn") : t("filters.hovering"));
  const resolvedTagClass = statusTagText
    ? "tag hovering"
    : `tag ${champSelect?.locked_in ? "locked" : "hovering"}`;

  const resolvedQueueLabel =
    queueModeLabel ??
    (champSelect?.queue_mode === "AramMayhem"
      ? t("filters.queueAramMayhem")
      : champSelect?.queue_mode === "Aram"
      ? t("filters.queueAram")
      : champSelect?.queue_mode === "PracticeTool"
      ? t("filters.queuePracticeTool")
      : t("filters.queueSr"));

  return (
    <section className="card champ-hero-card" data-testid="champ-hero">
      <LoadingBar active={loading} testId="filters-loading-bar" />

      <div className="champ-hero-top">
        <div className="champ-hero-left">
          {onBackToTierList && (
            <button
              type="button"
              className="ranked-back-btn"
              data-testid="ranked-back-btn"
              onClick={onBackToTierList}
              title={t("filters.backToTierListTitle")}
            >
              <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <polyline points="15 18 9 12 15 6" />
              </svg>
              <span>{t("common.back")}</span>
            </button>
          )}

          {build.champion_icon_url ? (
            <div className="dd-icon-frame champ-avatar-frame">
              <img
                src={resolveCachedAssetUrl(build.champion_icon_url)}
                alt={build.champion_name}
                className="champ-avatar"
                onError={(e) => {
                  (e.target as HTMLElement).style.display = "none";
                }}
              />
            </div>
          ) : (
            <div className="champ-avatar-placeholder">
              {build.champion_name.slice(0, 2).toUpperCase()}
            </div>
          )}

          <h2 className="champ-name" title={build.champion_name}>
            {build.champion_name}
          </h2>
        </div>

        <div className="champ-hero-right">
          <span className={resolvedTagClass}>{resolvedTagText}</span>

          <div className="champ-meta-row">
            <span className="badge-meta">{resolvedQueueLabel}</span>
            {(build.pick_rate ?? 0) > 0 && (
              <span className="badge badge-neutral badge-stat">
                {t("filters.pickRate", { rate: (build.pick_rate ?? 0).toFixed(1) })}
              </span>
            )}
            {isPatchProtectionActive && (
              <span
                className="badge badge-warning badge-stat patch-fallback-badge"
                title={protectionTooltipText}
                data-testid="patch-fallback-badge"
              >
                🛡️ {t("filters.protectedVolume", { patch: build.active_patch ?? "" })}
                {remainingHoursShort ? ` · ${remainingHoursShort}` : ""}
              </span>
            )}
          </div>
        </div>
      </div>

      <div className="champ-filters-bar" data-testid="champ-filters-bar">
        <div className="filter-group">
          <label id="filter-tier-label" htmlFor="filter-tier" className="filter-label">
            {t("filters.eloLabel")}
          </label>
          <FilterDropdown
            id="filter-tier"
            labelId="filter-tier-label"
            testId="filter-tier"
            value={build.active_tier || "emerald_plus"}
            options={tierOptions}
            disabled={loading}
            onChange={(val) => onFilterChange("tier", val)}
          />
        </div>

        {!isAram && (
          <div className="filter-group">
            <label id="filter-position-label" htmlFor="filter-position" className="filter-label">
              {t("filters.laneLabel")}
            </label>
            <FilterDropdown
              id="filter-position"
              labelId="filter-position-label"
              testId="filter-position"
              value={build.active_position || "mid"}
              options={positionOptions}
              disabled={loading}
              onChange={(val) => onFilterChange("position", val)}
            />
          </div>
        )}

        <div className="filter-group">
          <label id="filter-region-label" htmlFor="filter-region" className="filter-label">
            {t("filters.regionLabel")}
          </label>
          <FilterDropdown
            id="filter-region"
            labelId="filter-region-label"
            testId="filter-region"
            value={build.active_region || "global"}
            options={regionOptions}
            disabled={loading}
            onChange={(val) => onFilterChange("region", val)}
          />
        </div>

        <div className="filter-group filter-group-patch">
          <label id="filter-patch-label" htmlFor="filter-patch" className="filter-label">
            {t("filters.patchLabel")}
          </label>
          <FilterDropdown
            id="filter-patch"
            labelId="filter-patch-label"
            testId="filter-patch"
            value={build.active_patch || patchOptions[0] || ""}
            options={patchDropdownOptions}
            disabled={loading}
            onChange={(val) => onFilterChange("patch", val)}
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
                data-testid="patch-protection-indicator"
                aria-label={protectionTooltipText}
                aria-describedby={showProtectionTooltip ? "patch-protection-tooltip" : undefined}
                onFocus={() => setShowProtectionTooltip(true)}
                onBlur={() => setShowProtectionTooltip(false)}
              >
                !
              </button>
              {remainingHoursShort && (
                <span className="patch-protection-hours" data-testid="patch-protection-hours">
                  {remainingHoursShort}
                </span>
              )}
              {showProtectionTooltip && (
                <div id="patch-protection-tooltip" role="tooltip" className="patch-protection-tooltip" data-testid="patch-protection-tooltip">
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

import { useTranslation } from "../../i18n";
import { ChampionBuild, ChampSelectedPayload } from "../../types";
import { ChampHeroTop, resolveChampHeroQueueLabel } from "./ChampHeroTop";
import { FilterDropdown } from "./FilterDropdown";
import { LoadingBar } from "./LoadingIndicators";
import { getPositionOptions, getRegionOptions, getTierOptions } from "./filterOptions";
import { LaneMatchupsPopover } from "./LaneMatchupsPopover";
import { PatchProtectionIndicator } from "./PatchProtectionIndicator";
import { computePatchUiMeta } from "./patchUiMeta";
import { isAramBuildContextMode, isArenaBuildContext } from "./queueContext";
import { SkillOrderPopover } from "./SkillOrderPopover";

export * from "./filterOptions";
export * from "./patchUiMeta";

interface FiltersHeaderProps {
  build: ChampionBuild;
  champSelect: ChampSelectedPayload | null;
  loading: boolean;
  onFilterChange: (filterType: "position" | "tier" | "region" | "patch", value: string) => void;
  onBackToTierList?: () => void;
  backButtonTitle?: string;
  statusTagText?: string;
  queueModeLabel?: string;
  isAramMode?: boolean;
  readOnly?: boolean;
  onSelectMatchupChampion?: (championId: number) => void;
}

export function FiltersHeader({
  build,
  champSelect,
  loading,
  onFilterChange,
  onBackToTierList,
  backButtonTitle,
  statusTagText,
  queueModeLabel,
  isAramMode = false,
  readOnly = false,
  onSelectMatchupChampion,
}: FiltersHeaderProps) {
  const { t } = useTranslation();

  const isArena = isArenaBuildContext(build, champSelect);
  const isAram = isAramMode || isAramBuildContextMode(build, champSelect);
  const tierOptions = getTierOptions(t);
  const positionOptions = getPositionOptions(t);
  const regionOptions = getRegionOptions(t);

  const { patchOptions, patchDropdownOptions, isPatchProtectionActive, remainingHoursShort, protectionTooltipText } =
    computePatchUiMeta({
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
  const resolvedQueueLabel = resolveChampHeroQueueLabel(t, isArena, champSelect, queueModeLabel);

  const activeLaneValue = (build.active_position || "").trim().toLowerCase();
  const activeLaneLabel =
    positionOptions.find((opt) => opt.value === activeLaneValue)?.label ?? undefined;
  const resetKey = `${build.champion_id}-${build.active_position ?? ""}-${build.active_patch ?? ""}-${build.active_tier ?? ""}-${build.active_region ?? ""}`;

  return (
    <section className="card champ-hero-card" data-testid="champ-hero">
      <LoadingBar active={loading} testId="filters-loading-bar" />

      <ChampHeroTop
        build={build}
        isArena={isArena}
        resolvedTagClass={resolvedTagClass}
        resolvedTagText={resolvedTagText}
        resolvedQueueLabel={resolvedQueueLabel}
        isPatchProtectionActive={isPatchProtectionActive}
        remainingHoursShort={remainingHoursShort}
        protectionTooltipText={protectionTooltipText}
        onBackToTierList={onBackToTierList}
        backButtonTitle={backButtonTitle}
      />

      <div className="champ-filters-bar" data-testid="champ-filters-bar">
        {!isArena && (
          <div className="filter-group">
            <label id="filter-tier-label" htmlFor="filter-tier" className="filter-label">
              {t("filters.eloLabel")}
            </label>
            <FilterDropdown
              id="filter-tier"
              labelId="filter-tier-label"
              testId="filter-tier"
              value={build.active_tier || (isAram ? "all" : "emerald_plus")}
              options={tierOptions}
              disabled={loading}
              onChange={(val) => onFilterChange("tier", val)}
            />
          </div>
        )}

        {!isAram && !isArena && (
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

        {!isArena && (
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
        )}

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
            <PatchProtectionIndicator
              remainingHoursShort={remainingHoursShort}
              tooltipText={protectionTooltipText}
            />
          )}
        </div>

        <div className="champ-filters-actions">
          {!isArena && !isAram && build.lane_matchups && (
            <LaneMatchupsPopover
              matchups={build.lane_matchups}
              championName={build.champion_name}
              laneLabel={activeLaneLabel}
              onSelectChampion={onSelectMatchupChampion}
              resetKey={resetKey}
            />
          )}

          {!isArena && build.skill_order && (
            <SkillOrderPopover
              skillOrder={build.skill_order}
              readOnly={readOnly}
              resetKey={`${build.champion_id}-${build.active_position ?? ""}-${build.active_patch ?? ""}`}
            />
          )}
        </div>
      </div>
    </section>
  );
}

import { useState } from "react";
import { TranslationKey, TranslationParams, translate, useTranslation } from "../i18n";
import { ChampionBuild, ChampSelectedPayload } from "../types";
import { resolveCachedAssetUrl } from "../utils/assets";
import { FilterDropdown, FilterDropdownOption } from "./ui/FilterDropdown";
import { LoadingBar } from "./ui/LoadingIndicators";
import { RegionFlag } from "./ui/RegionFlag";

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

type TranslatorFn = (key: TranslationKey, params?: TranslationParams) => string;

const defaultTranslate: TranslatorFn = (key, params) => translate("pt_BR", key, params);

const CDRAGON_MINI_CRESTS_BASE =
  "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/images/ranked-mini-crests";
const CDRAGON_POSITION_SVG_BASE =
  "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/svg";

export function buildIconImg(src: string, className = "filter-option-icon") {
  return (
    <img
      src={resolveCachedAssetUrl(src)}
      alt=""
      aria-hidden="true"
      className={className}
      onError={(e) => {
        (e.currentTarget as HTMLImageElement).style.display = "none";
      }}
    />
  );
}

export function getTierOptions(t: TranslatorFn = defaultTranslate): FilterDropdownOption[] {
  return [
    {
      value: "all",
      label: t("filters.tiers.all"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/unranked.svg`),
    },
    {
      value: "challenger",
      label: t("filters.tiers.challenger"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/challenger.svg`),
    },
    {
      value: "grandmaster",
      label: t("filters.tiers.grandmaster"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/grandmaster.svg`),
    },
    {
      value: "master_plus",
      label: t("filters.tiers.master_plus"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/master.svg`),
    },
    {
      value: "diamond_plus",
      label: t("filters.tiers.diamond_plus"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/diamond.svg`),
    },
    {
      value: "emerald_plus",
      label: t("filters.tiers.emerald_plus"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/emerald.svg`),
    },
    {
      value: "platinum_plus",
      label: t("filters.tiers.platinum_plus"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/platinum.svg`),
    },
    {
      value: "gold_plus",
      label: t("filters.tiers.gold_plus"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/gold.svg`),
    },
    {
      value: "silver",
      label: t("filters.tiers.silver"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/silver.svg`),
    },
    {
      value: "bronze",
      label: t("filters.tiers.bronze"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/bronze.svg`),
    },
    {
      value: "iron",
      label: t("filters.tiers.iron"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/iron.svg`),
    },
  ];
}

export function getPositionOptions(t: TranslatorFn = defaultTranslate): FilterDropdownOption[] {
  return [
    {
      value: "top",
      label: t("filters.lanes.top"),
      icon: buildIconImg(`${CDRAGON_POSITION_SVG_BASE}/position-top-light.svg`),
    },
    {
      value: "jungle",
      label: t("filters.lanes.jungle"),
      icon: buildIconImg(`${CDRAGON_POSITION_SVG_BASE}/position-jungle-light.svg`),
    },
    {
      value: "mid",
      label: t("filters.lanes.mid"),
      icon: buildIconImg(`${CDRAGON_POSITION_SVG_BASE}/position-middle-light.svg`),
    },
    {
      value: "adc",
      label: t("filters.lanes.adc"),
      icon: buildIconImg(`${CDRAGON_POSITION_SVG_BASE}/position-bottom-light.svg`),
    },
    {
      value: "support",
      label: t("filters.lanes.support"),
      icon: buildIconImg(`${CDRAGON_POSITION_SVG_BASE}/position-utility-light.svg`),
    },
  ];
}

export function getRegionOptions(t: TranslatorFn = defaultTranslate): FilterDropdownOption[] {
  return [
    {
      value: "global",
      label: t("filters.regions.globalLabel"),
      title: t("filters.regions.globalTitle"),
      icon: <RegionFlag region="global" />,
    },
    {
      value: "br",
      label: "BR",
      title: t("filters.regions.brTitle"),
      icon: <RegionFlag region="br" />,
    },
    {
      value: "kr",
      label: "KR",
      title: t("filters.regions.krTitle"),
      icon: <RegionFlag region="kr" />,
    },
    {
      value: "na",
      label: "NA",
      title: t("filters.regions.naTitle"),
      icon: <RegionFlag region="na" />,
    },
    {
      value: "euw",
      label: "EUW",
      title: t("filters.regions.euwTitle"),
      icon: <RegionFlag region="euw" />,
    },
    {
      value: "eune",
      label: "EUNE",
      title: t("filters.regions.euneTitle"),
      icon: <RegionFlag region="eune" />,
    },
    {
      value: "lan",
      label: "LAN",
      title: t("filters.regions.lanTitle"),
      icon: <RegionFlag region="lan" />,
    },
    {
      value: "las",
      label: "LAS",
      title: t("filters.regions.lasTitle"),
      icon: <RegionFlag region="las" />,
    },
    {
      value: "oce",
      label: "OCE",
      title: t("filters.regions.oceTitle"),
      icon: <RegionFlag region="oce" />,
    },
    {
      value: "tr",
      label: "TR",
      title: t("filters.regions.trTitle"),
      icon: <RegionFlag region="tr" />,
    },
    {
      value: "ru",
      label: "RU",
      title: t("filters.regions.ruTitle"),
      icon: <RegionFlag region="ru" />,
    },
    {
      value: "jp",
      label: "JP",
      title: t("filters.regions.jpTitle"),
      icon: <RegionFlag region="jp" />,
    },
    {
      value: "me",
      label: "ME",
      title: t("filters.regions.meTitle"),
      icon: <RegionFlag region="me" />,
    },
    {
      value: "sg",
      label: "SG",
      title: t("filters.regions.sgTitle"),
      icon: <RegionFlag region="sg" />,
    },
    {
      value: "tw",
      label: "TW",
      title: t("filters.regions.twTitle"),
      icon: <RegionFlag region="tw" />,
    },
    {
      value: "vn",
      label: "VN",
      title: t("filters.regions.vnTitle"),
      icon: <RegionFlag region="vn" />,
    },
  ];
}

export const TIER_OPTIONS: FilterDropdownOption[] = getTierOptions();
export const POSITION_OPTIONS: FilterDropdownOption[] = getPositionOptions();
export const REGION_OPTIONS: FilterDropdownOption[] = getRegionOptions();

export function computePatchUiMeta(params: {
  activePatch?: string | null;
  availablePatches?: string[] | null;
  patchFallbackApplied?: boolean | null;
  remainingHours?: number | null;
  t?: TranslatorFn;
}) {
  const t = params.t ?? defaultTranslate;
  const patchOptions =
    params.availablePatches && params.availablePatches.length > 0
      ? params.availablePatches
      : params.activePatch
      ? [params.activePatch]
      : ["16.22", "16.21"];

  const patchDropdownOptions: FilterDropdownOption[] = patchOptions.map((p, idx) => ({
    value: p,
    label:
      idx === 0
        ? t("filters.latestPatch", { patch: p })
        : t("filters.patchOption", { patch: p }),
  }));

  const hasKnownLatest = Boolean(params.availablePatches && params.availablePatches.length > 0);
  const latestPatch = patchOptions[0];
  const isPatchProtectionActive = Boolean(
    params.patchFallbackApplied && (!hasKnownLatest || params.activePatch !== latestPatch)
  );

  const remainingHours =
    isPatchProtectionActive &&
    typeof params.remainingHours === "number" &&
    params.remainingHours > 0
      ? params.remainingHours
      : null;

  const remainingHoursShort =
    remainingHours !== null
      ? remainingHours === 1
        ? t("filters.remaining1h")
        : t("filters.remainingNh", { hours: remainingHours })
      : null;

  const protectionTooltipText =
    remainingHours !== null
      ? remainingHours === 1
        ? t("filters.protectionTooltipWithHours1")
        : t("filters.protectionTooltipWithHoursN", { hours: remainingHours })
      : t("filters.protectionTooltipBase");

  return {
    patchOptions,
    patchDropdownOptions,
    isPatchProtectionActive,
    remainingHoursShort,
    protectionTooltipText,
  };
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

      {/* Filters: Elo Base, Rota (SR only), Servidor, Patch */}
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
                <div
                  id="patch-protection-tooltip"
                  role="tooltip"
                  className="patch-protection-tooltip"
                  data-testid="patch-protection-tooltip"
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

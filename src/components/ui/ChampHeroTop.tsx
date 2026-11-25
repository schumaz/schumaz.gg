import { useTranslation } from "../../i18n";
import { ChampionBuild, ChampSelectedPayload } from "../../types";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { getChampionTierClass, getChampionTierLabel } from "./championTier";

interface ChampHeroTopProps {
  build: ChampionBuild;
  isArena: boolean;
  resolvedTagClass: string;
  resolvedTagText: string;
  resolvedQueueLabel: string;
  isPatchProtectionActive: boolean;
  remainingHoursShort: string | null;
  protectionTooltipText: string;
  onBackToTierList?: () => void;
  backButtonTitle?: string;
}

/**
 * Hero summary row inside `FiltersHeader` showing back navigation, champion avatar/name/tier,
 * status tag, queue badge, and key meta rates.
 */
export function ChampHeroTop({
  build,
  isArena,
  resolvedTagClass,
  resolvedTagText,
  resolvedQueueLabel,
  isPatchProtectionActive,
  remainingHoursShort,
  protectionTooltipText,
  onBackToTierList,
  backButtonTitle,
}: ChampHeroTopProps) {
  const { t } = useTranslation();

  return (
    <div className="champ-hero-top">
      <div className="champ-hero-left">
        {onBackToTierList && (
          <button
            type="button"
            className="ranked-back-btn"
            data-testid="ranked-back-btn"
            onClick={onBackToTierList}
            title={backButtonTitle ?? t("filters.backToTierListTitle")}
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

        {typeof build.tier === "number" && (
          <span
            className={`aram-tier-pill champ-hero-tier-pill ${getChampionTierClass(build.tier)}`}
            data-testid="champ-tier-badge"
          >
            {getChampionTierLabel(build.tier)}
          </span>
        )}
      </div>

      <div className="champ-hero-right">
        <span className={resolvedTagClass}>{resolvedTagText}</span>

        <div className="champ-meta-row">
          <span className="badge-meta">{resolvedQueueLabel}</span>
          {isArena && (build.win_rate ?? 0) > 0 && (
            <span className="badge badge-neutral badge-stat">
              {t("filters.top3Rate", { rate: (build.win_rate ?? 0).toFixed(1) })}
            </span>
          )}
          {isArena && (build.first_place_rate ?? 0) > 0 && (
            <span className="badge badge-neutral badge-stat">
              {t("filters.firstPlaceRate", { rate: (build.first_place_rate ?? 0).toFixed(1) })}
            </span>
          )}
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
  );
}

export function resolveChampHeroQueueLabel(
  t: ReturnType<typeof useTranslation>["t"],
  isArena: boolean,
  champSelect: ChampSelectedPayload | null,
  queueModeLabel?: string
): string {
  return (
    queueModeLabel ??
    (isArena
      ? t("filters.queueArena")
      : champSelect?.queue_mode === "AramMayhem"
      ? t("filters.queueAramMayhem")
      : champSelect?.queue_mode === "Aram"
      ? t("filters.queueAram")
      : champSelect?.queue_mode === "PracticeTool"
      ? t("filters.queuePracticeTool")
      : champSelect?.queue_id === 480
      ? t("filters.queueSwiftplay")
      : champSelect?.queue_id === 490
      ? t("filters.queueQuickplay")
      : champSelect?.queue_id === 700
      ? t("filters.queueClash")
      : t("filters.queueSr"))
  );
}

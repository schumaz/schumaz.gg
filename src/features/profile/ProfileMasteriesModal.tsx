import { useEffect, useMemo, useState } from "react";
import { createPortal } from "react-dom";
import { ChampionMasteryInfo } from "../../types";
import { useTranslation } from "../../i18n";
import { SearchInput } from "../../components/ui";
import { resolveCachedAssetUrl } from "../../utils/assets";
import {
  computeMasteryProgressPercent,
  computeMasterySummary,
  filterAndSortMasteries,
  MasterySortKey,
  ProfileChampionTarget,
} from "./profileHelpers";

interface ProfileMasteriesModalProps {
  masteries: ChampionMasteryInfo[];
  onClose: () => void;
  onSelectChampion: (target: ProfileChampionTarget) => void;
}

const SORT_OPTIONS: ReadonlyArray<{
  key: MasterySortKey;
  labelKey:
    | "profile.masteriesSortPoints"
    | "profile.masteriesSortLevel"
    | "profile.masteriesSortRecent"
    | "profile.masteriesSortName";
}> = [
  { key: "points", labelKey: "profile.masteriesSortPoints" },
  { key: "level", labelKey: "profile.masteriesSortLevel" },
  { key: "recent", labelKey: "profile.masteriesSortRecent" },
  { key: "name", labelKey: "profile.masteriesSortName" },
];

export function ProfileMasteriesModal({
  masteries,
  onClose,
  onSelectChampion,
}: ProfileMasteriesModalProps) {
  const { t, locale, formatNumber, formatRelativeTime } = useTranslation();
  const [query, setQuery] = useState("");
  const [sortBy, setSortBy] = useState<MasterySortKey>("points");

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  const summary = useMemo(() => computeMasterySummary(masteries), [masteries]);
  const visibleMasteries = useMemo(
    () => filterAndSortMasteries(masteries, query, sortBy, locale),
    [masteries, query, sortBy, locale]
  );

  return createPortal(
    <div
      className="modal-backdrop masteries-modal-backdrop"
      data-testid="profile-masteries-backdrop"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="card modal-dialog masteries-modal-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="profile-masteries-modal-title"
        data-testid="profile-masteries-modal"
      >
        <div className="masteries-modal-header">
          <div className="masteries-modal-title-group">
            <h2 id="profile-masteries-modal-title" className="masteries-modal-title">
              {t("profile.masteriesModalTitle")}
            </h2>
            <p className="masteries-modal-subtitle">{t("profile.masteriesModalSubtitle")}</p>
          </div>
          <button
            type="button"
            className="masteries-modal-close-btn"
            data-testid="profile-masteries-close-btn"
            aria-label={t("profile.masteriesModalCloseAria")}
            onClick={onClose}
          >
            <svg
              viewBox="0 0 24 24"
              width="16"
              height="16"
              fill="none"
              stroke="currentColor"
              strokeWidth="2.2"
              strokeLinecap="round"
              aria-hidden="true"
            >
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <div className="masteries-summary-strip">
          <div className="masteries-summary-stat">
            <span className="masteries-summary-label">{t("profile.masteriesTotalLevel")}</span>
            <strong className="masteries-summary-value" data-testid="masteries-total-level">
              {formatNumber(summary.totalLevel)}
            </strong>
          </div>
          <div className="masteries-summary-stat">
            <span className="masteries-summary-label">{t("profile.masteriesTotalPoints")}</span>
            <strong className="masteries-summary-value" data-testid="masteries-total-points">
              {formatNumber(summary.totalPoints)}
            </strong>
          </div>
          <div className="masteries-summary-stat">
            <span className="masteries-summary-label">{t("profile.masteriesEnabledChamps")}</span>
            <strong className="masteries-summary-value" data-testid="masteries-total-champions">
              {formatNumber(summary.championCount)}
            </strong>
          </div>
        </div>

        <div className="masteries-controls-bar">
          <SearchInput
            value={query}
            onChange={setQuery}
            placeholder={t("profile.masteriesSearchPlaceholder")}
            ariaLabel={t("profile.masteriesSearchAria")}
            className="masteries-search-box"
            testId="masteries-search-input"
          />

          <div
            className="masteries-sort-group"
            role="group"
            aria-label={t("profile.masteriesSortAria")}
          >
            {SORT_OPTIONS.map((opt) => (
              <button
                key={opt.key}
                type="button"
                className={`masteries-sort-btn${sortBy === opt.key ? " active" : ""}`}
                data-testid={`masteries-sort-${opt.key}`}
                aria-pressed={sortBy === opt.key}
                onClick={() => setSortBy(opt.key)}
              >
                {t(opt.labelKey)}
              </button>
            ))}
          </div>
        </div>

        <div className="masteries-catalog-scroll" data-testid="masteries-catalog-list">
          {visibleMasteries.length > 0 ? (
            <div className="masteries-catalog-grid">
              {visibleMasteries.map((mastery) => {
                const progressPct = computeMasteryProgressPercent(mastery);
                const lastPlayMs = mastery.last_play_time_ms ?? 0;
                const tokens = mastery.tokens_earned ?? 0;
                const markReq = mastery.mark_required_for_next_level ?? 0;
                const marksLabel =
                  markReq > 0
                    ? t("profile.masteriesMarksProgress", { earned: tokens, required: markReq })
                    : tokens > 0
                      ? t("profile.masteriesMarksCount", { earned: tokens })
                      : null;

                return (
                  <button
                    key={mastery.champion_id}
                    type="button"
                    className="mastery-catalog-card"
                    data-testid={`mastery-catalog-item-${mastery.champion_id}`}
                    aria-label={t("profile.openRankedBuildAria", { name: mastery.champion_name })}
                    onClick={() =>
                      onSelectChampion({
                        championId: mastery.champion_id,
                        championName: mastery.champion_name,
                        championIconUrl: mastery.champion_icon_url,
                      })
                    }
                  >
                    <div className="mastery-catalog-top">
                      {mastery.champion_icon_url ? (
                        <div className="dd-icon-frame mastery-champ-frame">
                          <img
                            src={resolveCachedAssetUrl(mastery.champion_icon_url)}
                            alt={mastery.champion_name}
                            className="mastery-champ-icon"
                            loading="lazy"
                            decoding="async"
                          />
                        </div>
                      ) : (
                        <div className="mastery-champ-placeholder">
                          {mastery.champion_name.charAt(0)}
                        </div>
                      )}
                      <div className="mastery-catalog-meta">
                        <span className="mastery-catalog-name">{mastery.champion_name}</span>
                        <span className="mastery-catalog-points">
                          {t("profile.pointsSuffix", {
                            points: formatNumber(mastery.champion_points),
                          })}
                        </span>
                      </div>
                      <span className="mastery-level-pill">
                        {t("profile.levelPrefix", { level: mastery.champion_level })}
                      </span>
                    </div>

                    {(lastPlayMs > 0 || marksLabel) && (
                      <div className="mastery-catalog-foot">
                        <span>{lastPlayMs > 0 ? formatRelativeTime(lastPlayMs) : ""}</span>
                        {marksLabel && <span className="mastery-catalog-marks">{marksLabel}</span>}
                      </div>
                    )}

                    {progressPct !== null && (
                      <div
                        className="mastery-progress-track"
                        data-testid={`mastery-progress-${mastery.champion_id}`}
                        title={t("profile.masteriesPointsToNext", {
                          points: formatNumber(mastery.champion_points_until_next_level ?? 0),
                        })}
                      >
                        <div
                          className="mastery-progress-fill"
                          style={{ width: `${progressPct}%` }}
                        />
                      </div>
                    )}
                  </button>
                );
              })}
            </div>
          ) : (
            <div className="masteries-empty-state" data-testid="masteries-empty-state">
              {t("profile.masteriesNoResults", { query })}
            </div>
          )}
        </div>
      </div>
    </div>,
    document.body
  );
}

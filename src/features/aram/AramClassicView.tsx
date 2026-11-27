import { useTranslation } from "../../i18n";
import { AramChampionRank, AramMode } from "../../types";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { ListSkeleton, LoadingBar, LoadingDots } from "../../components/ui/LoadingIndicators";
import { SearchInput } from "../../components/ui/SearchInput";
import { getChampionTierClass, getChampionTierLabel } from "./aramHelpers";

interface AramClassicViewProps {
  currentChampions: AramChampionRank[];
  filteredAramChampions: AramChampionRank[];
  isLoadingList: boolean;
  listError: string | null;
  aramSearch: string;
  onSearchChange: (val: string) => void;
  onSelectChampion?: (champ: AramChampionRank) => void;
  onRetryTierList: (mode: AramMode) => void;
  motionClassName?: string;
}

export function AramClassicView({
  currentChampions,
  filteredAramChampions,
  isLoadingList,
  listError,
  aramSearch,
  onSearchChange,
  onSelectChampion,
  onRetryTierList,
  motionClassName,
}: AramClassicViewProps) {
  const { t } = useTranslation();

  return (
    <div
      className={`aram-layout${motionClassName ? ` ${motionClassName}` : ""}`}
      data-testid="aram-classic-view"
    >
      <div className="card aram-toolbar-card">
        <LoadingBar
          active={isLoadingList && currentChampions.length > 0}
          testId="aram-classic-loading-bar"
        />
        <div className="aram-toolbar-left">
          <h2 className="page-hero-title">{t("aram.classicRankingTitle")}</h2>
        </div>

        <div className="aram-hero-right">
          <span className="aram-mode-badge">{t("aram.aramTitle")}</span>

          <SearchInput
            value={aramSearch}
            onChange={onSearchChange}
            placeholder={t("common.searchChampionPlaceholder")}
            ariaLabel={t("aram.searchAramAria")}
            clearAriaLabel={t("aram.clearSearchAramAria")}
          />
        </div>
      </div>

      <div className="card aram-classic-list-card">
        {isLoadingList && currentChampions.length === 0 ? (
          <div className="loading-skeleton-shell" data-testid="aram-classic-loading">
            <div className="loading-banner">
              <div className="spinner aram-spinner" />
              <span>{t("aram.loadingRanking")}</span>
              <LoadingDots size="sm" testId="aram-classic-loading-dots" />
            </div>
            <ListSkeleton rows={7} variant="aram" testId="aram-classic-skeleton" />
          </div>
        ) : listError && currentChampions.length === 0 ? (
          <div className="aram-state-box">
            <span className="aram-error-text">{listError}</span>
            <button
              type="button"
              className="aram-retry-btn"
              onClick={() => onRetryTierList("aram")}
            >
              {t("common.retry")}
            </button>
          </div>
        ) : filteredAramChampions.length === 0 ? (
          <div className="aram-state-box">
            <span>{t("aram.noChampFoundSearch", { query: aramSearch })}</span>
          </div>
        ) : (
          <div className="aram-classic-list" data-testid="aram-classic-list">
            {filteredAramChampions.map((champ) => (
              <button
                key={champ.id}
                type="button"
                className="aram-classic-row"
                onClick={() => onSelectChampion?.(champ)}
                data-testid={`aram-classic-champ-${champ.key}`}
              >
                <span className="aram-rank-num">#{champ.rank}</span>
                <div className="dd-icon-frame aram-champ-frame">
                  <img
                    src={resolveCachedAssetUrl(champ.icon_url)}
                    alt={champ.name}
                    className="aram-champ-avatar"
                    loading="lazy"
                    decoding="async"
                  />
                </div>
                <span className="aram-champ-name">{champ.name}</span>

                <div className="aram-classic-stats">
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
                  <span className={`aram-tier-pill ${getChampionTierClass(champ.tier)}`}>
                    {getChampionTierLabel(champ.tier)}
                  </span>
                </div>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

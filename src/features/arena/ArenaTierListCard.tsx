import { useTranslation } from "../../i18n";
import type { ArenaChampionEntry, ArenaTierListResponse } from "../../types";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { ListSkeleton, LoadingDots } from "../../components/ui/LoadingIndicators";
import { getChampionTierClass, getChampionTierLabel } from "../aram/aramHelpers";
import type { ArenaTierGroup } from "./arenaHelpers";

interface ArenaTierListCardProps {
  tierLoading: boolean;
  tierData: ArenaTierListResponse | null;
  tierError: string | null;
  tierGroups: ArenaTierGroup[];
  search: string;
  tierFilter: string;
  onSelectChampion: (entry: ArenaChampionEntry) => void;
  onRetryTierList: () => void;
}

/**
 * Scrollable tier-grouped Arena (`3x6`) champion list displaying Top 3 rate, 1st place rate, pick rate, and ban rate.
 */
export function ArenaTierListCard({
  tierLoading,
  tierData,
  tierError,
  tierGroups,
  search,
  tierFilter,
  onSelectChampion,
  onRetryTierList,
}: ArenaTierListCardProps) {
  const { t } = useTranslation();

  let runningRank = 0;

  return (
    <div className="card tierlist-list-card">
      {tierLoading && !tierData ? (
        <div className="loading-skeleton-shell" data-testid="arena-tierlist-loading">
          <div className="loading-banner">
            <div className="spinner" />
            <span>{t("arenaTab.loadingTierList")}</span>
            <LoadingDots size="sm" testId="arena-tierlist-loading-dots" />
          </div>
          <ListSkeleton rows={7} variant="ranked" testId="arena-tierlist-skeleton" />
        </div>
      ) : tierError && !tierData ? (
        <div className="tierlist-state-box">
          <span className="tierlist-error-text">{tierError}</span>
          <button type="button" className="tierlist-retry-btn" onClick={onRetryTierList}>
            {t("common.retry")}
          </button>
        </div>
      ) : tierGroups.length === 0 ? (
        <div className="tierlist-state-box">
          <span>
            {search.trim()
              ? t("arenaTab.noChampFoundSearch", { query: search })
              : tierFilter !== "all"
              ? t("arenaTab.noChampFoundTier")
              : t("arenaTab.noDataAvailable")}
          </span>
        </div>
      ) : (
        <div className="tierlist-list-scroll arena-list-scroll" data-testid="arena-tierlist-scroll">
          <div className="tierlist-tier-groups arena-tier-groups" data-testid="arena-tier-groups">
            {tierGroups.map((group) => (
              <div
                key={`arena-tier-${group.tier}`}
                className="tierlist-tier-group"
                data-testid={`arena-tier-group-${group.tier}`}
              >
                <div className="tierlist-tier-group-header">
                  <span className={`tier-pill ${getChampionTierClass(group.tier)}`}>
                    {getChampionTierLabel(group.tier)}
                  </span>
                  <span className="tierlist-tier-group-count">
                    {group.items.length}{" "}
                    {group.items.length === 1
                      ? t("common.championSingular")
                      : t("common.championPlural")}
                  </span>
                </div>

                <div className="tierlist-tier-rows">
                  {group.items.map((champ) => {
                    runningRank += 1;
                    const displayRank = champ.rank > 0 ? champ.rank : runningRank;

                    return (
                      <button
                        key={champ.id}
                        type="button"
                        className="tierlist-champ-row"
                        onClick={() => onSelectChampion(champ)}
                        data-testid={`arena-champ-${champ.key}`}
                      >
                        <span className="tierlist-rank-num">
                          {displayRank > 0 ? `#${displayRank}` : "—"}
                        </span>
                        <div className="dd-icon-frame tierlist-champ-frame">
                          <img
                            src={resolveCachedAssetUrl(champ.icon_url)}
                            alt={champ.name}
                            className="tierlist-champ-avatar"
                            loading="lazy"
                            decoding="async"
                          />
                        </div>
                        <span className="tierlist-champ-name">{champ.name}</span>

                        <div className="tierlist-row-stats">
                          {champ.win_rate !== null && champ.win_rate !== undefined && (
                            <span className="stat-pill">
                              <span className="stat-pill-label">{t("arenaTab.top3Label")}</span>
                              <span
                                className={`stat-pill-val ${
                                  champ.win_rate >= 53
                                    ? "is-high"
                                    : champ.win_rate < 47
                                    ? "is-low"
                                    : ""
                                }`}
                              >
                                {champ.win_rate.toFixed(1)}%
                              </span>
                            </span>
                          )}
                          {champ.first_place_rate !== null &&
                            champ.first_place_rate !== undefined && (
                              <span className="stat-pill">
                                <span className="stat-pill-label">
                                  {t("arenaTab.firstPlaceLabel")}
                                </span>
                                <span
                                  className={`stat-pill-val ${
                                    champ.first_place_rate >= 15 ? "is-accent" : ""
                                  }`}
                                >
                                  {champ.first_place_rate.toFixed(1)}%
                                </span>
                              </span>
                            )}
                          {champ.pick_rate !== null && champ.pick_rate !== undefined && (
                            <span className="stat-pill">
                              <span className="stat-pill-label">PR</span>
                              <span className="stat-pill-val">{champ.pick_rate.toFixed(1)}%</span>
                            </span>
                          )}
                          {champ.ban_rate !== null && champ.ban_rate !== undefined && (
                            <span className="stat-pill">
                              <span className="stat-pill-label">BR</span>
                              <span className="stat-pill-val">{champ.ban_rate.toFixed(1)}%</span>
                            </span>
                          )}
                          <span className={`tier-pill ${getChampionTierClass(champ.tier)}`}>
                            {getChampionTierLabel(champ.tier)}
                          </span>
                        </div>
                      </button>
                    );
                  })}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

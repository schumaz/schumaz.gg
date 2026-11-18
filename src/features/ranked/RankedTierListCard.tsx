import { useTranslation } from "../../i18n";
import {
  RankedChampionEntry,
  RankedLane,
  RankedTierListResponse,
} from "../../types";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { buildIconImg } from "../../components/ui/filterOptions";
import { ListSkeleton, LoadingDots } from "../../components/ui/LoadingIndicators";
import { getChampionTierClass, getChampionTierLabel } from "../aram/aramHelpers";
import { RankedLaneFilter } from "./rankedStorage";
import { LaneMetaItem, TierGroup } from "./rankedHelpers";

interface RankedTierListCardProps {
  tierLoading: boolean;
  tierData: RankedTierListResponse | null;
  tierError: string | null;
  tierGroups: TierGroup[];
  search: string;
  tierFilter: string;
  laneFilter: RankedLaneFilter;
  isAllLanes: boolean;
  activeLaneMeta: LaneMetaItem | null;
  laneMeta: Record<RankedLane, LaneMetaItem>;
  onSelectChampion: (entry: RankedChampionEntry) => void;
  onRetryTierList: () => void;
}

export function RankedTierListCard({
  tierLoading,
  tierData,
  tierError,
  tierGroups,
  search,
  tierFilter,
  laneFilter,
  isAllLanes,
  activeLaneMeta,
  laneMeta,
  onSelectChampion,
  onRetryTierList,
}: RankedTierListCardProps) {
  const { t } = useTranslation();

  const renderTierGroups = (groups: TierGroup[], scopeKey: string) => {
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
            {group.items.length}{" "}
            {group.items.length === 1 ? t("common.championSingular") : t("common.championPlural")}
          </span>
        </div>

        <div className="ranked-tier-rows">
          {group.items.map((champ) => {
            runningGlobalRank += 1;
            const laneInfo = laneMeta[champ.lane];
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
                    src={resolveCachedAssetUrl(champ.icon_url)}
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
                          ? t("ranked.rankInLaneTitle", {
                              rank: champ.rank,
                              lane: laneInfo.shortLabel,
                            })
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
    <div className="card ranked-list-card">
      {tierLoading && !tierData ? (
        <div className="loading-skeleton-shell" data-testid="ranked-tierlist-loading">
          <div className="loading-banner">
            <div className="spinner aram-spinner" />
            <span>{t("ranked.loadingTierList")}</span>
            <LoadingDots size="sm" testId="ranked-tierlist-loading-dots" />
          </div>
          <ListSkeleton rows={7} variant="ranked" testId="ranked-tierlist-skeleton" />
        </div>
      ) : tierError && !tierData ? (
        <div className="aram-state-box">
          <span className="aram-error-text">{tierError}</span>
          <button type="button" className="aram-retry-btn" onClick={onRetryTierList}>
            {t("common.retry")}
          </button>
        </div>
      ) : tierGroups.length === 0 ? (
        <div className="aram-state-box">
          <span>
            {search.trim()
              ? t("ranked.noChampFoundSearch", { query: search })
              : tierFilter !== "all"
              ? t("ranked.noChampFoundTier")
              : t("ranked.noDataAvailable")}
          </span>
        </div>
      ) : (
        <div className="ranked-list-scroll" data-testid="ranked-tierlist-scroll">
          {isAllLanes ? (
            <div className="ranked-tier-groups" data-testid="ranked-all-lanes-list">
              {renderTierGroups(tierGroups, "all")}
            </div>
          ) : (
            activeLaneMeta && (
              <section
                className="ranked-lane-section"
                data-testid={`ranked-lane-section-${laneFilter}`}
              >
                <header className="ranked-lane-section-header">
                  <div className="ranked-lane-section-title-wrap">
                    {buildIconImg(activeLaneMeta.iconUrl, "ranked-lane-section-icon")}
                    <h3 className="ranked-lane-section-title">{activeLaneMeta.label}</h3>
                  </div>
                </header>

                <div className="ranked-tier-groups">
                  {renderTierGroups(tierGroups, laneFilter)}
                </div>
              </section>
            )
          )}
        </div>
      )}
    </div>
  );
}

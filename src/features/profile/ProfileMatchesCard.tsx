import { CSSProperties, useEffect, useState } from "react";
import { RecentMatchInfo } from "../../types";
import { useTranslation } from "../../i18n";
import {
  AugmentMetadataMap,
  ItemMetadataMap,
  LoadingDots,
  RuneMetadataMap,
  SpellMetadataMap,
} from "../../components/ui";
import { MatchRowSummary } from "./MatchRowSummary";
import { MatchScoreboardDrawer } from "./MatchScoreboardDrawer";
import { AvailableModeItem } from "./profileHelpers";
import { useMatchScoreboard } from "./useMatchScoreboard";

interface ProfileMatchesCardProps {
  validMatches: RecentMatchInfo[];
  filteredMatches: RecentMatchInfo[];
  availableModes: AvailableModeItem[];
  activeMode: string;
  onSelectMode: (mode: string) => void;
  glassBlurStyle: CSSProperties;
  itemMetadata?: ItemMetadataMap;
  runeMetadata?: RuneMetadataMap;
  spellMetadata?: SpellMetadataMap;
  augmentMetadata?: AugmentMetadataMap;
  isLcuConnected?: boolean;
  hasMore?: boolean;
  canLoadMore?: boolean;
  isLoadingMore?: boolean;
  loadMoreError?: string | null;
  onLoadMore?: () => void;
  resetKey?: unknown;
}

export function ProfileMatchesCard({
  validMatches,
  filteredMatches,
  availableModes,
  activeMode,
  onSelectMode,
  glassBlurStyle,
  itemMetadata,
  runeMetadata,
  spellMetadata,
  augmentMetadata,
  isLcuConnected = true,
  hasMore = false,
  canLoadMore = false,
  isLoadingMore = false,
  loadMoreError = null,
  onLoadMore,
  resetKey,
}: ProfileMatchesCardProps) {
  const { t, locale } = useTranslation();
  const [expandedGameId, setExpandedGameId] = useState<number | null>(null);
  const scoreboard = useMatchScoreboard(isLcuConnected, resetKey, locale);

  useEffect(() => {
    setExpandedGameId(null);
  }, [resetKey]);

  useEffect(() => {
    if (expandedGameId !== null) {
      scoreboard.loadScoreboard(expandedGameId);
    }
  }, [expandedGameId, isLcuConnected, scoreboard.loadScoreboard]);

  return (
    <div className="profile-col profile-col-right">
      <section className="card profile-matches-card" style={glassBlurStyle}>
        <div className="profile-matches-header">
          <div>
            <h3>{t("profile.recentMatchesTitle")}</h3>
            <span className="profile-card-subtitle">
              {t("profile.showingMatches", {
                shown: filteredMatches.length,
                total: validMatches.length,
              })}
            </span>
          </div>

          <div
            className="mode-filter-chips"
            role="group"
            aria-label={t("profile.filterMatchesAria")}
            data-testid="match-mode-filters"
          >
            <button
              type="button"
              className={`mode-chip ${activeMode === "all" ? "active" : ""}`}
              onClick={() => onSelectMode("all")}
              aria-pressed={activeMode === "all"}
              data-testid="mode-filter-all"
            >
              {t("profile.allModes", { count: validMatches.length })}
            </button>
            {availableModes.map((mode) => (
              <button
                key={mode.key}
                type="button"
                className={`mode-chip ${activeMode === mode.key ? "active" : ""}`}
                onClick={() => onSelectMode(mode.key)}
                aria-pressed={activeMode === mode.key}
                data-testid={`mode-filter-${mode.key}`}
              >
                {mode.label} ({mode.count})
              </button>
            ))}
          </div>
        </div>

        <div className="matches-scroll-list" data-testid="recent-matches-list">
          {filteredMatches.length > 0 ? (
            filteredMatches.map((match) => {
              const isExpanded = expandedGameId === match.game_id;
              const resultClass = match.remake ? "remake" : match.win ? "win" : "loss";

              return (
                <div
                  key={match.game_id}
                  className={`match-card ${resultClass} ${isExpanded ? "expanded" : ""}`}
                  data-testid={`match-item-${match.game_id}`}
                >
                  <button
                    type="button"
                    className="match-row-btn"
                    onClick={() => setExpandedGameId(isExpanded ? null : match.game_id)}
                    aria-expanded={isExpanded}
                  >
                    <MatchRowSummary
                      match={match}
                      itemMetadata={itemMetadata}
                      runeMetadata={runeMetadata}
                      spellMetadata={spellMetadata}
                      augmentMetadata={augmentMetadata}
                    />
                  </button>

                  {isExpanded && (
                    <MatchScoreboardDrawer
                      match={match}
                      entry={scoreboard.getEntry(match.game_id)}
                      isLcuConnected={isLcuConnected}
                      onRetry={() => scoreboard.retryScoreboard(match.game_id)}
                    />
                  )}
                </div>
              );
            })
          ) : (
            <div className="matches-empty">
              <p className="text-muted">{t("profile.noMatchesMode")}</p>
            </div>
          )}

          {(hasMore || loadMoreError) && (
            <div className="matches-pagination-footer">
              {loadMoreError && (
                <p
                  className="matches-load-more-error"
                  role="alert"
                  data-testid="load-more-matches-error"
                >
                  {loadMoreError}
                </p>
              )}
              {hasMore && (
                <button
                  type="button"
                  className="matches-load-more-btn"
                  data-testid="load-more-matches-btn"
                  disabled={!canLoadMore || isLoadingMore}
                  onClick={() => onLoadMore?.()}
                >
                  {isLoadingMore ? (
                    <>
                      <span>{t("profile.loadingMoreMatches")}</span>
                      <LoadingDots
                        variant="inline"
                        size="sm"
                        testId="load-more-matches-dots"
                      />
                    </>
                  ) : (
                    t("profile.loadMoreMatches")
                  )}
                </button>
              )}
            </div>
          )}
        </div>
      </section>
    </div>
  );
}

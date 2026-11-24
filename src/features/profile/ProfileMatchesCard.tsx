import { CSSProperties, useState } from "react";
import { RecentMatchInfo } from "../../types";
import { useTranslation } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { ItemMetadataMap } from "../../components/ui";
import { MatchItemsGrid } from "./MatchItemsGrid";
import { AvailableModeItem, formatDuration, getMultiKillBadge } from "./profileHelpers";

interface ProfileMatchesCardProps {
  validMatches: RecentMatchInfo[];
  filteredMatches: RecentMatchInfo[];
  availableModes: AvailableModeItem[];
  activeMode: string;
  onSelectMode: (mode: string) => void;
  glassBlurStyle: CSSProperties;
  itemMetadata?: ItemMetadataMap;
}

export function ProfileMatchesCard({
  validMatches,
  filteredMatches,
  availableModes,
  activeMode,
  onSelectMode,
  glassBlurStyle,
  itemMetadata,
}: ProfileMatchesCardProps) {
  const { t, formatCompactNumber, formatRelativeTime } = useTranslation();
  const [expandedGameId, setExpandedGameId] = useState<number | null>(null);

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
              const multiKill = getMultiKillBadge(match);
              const mins = Math.max(1, match.game_duration_secs / 60);
              const csPerMin = (match.cs / mins).toFixed(1);
              const kdaText =
                match.deaths === 0
                  ? t("profile.perfectKda")
                  : t("profile.kdaRatio", {
                      ratio: ((match.kills + match.assists) / match.deaths).toFixed(2),
                    });

              return (
                <div
                  key={match.game_id}
                  className={`match-card ${match.win ? "win" : "loss"} ${
                    isExpanded ? "expanded" : ""
                  }`}
                  data-testid={`match-item-${match.game_id}`}
                >
                  <button
                    type="button"
                    className="match-row-btn"
                    onClick={() => setExpandedGameId(isExpanded ? null : match.game_id)}
                    aria-expanded={isExpanded}
                  >
                    <div className="match-champ-col">
                      {match.champion_icon_url ? (
                        <div className="dd-icon-frame match-champ-frame">
                          <img
                            src={resolveCachedAssetUrl(match.champion_icon_url)}
                            alt={match.champion_name}
                            className="match-champ-icon"
                          />
                        </div>
                      ) : (
                        <div className="match-champ-placeholder">
                          {match.champion_name.charAt(0)}
                        </div>
                      )}
                      <div className="match-spells-col">
                        {match.spell1_icon_url && (
                          <div className="dd-icon-frame match-spell-frame">
                            <img
                              src={resolveCachedAssetUrl(match.spell1_icon_url)}
                              alt={t("profile.spell1Alt")}
                              className="match-spell-icon"
                            />
                          </div>
                        )}
                        {match.spell2_icon_url && (
                          <div className="dd-icon-frame match-spell-frame">
                            <img
                              src={resolveCachedAssetUrl(match.spell2_icon_url)}
                              alt={t("profile.spell2Alt")}
                              className="match-spell-icon"
                            />
                          </div>
                        )}
                      </div>
                    </div>

                    <div className="match-main-info">
                      <div className="match-top-line">
                        <span className={`match-result-tag ${match.win ? "win" : "loss"}`}>
                          {match.win ? t("profile.victory") : t("profile.defeat")}
                        </span>
                        <strong className="match-champ-name">{match.champion_name}</strong>
                        <span className="match-mode-pill">{match.game_mode_label}</span>
                      </div>
                      <div className="match-sub-line">
                        <span>{formatDuration(match.game_duration_secs)}</span>
                        {match.game_creation_ms > 0 && (
                          <span>· {formatRelativeTime(match.game_creation_ms)}</span>
                        )}
                        {multiKill && <span className="match-multikill-pill">{multiKill}</span>}
                      </div>
                    </div>

                    <div className="match-kda-col">
                      <span className="match-kda-score">
                        {match.kills} / <span className="deaths">{match.deaths}</span> /{" "}
                        {match.assists}
                      </span>
                      <span className="match-kda-ratio">
                        {kdaText} · {match.cs} CS
                      </span>
                    </div>

                    <MatchItemsGrid
                      itemIds={match.item_ids}
                      itemIconUrls={match.item_icon_urls}
                      itemMetadata={itemMetadata}
                    />
                  </button>

                  {isExpanded && (
                    <div
                      className="match-details-drawer"
                      data-testid={`match-details-${match.game_id}`}
                    >
                      <div className="match-detail-stat">
                        <span>{t("profile.detailLevel")}</span>
                        <strong>{match.champ_level}</strong>
                      </div>
                      <div className="match-detail-stat">
                        <span>{t("profile.detailDamage")}</span>
                        <strong>
                          {formatCompactNumber(match.total_damage_dealt_to_champions)}
                        </strong>
                      </div>
                      <div className="match-detail-stat">
                        <span>{t("profile.detailGold")}</span>
                        <strong>{formatCompactNumber(match.gold_earned)}</strong>
                      </div>
                      <div className="match-detail-stat">
                        <span>{t("profile.detailVision")}</span>
                        <strong>
                          {match.vision_score} (
                          {t("profile.wardsPlacedShort", { count: match.wards_placed })})
                        </strong>
                      </div>
                      <div className="match-detail-stat">
                        <span>{t("profile.detailCsPerMin")}</span>
                        <strong>{csPerMin}</strong>
                      </div>
                    </div>
                  )}
                </div>
              );
            })
          ) : (
            <div className="matches-empty">
              <p className="text-muted">{t("profile.noMatchesMode")}</p>
            </div>
          )}
        </div>
      </section>
    </div>
  );
}

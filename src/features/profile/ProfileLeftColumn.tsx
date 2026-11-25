import { CSSProperties } from "react";
import { PlayerProfile, RecentMatchInfo } from "../../types";
import { useTranslation } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { HighlightedChampion, WinLossStats } from "./profileHelpers";

interface ProfileLeftColumnProps {
  profile: PlayerProfile | null;
  filteredMatches: RecentMatchInfo[];
  winLossStats: WinLossStats;
  highlightedChampion: HighlightedChampion | null;
  glassBlurStyle: CSSProperties;
}

export function ProfileLeftColumn({
  profile,
  filteredMatches,
  winLossStats,
  highlightedChampion,
  glassBlurStyle,
}: ProfileLeftColumnProps) {
  const { t, formatNumber } = useTranslation();

  const donutRadius = 38;
  const donutCircumference = 2 * Math.PI * donutRadius;
  const winArcLength =
    winLossStats.total > 0 ? (winLossStats.wins / winLossStats.total) * donutCircumference : 0;

  return (
    <div className="profile-col profile-col-left">
      <section
        className="card profile-section-card"
        data-testid="winloss-chart-card"
        style={glassBlurStyle}
      >
        <div className="profile-card-header">
          <h3>{t("profile.winLossTitle")}</h3>
          <span className="profile-card-subtitle">
            {winLossStats.total}{" "}
            {winLossStats.total === 1 ? t("common.gamesSingular") : t("common.gamesPlural")}
          </span>
        </div>

        {winLossStats.total > 0 ? (
          <>
            <div className="winloss-overview">
              <div className="winloss-donut-wrap">
                <svg
                  viewBox="0 0 100 100"
                  className="winloss-donut-svg"
                  data-testid="winloss-donut-svg"
                  role="img"
                  aria-label={t("profile.winRateAria", {
                    rate: winLossStats.winRate,
                    wins: winLossStats.wins,
                    losses: winLossStats.losses,
                  })}
                >
                  <circle
                    cx="50"
                    cy="50"
                    r={donutRadius}
                    fill="transparent"
                    stroke="var(--accent-danger)"
                    strokeOpacity="0.35"
                    strokeWidth="10"
                  />
                  <circle
                    cx="50"
                    cy="50"
                    r={donutRadius}
                    fill="transparent"
                    stroke="var(--accent-success)"
                    strokeWidth="10"
                    strokeDasharray={`${winArcLength} ${donutCircumference}`}
                    strokeLinecap="round"
                    transform="rotate(-90 50 50)"
                  />
                  <text x="50" y="48" textAnchor="middle" className="donut-center-value">
                    {winLossStats.winRate}%
                  </text>
                  <text x="50" y="62" textAnchor="middle" className="donut-center-label">
                    {t("profile.winRateLabel")}
                  </text>
                </svg>
              </div>

              <div className="winloss-metrics">
                <div className="winloss-stat-line">
                  <span className="winloss-dot win" />
                  <span>{t("profile.winsLabel")}</span>
                  <strong data-testid="win-count">{winLossStats.wins}</strong>
                </div>
                <div className="winloss-stat-line">
                  <span className="winloss-dot loss" />
                  <span>{t("profile.lossesLabel")}</span>
                  <strong data-testid="loss-count">{winLossStats.losses}</strong>
                </div>
                {highlightedChampion && (
                  <div className="highlighted-champ-box" data-testid="highlighted-champion">
                    {highlightedChampion.icon && (
                      <div className="dd-icon-frame highlighted-champ-frame">
                        <img
                          src={resolveCachedAssetUrl(highlightedChampion.icon)}
                          alt={highlightedChampion.name}
                          className="highlighted-champ-icon"
                        />
                      </div>
                    )}
                    <div className="highlighted-champ-text">
                      <span className="highlighted-label">{t("profile.highlightLabel")}</span>
                      <strong>{highlightedChampion.name}</strong>
                      <span className="highlighted-sub">
                        {t("profile.highlightSub", {
                          games: highlightedChampion.games,
                          wr: Math.round(
                            (highlightedChampion.wins / highlightedChampion.games) * 100
                          ),
                        })}
                      </span>
                    </div>
                  </div>
                )}
              </div>
            </div>

            <div className="recent-streak-block" data-testid="recent-streak">
              <span className="recent-streak-label">{t("profile.recentStreak")}</span>
              <div className="recent-streak-pills">
                {filteredMatches
                  .filter((m) => !m.remake)
                  .map((m) => (
                    <span
                      key={m.game_id}
                      className={`streak-pill ${m.win ? "win" : "loss"}`}
                      title={`${m.champion_name} (${m.kills}/${m.deaths}/${m.assists}) — ${
                        m.win ? t("profile.victory") : t("profile.defeat")
                      }`}
                    >
                      {m.win ? t("profile.winShort") : t("profile.lossShort")}
                    </span>
                  ))}
              </div>
            </div>
          </>
        ) : (
          <p className="text-muted">{t("profile.noMatchesFilter")}</p>
        )}
      </section>

      <section
        className="card profile-section-card"
        data-testid="top-masteries-card"
        style={glassBlurStyle}
      >
        <div className="profile-card-header">
          <h3>{t("profile.topMasteriesTitle")}</h3>
          <span className="profile-card-subtitle">{t("profile.topMasteriesSubtitle")}</span>
        </div>

        {profile && profile.top_masteries.length > 0 ? (
          <div className="mastery-list">
            {profile.top_masteries.map((mastery, idx) => (
              <div
                key={mastery.champion_id}
                className="mastery-row"
                data-testid={`mastery-item-${idx + 1}`}
              >
                <span className={`mastery-rank rank-${idx + 1}`}>#{idx + 1}</span>
                {mastery.champion_icon_url ? (
                  <div className="dd-icon-frame mastery-champ-frame">
                    <img
                      src={resolveCachedAssetUrl(mastery.champion_icon_url)}
                      alt={mastery.champion_name}
                      className="mastery-champ-icon"
                    />
                  </div>
                ) : (
                  <div className="mastery-champ-placeholder">
                    {mastery.champion_name.charAt(0)}
                  </div>
                )}
                <div className="mastery-info">
                  <span className="mastery-champ-name">{mastery.champion_name}</span>
                  <span className="mastery-points">
                    {t("profile.pointsSuffix", {
                      points: formatNumber(mastery.champion_points),
                    })}
                  </span>
                </div>
                <span className="mastery-level-pill">
                  {t("profile.levelPrefix", { level: mastery.champion_level })}
                </span>
              </div>
            ))}
          </div>
        ) : (
          <p className="text-muted">{t("profile.noMasteries")}</p>
        )}
      </section>
    </div>
  );
}

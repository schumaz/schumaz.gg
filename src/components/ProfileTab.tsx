import { CSSProperties, useMemo, useState } from "react";
import { useTranslation } from "../i18n";
import { PlayerProfile, RecentMatchInfo, SummonerInfo } from "../types";
import { profileBlurPercentToPx } from "../hooks/usePreferences";
import { IdleIsland } from "./IdleState";
import { LoadingBar, LoadingDots } from "./LoadingIndicators";

interface ProfileTabProps {
  summoner: SummonerInfo | null;
  profile: PlayerProfile | null;
  loading: boolean;
  errorMsg: string | null;
  onRefresh: () => void;
  refreshCooldownSecs?: number;
  profileBlur?: number;
  isActive?: boolean;
}

const CDRAGON_RANKED_EMBLEM_BASE =
  "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-shared-components/global/default/images";
const CDRAGON_RANKED_MINI_CRESTS_BASE =
  "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/images/ranked-mini-crests";

const VALID_RANK_TIERS = new Set([
  "iron",
  "bronze",
  "silver",
  "gold",
  "platinum",
  "emerald",
  "diamond",
  "master",
  "grandmaster",
  "challenger",
  "unranked",
]);

function normalizeRankTierKey(tier: string): string {
  const normalized = tier.trim().toLowerCase();
  return VALID_RANK_TIERS.has(normalized) ? normalized : "unranked";
}

function getRankedCrestUrl(tier: string): string {
  return `${CDRAGON_RANKED_EMBLEM_BASE}/${normalizeRankTierKey(tier)}.png`;
}

function getRankedMiniCrestUrl(tier: string): string {
  return `${CDRAGON_RANKED_MINI_CRESTS_BASE}/${normalizeRankTierKey(tier)}.svg`;
}

function formatDuration(secs: number): string {
  const mins = Math.floor(Math.max(0, secs) / 60);
  const rem = Math.max(0, secs) % 60;
  return `${mins}m ${rem.toString().padStart(2, "0")}s`;
}

function getMultiKillBadge(match: RecentMatchInfo): string | null {
  if (match.penta_kills > 0) return "Penta Kill";
  if (match.quadra_kills > 0) return "Quadra Kill";
  if (match.triple_kills > 0) return "Triple Kill";
  if (match.double_kills > 0) return "Double Kill";
  return null;
}

export function ProfileTab({
  summoner,
  profile,
  loading,
  errorMsg,
  onRefresh,
  refreshCooldownSecs = 0,
  profileBlur = 10,
  isActive = true,
}: ProfileTabProps) {
  const { t, formatNumber, formatCompactNumber, formatRelativeTime } = useTranslation();
  const [selectedMode, setSelectedMode] = useState<string>("all");
  const [expandedGameId, setExpandedGameId] = useState<number | null>(null);

  // Defensive exclusion of Practice Tool on frontend as well
  const validMatches = useMemo(() => {
    if (!profile?.recent_matches) return [];
    return profile.recent_matches.filter((m) => {
      const modeLabelLower = m.game_mode_label.toLowerCase();
      return (
        m.queue_id >= 0 &&
        m.game_mode_key.toLowerCase() !== "practicetool" &&
        !modeLabelLower.includes("treino") &&
        !modeLabelLower.includes("practice")
      );
    });
  }, [profile]);

  const availableModes = useMemo(() => {
    const modeMap = new Map<string, { key: string; label: string; count: number }>();
    for (const match of validMatches) {
      const existing = modeMap.get(match.game_mode_key);
      if (existing) {
        existing.count += 1;
      } else {
        modeMap.set(match.game_mode_key, {
          key: match.game_mode_key,
          label: match.game_mode_label,
          count: 1,
        });
      }
    }
    return Array.from(modeMap.values());
  }, [validMatches]);

  const activeMode =
    selectedMode === "all" || availableModes.some((m) => m.key === selectedMode)
      ? selectedMode
      : "all";

  const filteredMatches = useMemo(() => {
    if (activeMode === "all") return validMatches;
    return validMatches.filter((m) => m.game_mode_key === activeMode);
  }, [validMatches, activeMode]);

  const winLossStats = useMemo(() => {
    const total = filteredMatches.length;
    const wins = filteredMatches.filter((m) => m.win).length;
    const losses = total - wins;
    const winRate = total > 0 ? Math.round((wins / total) * 100) : 0;
    return { total, wins, losses, winRate };
  }, [filteredMatches]);

  const highlightedChampion = useMemo(() => {
    if (filteredMatches.length === 0) return null;
    const byChamp = new Map<
      number,
      { name: string; icon: string; games: number; wins: number; kdaSum: number }
    >();
    for (const m of filteredMatches) {
      const kda = m.deaths === 0 ? m.kills + m.assists : (m.kills + m.assists) / m.deaths;
      const cur = byChamp.get(m.champion_id);
      if (cur) {
        cur.games += 1;
        cur.wins += m.win ? 1 : 0;
        cur.kdaSum += kda;
      } else {
        byChamp.set(m.champion_id, {
          name: m.champion_name,
          icon: m.champion_icon_url,
          games: 1,
          wins: m.win ? 1 : 0,
          kdaSum: kda,
        });
      }
    }
    const sorted = Array.from(byChamp.values()).sort(
      (a, b) => b.games - a.games || b.wins - a.wins || b.kdaSum - a.kdaSum
    );
    return sorted[0] ?? null;
  }, [filteredMatches]);

  // SVG Donut geometry
  const donutRadius = 38;
  const donutCircumference = 2 * Math.PI * donutRadius;
  const winArcLength =
    winLossStats.total > 0 ? (winLossStats.wins / winLossStats.total) * donutCircumference : 0;

  const profileUiBlurPx = profileBlurPercentToPx(profileBlur);
  const glassBlurStyle: CSSProperties = {
    backdropFilter: `blur(${profileUiBlurPx}px)`,
    WebkitBackdropFilter: `blur(${profileUiBlurPx}px)`,
  };

  return (
    <div
      className={`tab-content ${isActive ? "active-tab" : "inactive-tab"}`}
      data-testid="view-profile"
      aria-hidden={!isActive}
      style={{ "--profile-ui-blur": `${profileUiBlurPx}px` } as CSSProperties}
    >
      {!summoner ? (
        <IdleIsland
          testId="profile-waiting-client"
          mode="waiting"
          badge={t("profile.badge")}
          title={t("profile.waitingTitle")}
          description={t("profile.waitingDesc")}
          errorMsg={errorMsg}
          icon={
            <svg
              viewBox="0 0 24 24"
              width="42"
              height="42"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.75"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
              <circle cx="12" cy="7" r="4" />
            </svg>
          }
        />
      ) : loading && !profile ? (
        <IdleIsland
          testId="profile-loading"
          mode="loading"
          badge={t("profile.badge")}
          title={t("profile.loadingTitle")}
          description={t("profile.loadingDesc")}
          icon={
            <svg
              viewBox="0 0 24 24"
              width="42"
              height="42"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.75"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
              <circle cx="12" cy="7" r="4" />
            </svg>
          }
        />
      ) : (
        <div className="profile-layout" data-testid="profile-loaded">
          {/* Top Profile Hero Card */}
          <section className="card profile-hero-card" style={glassBlurStyle}>
            <LoadingBar active={loading} testId="profile-loading-bar" />

            <div className="profile-hero-main">
              <div className="profile-identity">
                <div className="profile-avatar-wrap">
                  {profile?.profile_icon_url ? (
                    <div className="dd-icon-frame profile-avatar-frame">
                      <img
                        src={profile.profile_icon_url}
                        alt={t("profile.summonerIconAlt")}
                        className="profile-avatar-img"
                      />
                    </div>
                  ) : (
                    <div className="profile-avatar-placeholder">
                      {(summoner.gameName || summoner.displayName || "?").charAt(0).toUpperCase()}
                    </div>
                  )}
                  <span className="profile-level-badge" data-testid="profile-level">
                    {t("profile.levelPrefix", { level: summoner.summonerLevel })}
                  </span>
                </div>

                <div className="profile-riot-id">
                  <h2
                    className="profile-summoner-name"
                    title={`${summoner.gameName || summoner.displayName}${summoner.tagLine ? `#${summoner.tagLine}` : ""}`}
                  >
                    {summoner.gameName || summoner.displayName}
                  </h2>
                  {summoner.tagLine && (
                    <span className="profile-tagline">#{summoner.tagLine}</span>
                  )}
                </div>
              </div>

              <div className="profile-hero-right">
                {profile?.ranked_stats && profile.ranked_stats.length > 0 && (
                  <div className="profile-ranked-row" data-testid="profile-ranked-row">
                    {profile.ranked_stats.map((rank) => {
                      const isUnranked = rank.tier === "UNRANKED";
                      const totalRanked = rank.wins + rank.losses;
                      const rankedWr =
                        totalRanked > 0 ? Math.round((rank.wins / totalRanked) * 100) : 0;
                      const rankDisplay = isUnranked
                        ? t("profile.unranked")
                        : `${rank.tier_label}${rank.division ? ` ${rank.division}` : ""}`;
                      const lpLabel = t("profile.lpLabel");
                      const winShort = t("profile.winShort");
                      const lossShort = t("profile.lossShort");

                      return (
                        <div
                          key={rank.queue_type}
                          className={`ranked-pill ${isUnranked ? "unranked" : ""}`}
                          data-testid={`ranked-pill-${rank.queue_type}`}
                          title={
                            isUnranked
                              ? `${rank.queue_label}: ${t("profile.unranked")}`
                              : `${rank.queue_label}: ${rankDisplay} — ${rank.league_points} ${lpLabel} · ${rank.wins}${winShort} ${rank.losses}${lossShort} (${rankedWr}%)`
                          }
                        >
                          <div className="ranked-crest-wrap">
                            <img
                              src={getRankedCrestUrl(rank.tier)}
                              alt={t("profile.rankIconAlt", {
                                queue: rank.queue_label,
                                rank: rankDisplay,
                              })}
                              className={`ranked-crest-img ${isUnranked ? "unranked" : ""}`}
                              onError={(e) => {
                                const img = e.currentTarget;
                                if (!img.dataset.fallbackUsed) {
                                  img.dataset.fallbackUsed = "1";
                                  img.src = getRankedMiniCrestUrl(rank.tier);
                                } else {
                                  img.style.display = "none";
                                }
                              }}
                            />
                          </div>
                          <div className="ranked-info">
                            <span className="ranked-queue-label">{rank.queue_label}</span>
                            <span className="ranked-tier-text">{rankDisplay}</span>
                            {!isUnranked && (
                              <span className="ranked-lp-meta">
                                {rank.league_points} {lpLabel} · {rank.wins}
                                {winShort} {rank.losses}
                                {lossShort} ({rankedWr}%)
                              </span>
                            )}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}

                <div className="profile-hero-actions">
                  <button
                    type="button"
                    className="profile-action-btn primary"
                    data-testid="refresh-profile-btn"
                    onClick={onRefresh}
                    disabled={loading || refreshCooldownSecs > 0}
                  >
                    {loading ? (
                      <>
                        <span>{t("profile.refreshing")}</span>
                        <LoadingDots variant="inline" size="sm" testId="profile-refresh-dots" />
                      </>
                    ) : refreshCooldownSecs > 0 ? (
                      t("profile.refreshCooldown", { secs: refreshCooldownSecs })
                    ) : (
                      t("profile.refresh")
                    )}
                  </button>
                </div>
              </div>
            </div>
          </section>

          {/* Main 2-Column Split Area */}
          <div className="profile-split-grid">
            {/* Left Column: Win/Loss Chart & Recent Form + Top 3 Masteries */}
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
                    {winLossStats.total === 1
                      ? t("common.gamesSingular")
                      : t("common.gamesPlural")}
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
                          <text
                            x="50"
                            y="48"
                            textAnchor="middle"
                            className="donut-center-value"
                          >
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
                          <div
                            className="highlighted-champ-box"
                            data-testid="highlighted-champion"
                          >
                            {highlightedChampion.icon && (
                              <div className="dd-icon-frame highlighted-champ-frame">
                                <img
                                  src={highlightedChampion.icon}
                                  alt={highlightedChampion.name}
                                  className="highlighted-champ-icon"
                                />
                              </div>
                            )}
                            <div className="highlighted-champ-text">
                              <span className="highlighted-label">
                                {t("profile.highlightLabel")}
                              </span>
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

                    {/* Idea 3: Recent Form Streak Pills */}
                    <div className="recent-streak-block" data-testid="recent-streak">
                      <span className="recent-streak-label">{t("profile.recentStreak")}</span>
                      <div className="recent-streak-pills">
                        {filteredMatches.map((m) => (
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
                              src={mastery.champion_icon_url}
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

            {/* Right Column: Mode Filters + Scrollable Last 10 Matches */}
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
                      onClick={() => setSelectedMode("all")}
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
                        onClick={() => setSelectedMode(mode.key)}
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
                            onClick={() =>
                              setExpandedGameId(isExpanded ? null : match.game_id)
                            }
                            aria-expanded={isExpanded}
                          >
                            <div className="match-champ-col">
                              {match.champion_icon_url ? (
                                <div className="dd-icon-frame match-champ-frame">
                                  <img
                                    src={match.champion_icon_url}
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
                                      src={match.spell1_icon_url}
                                      alt={t("profile.spell1Alt")}
                                      className="match-spell-icon"
                                    />
                                  </div>
                                )}
                                {match.spell2_icon_url && (
                                  <div className="dd-icon-frame match-spell-frame">
                                    <img
                                      src={match.spell2_icon_url}
                                      alt={t("profile.spell2Alt")}
                                      className="match-spell-icon"
                                    />
                                  </div>
                                )}
                              </div>
                            </div>

                            <div className="match-main-info">
                              <div className="match-top-line">
                                <span
                                  className={`match-result-tag ${match.win ? "win" : "loss"}`}
                                >
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
                                {multiKill && (
                                  <span className="match-multikill-pill">{multiKill}</span>
                                )}
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

                            <div className="match-items-grid" aria-label={t("profile.finalItemsAria")}>
                              {Array.from({ length: 6 }).map((_, slotIdx) => {
                                const iconUrl = match.item_icon_urls[slotIdx] || "";
                                const itemId = match.item_ids[slotIdx] || 0;
                                return (
                                  <div key={slotIdx} className="match-item-slot">
                                    {itemId > 0 && iconUrl ? (
                                      <img
                                        src={iconUrl}
                                        alt={t("common.itemAlt", { id: itemId })}
                                        className="match-item-img"
                                      />
                                    ) : (
                                      <span className="match-item-empty" />
                                    )}
                                  </div>
                                );
                              })}
                            </div>
                          </button>

                          {/* Idea 5: Inline Expandable Details */}
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
          </div>
        </div>
      )}
    </div>
  );
}

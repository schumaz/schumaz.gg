import { CSSProperties, useMemo, useState } from "react";
import { PlayerProfile, RecentMatchInfo, SummonerInfo } from "../types";
import { profileBlurPercentToPx } from "../hooks/usePreferences";

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

function formatPoints(points: number): string {
  return points.toLocaleString("pt-BR");
}

function formatDuration(secs: number): string {
  const mins = Math.floor(Math.max(0, secs) / 60);
  const rem = Math.max(0, secs) % 60;
  return `${mins}m ${rem.toString().padStart(2, "0")}s`;
}

function formatRelativeTime(creationMs: number): string {
  if (!creationMs) return "";
  const diffMs = Date.now() - creationMs;
  const mins = Math.floor(diffMs / 60000);
  if (mins < 60) return `há ${Math.max(1, mins)}min`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `há ${hours}h`;
  const days = Math.floor(hours / 24);
  return `há ${days}d`;
}

function formatCompactNumber(val: number): string {
  if (val >= 1000) {
    return `${(val / 1000).toFixed(1).replace(".", ",")}k`;
  }
  return String(val);
}

function formatKdaRatio(kills: number, deaths: number, assists: number): string {
  if (deaths === 0) return "Perfeito";
  return `${((kills + assists) / deaths).toFixed(2)} KDA`;
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
  const [selectedMode, setSelectedMode] = useState<string>("all");
  const [expandedGameId, setExpandedGameId] = useState<number | null>(null);

  // Defensive exclusion of Practice Tool on frontend as well
  const validMatches = useMemo(() => {
    if (!profile?.recent_matches) return [];
    return profile.recent_matches.filter(
      (m) =>
        m.queue_id >= 0 &&
        m.game_mode_key.toLowerCase() !== "practicetool" &&
        !m.game_mode_label.toLowerCase().includes("treino")
    );
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

  const refreshBtnLabel = loading
    ? "Atualizando..."
    : refreshCooldownSecs > 0
      ? `Atualizar (${refreshCooldownSecs}s)`
      : "Atualizar";

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
        <div className="idle-state-container" data-testid="profile-waiting-client">
          <section className="card idle-card" style={glassBlurStyle}>
            <div className="idle-icon-box">
              <svg
                viewBox="0 0 24 24"
                width="48"
                height="48"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
                <circle cx="12" cy="7" r="4" />
              </svg>
            </div>
            <h2>Aguardando Cliente do LoL</h2>
            <p className="card-desc">
              Abra o cliente do League of Legends para carregar seu perfil, maestrias e partidas recentes.
            </p>
            {errorMsg && (
              <div className="alert alert-error" role="alert">
                <strong>Erro: </strong> {errorMsg}
              </div>
            )}
          </section>
        </div>
      ) : loading && !profile ? (
        <div className="idle-state-container" data-testid="profile-loading">
          <section className="card idle-card" style={glassBlurStyle}>
            <div className="idle-icon-box">
              <svg
                viewBox="0 0 24 24"
                width="44"
                height="44"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
                <circle cx="12" cy="7" r="4" />
              </svg>
            </div>
            <h2>Carregando Perfil</h2>
            <p className="card-desc">
              Obtendo ícone, nível, wallpaper do perfil, maestrias e histórico recente...
            </p>
          </section>
        </div>
      ) : (
        <div className="profile-layout" data-testid="profile-loaded">
          {/* Top Profile Hero Card */}
          <section className="card profile-hero-card" style={glassBlurStyle}>
            <div className="profile-hero-main">
              <div className="profile-identity">
                <div className="profile-avatar-wrap">
                  {profile?.profile_icon_url ? (
                    <div className="dd-icon-frame profile-avatar-frame">
                      <img
                        src={profile.profile_icon_url}
                        alt="Ícone do Invocador"
                        className="profile-avatar-img"
                      />
                    </div>
                  ) : (
                    <div className="profile-avatar-placeholder">
                      {(summoner.gameName || summoner.displayName || "?").charAt(0).toUpperCase()}
                    </div>
                  )}
                  <span className="profile-level-badge" data-testid="profile-level">
                    Nv. {summoner.summonerLevel}
                  </span>
                </div>

                <div className="profile-name-block">
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

                  {/* Idea 1: Ranked Queue Cards (Solo/Duo & Flex) */}
                  <div className="profile-ranked-row" data-testid="profile-ranked-row">
                    {(profile?.ranked_stats ?? []).map((rank) => {
                      const isUnranked = rank.tier === "UNRANKED";
                      const totalRanked = rank.wins + rank.losses;
                      const rankedWr =
                        totalRanked > 0 ? Math.round((rank.wins / totalRanked) * 100) : 0;

                      return (
                        <div
                          key={rank.queue_type}
                          className={`ranked-pill ${isUnranked ? "unranked" : ""}`}
                          data-testid={`ranked-pill-${rank.queue_type}`}
                        >
                          <span className="ranked-queue-label">{rank.queue_label}</span>
                          <span className="ranked-tier-text">
                            {isUnranked
                              ? "Sem Ranque"
                              : `${rank.tier_label}${rank.division ? ` ${rank.division}` : ""}`}
                          </span>
                          {!isUnranked && (
                            <span className="ranked-lp-meta">
                              {rank.league_points} PDL · {rank.wins}V {rank.losses}D ({rankedWr}%)
                            </span>
                          )}
                        </div>
                      );
                    })}
                  </div>
                </div>
              </div>

              <div className="profile-hero-actions">
                <button
                  type="button"
                  className="profile-action-btn primary"
                  data-testid="refresh-profile-btn"
                  onClick={onRefresh}
                  disabled={loading || refreshCooldownSecs > 0}
                >
                  {refreshBtnLabel}
                </button>
              </div>
            </div>
          </section>

          {/* Main 2-Column Split Area */}
          <div className="profile-split-grid">
            {/* Left Column: Top 3 Masteries + Win/Loss Chart & Recent Form */}
            <div className="profile-col profile-col-left">
              <section className="card profile-section-card" data-testid="top-masteries-card" style={glassBlurStyle}>
                <div className="profile-card-header">
                  <h3>Top 3 Maestrias</h3>
                  <span className="profile-card-subtitle">Maiores pontuações</span>
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
                            {formatPoints(mastery.champion_points)} pts
                          </span>
                        </div>
                        <span className="mastery-level-pill">Nv. {mastery.champion_level}</span>
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="text-muted">Nenhuma maestria encontrada.</p>
                )}
              </section>

              <section className="card profile-section-card" data-testid="winloss-chart-card" style={glassBlurStyle}>
                <div className="profile-card-header">
                  <h3>Desempenho Win / Loss</h3>
                  <span className="profile-card-subtitle">
                    {winLossStats.total} {winLossStats.total === 1 ? "partida" : "partidas"}
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
                          aria-label={`Taxa de vitória ${winLossStats.winRate}% (${winLossStats.wins} vitórias e ${winLossStats.losses} derrotas)`}
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
                            Win Rate
                          </text>
                        </svg>
                      </div>

                      <div className="winloss-metrics">
                        <div className="winloss-stat-line">
                          <span className="winloss-dot win" />
                          <span>Vitórias</span>
                          <strong data-testid="win-count">{winLossStats.wins}</strong>
                        </div>
                        <div className="winloss-stat-line">
                          <span className="winloss-dot loss" />
                          <span>Derrotas</span>
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
                              <span className="highlighted-label">Destaque</span>
                              <strong>{highlightedChampion.name}</strong>
                              <span className="highlighted-sub">
                                {highlightedChampion.games}J ·{" "}
                                {Math.round(
                                  (highlightedChampion.wins / highlightedChampion.games) * 100
                                )}
                                % WR
                              </span>
                            </div>
                          </div>
                        )}
                      </div>
                    </div>

                    {/* Idea 3: Recent Form Streak Pills */}
                    <div className="recent-streak-block" data-testid="recent-streak">
                      <span className="recent-streak-label">Sequência recente</span>
                      <div className="recent-streak-pills">
                        {filteredMatches.map((m) => (
                          <span
                            key={m.game_id}
                            className={`streak-pill ${m.win ? "win" : "loss"}`}
                            title={`${m.champion_name} (${m.kills}/${m.deaths}/${m.assists}) — ${
                              m.win ? "Vitória" : "Derrota"
                            }`}
                          >
                            {m.win ? "V" : "D"}
                          </span>
                        ))}
                      </div>
                    </div>
                  </>
                ) : (
                  <p className="text-muted">Sem partidas para o filtro selecionado.</p>
                )}
              </section>
            </div>

            {/* Right Column: Mode Filters + Scrollable Last 10 Matches */}
            <div className="profile-col profile-col-right">
              <section className="card profile-matches-card" style={glassBlurStyle}>
                <div className="profile-matches-header">
                  <div>
                    <h3>Últimas 10 Partidas</h3>
                    <span className="profile-card-subtitle">
                      Exibindo {filteredMatches.length} de {validMatches.length} partidas
                    </span>
                  </div>

                  <div
                    className="mode-filter-chips"
                    role="group"
                    aria-label="Filtrar partidas por modo de jogo"
                    data-testid="match-mode-filters"
                  >
                    <button
                      type="button"
                      className={`mode-chip ${activeMode === "all" ? "active" : ""}`}
                      onClick={() => setSelectedMode("all")}
                      aria-pressed={activeMode === "all"}
                      data-testid="mode-filter-all"
                    >
                      Todos ({validMatches.length})
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
                                      alt="Feitiço 1"
                                      className="match-spell-icon"
                                    />
                                  </div>
                                )}
                                {match.spell2_icon_url && (
                                  <div className="dd-icon-frame match-spell-frame">
                                    <img
                                      src={match.spell2_icon_url}
                                      alt="Feitiço 2"
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
                                  {match.win ? "Vitória" : "Derrota"}
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
                                {formatKdaRatio(match.kills, match.deaths, match.assists)} ·{" "}
                                {match.cs} CS
                              </span>
                            </div>

                            <div className="match-items-grid" aria-label="Itens finais">
                              {Array.from({ length: 6 }).map((_, slotIdx) => {
                                const iconUrl = match.item_icon_urls[slotIdx] || "";
                                const itemId = match.item_ids[slotIdx] || 0;
                                return (
                                  <div key={slotIdx} className="match-item-slot">
                                    {itemId > 0 && iconUrl ? (
                                      <img
                                        src={iconUrl}
                                        alt={`Item ${itemId}`}
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
                                <span>Nível</span>
                                <strong>{match.champ_level}</strong>
                              </div>
                              <div className="match-detail-stat">
                                <span>Dano</span>
                                <strong>
                                  {formatCompactNumber(match.total_damage_dealt_to_champions)}
                                </strong>
                              </div>
                              <div className="match-detail-stat">
                                <span>Ouro</span>
                                <strong>{formatCompactNumber(match.gold_earned)}</strong>
                              </div>
                              <div className="match-detail-stat">
                                <span>Visão</span>
                                <strong>
                                  {match.vision_score} ({match.wards_placed} sent.)
                                </strong>
                              </div>
                              <div className="match-detail-stat">
                                <span>CS/min</span>
                                <strong>{csPerMin}</strong>
                              </div>
                            </div>
                          )}
                        </div>
                      );
                    })
                  ) : (
                    <div className="matches-empty">
                      <p className="text-muted">Nenhuma partida encontrada para este modo.</p>
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

import { RecentMatchInfo } from "../../types";
import { useTranslation } from "../../i18n";
import type { ProfileChampionTarget } from "./profileHelpers";
import { ScoreboardEntryState } from "./useMatchScoreboard";
import { ScoreboardArenaGrid } from "./ScoreboardArenaGrid";
import { ScoreboardTeamTable } from "./ScoreboardTeamTable";

interface MatchScoreboardDrawerProps {
  match: RecentMatchInfo;
  entry: ScoreboardEntryState;
  isLcuConnected: boolean;
  onRetry: () => void;
  onOpenChampionBuild?: (target: ProfileChampionTarget) => void;
}

function MatchIndividualSummary({ match }: { match: RecentMatchInfo }) {
  const { t, formatCompactNumber } = useTranslation();
  const mins = Math.max(1, match.game_duration_secs / 60);
  const csPerMin = (match.cs / mins).toFixed(1);

  return (
    <div className="sb-individual-summary">
      <div className="match-detail-stat">
        <span>{t("profile.detailLevel")}</span>
        <strong>{match.champ_level}</strong>
      </div>
      <div className="match-detail-stat">
        <span>{t("profile.detailDamage")}</span>
        <strong>{formatCompactNumber(match.total_damage_dealt_to_champions)}</strong>
      </div>
      <div className="match-detail-stat">
        <span>{t("profile.detailGold")}</span>
        <strong>{formatCompactNumber(match.gold_earned)}</strong>
      </div>
      <div className="match-detail-stat">
        <span>{t("profile.detailVision")}</span>
        <strong>
          {match.vision_score} ({t("profile.wardsPlacedShort", { count: match.wards_placed })})
        </strong>
      </div>
      <div className="match-detail-stat">
        <span>{t("profile.detailCsPerMin")}</span>
        <strong>{csPerMin}</strong>
      </div>
    </div>
  );
}

function ScoreboardLoadingSkeleton({ gameId }: { gameId: number }) {
  const { t } = useTranslation();
  return (
    <div
      className="sb-loading-skeleton"
      data-testid={`scoreboard-skeleton-${gameId}`}
      aria-busy="true"
    >
      <div className="sb-loading-caption">{t("profile.scoreboardLoading")}</div>
      <div className="skeleton-list">
        {Array.from({ length: 4 }).map((_, idx) => (
          <div key={idx} className="skeleton-row sb-skeleton-row">
            <div className="skeleton-block sb-skeleton-avatar" />
            <div className="skeleton-block skeleton-line sb-skeleton-name" />
            <div className="skeleton-block skeleton-line sb-skeleton-kda" />
            <div className="skeleton-block skeleton-line sb-skeleton-bar" />
          </div>
        ))}
      </div>
    </div>
  );
}

export function MatchScoreboardDrawer({
  match,
  entry,
  isLcuConnected,
  onRetry,
  onOpenChampionBuild,
}: MatchScoreboardDrawerProps) {
  const { t } = useTranslation();
  const arenaTeams = entry.status === "ready" ? (entry.data.arena_teams ?? []) : [];
  const standardTeams = entry.status === "ready" ? (entry.data.teams ?? []) : [];
  const hasReadyTeams =
    entry.status === "ready" &&
    ((entry.data.is_arena && arenaTeams.length > 0) ||
      (!entry.data.is_arena && standardTeams.length > 0));

  return (
    <div
      className="match-details-drawer match-scoreboard-drawer"
      data-testid={`match-details-${match.game_id}`}
    >
      <MatchIndividualSummary match={match} />

      {entry.status === "ready" && hasReadyTeams ? (
        entry.data.is_arena ? (
          <ScoreboardArenaGrid
            scoreboard={entry.data}
            onOpenChampionBuild={onOpenChampionBuild}
          />
        ) : (
          <ScoreboardTeamTable
            scoreboard={entry.data}
            onOpenChampionBuild={onOpenChampionBuild}
          />
        )
      ) : entry.status === "loading" || entry.status === "idle" ? (
        isLcuConnected ? (
          <ScoreboardLoadingSkeleton gameId={match.game_id} />
        ) : (
          <div
            className="sb-notice is-offline"
            data-testid={`scoreboard-fallback-notice-${match.game_id}`}
          >
            {t("profile.scoreboardOfflineNotice")}
          </div>
        )
      ) : entry.status === "offline" || !isLcuConnected ? (
        <div
          className="sb-notice is-offline"
          data-testid={`scoreboard-fallback-notice-${match.game_id}`}
        >
          {t("profile.scoreboardOfflineNotice")}
        </div>
      ) : (
        <div
          className="sb-notice is-error"
          role="alert"
          data-testid={`scoreboard-fallback-notice-${match.game_id}`}
        >
          <span>{t("profile.scoreboardErrorNotice")}</span>
          <button
            type="button"
            className="sb-retry-btn"
            data-testid={`scoreboard-retry-${match.game_id}`}
            onClick={onRetry}
          >
            {t("profile.scoreboardRetry")}
          </button>
        </div>
      )}
    </div>
  );
}

import { MatchScoreboardDetail } from "../../types";
import { useTranslation } from "../../i18n";
import type { ProfileChampionTarget } from "./profileHelpers";
import {
  ScoreboardChampCell,
  ScoreboardDamageBar,
  ScoreboardItemsStrip,
  ScoreboardPlayerName,
} from "./ScoreboardSharedCells";
import { ScoreboardLoadoutCell } from "./ScoreboardLoadoutCell";

interface ScoreboardTeamTableProps {
  scoreboard: MatchScoreboardDetail;
  onOpenChampionBuild?: (target: ProfileChampionTarget) => void;
}

export function ScoreboardTeamTable({
  scoreboard,
  onOpenChampionBuild,
}: ScoreboardTeamTableProps) {
  const { t, formatCompactNumber } = useTranslation();
  const mins = Math.max(1, scoreboard.game_duration_secs / 60);
  const isSrMode = !scoreboard.game_mode_key.startsWith("aram");
  const teams = scoreboard.teams ?? [];

  return (
    <div className="sb-teams-stack">
      {teams.map((team) => {
        const teamLabel =
          team.team_id === 100
            ? t("profile.blueTeam")
            : team.team_id === 200
              ? t("profile.redTeam")
              : t("profile.teamFallback", { id: team.team_id });
        const sideClass = team.team_id === 100 ? "is-blue" : team.team_id === 200 ? "is-red" : "";
        const resultClass = scoreboard.remake ? "remake" : team.win ? "win" : "loss";
        const resultLabel = scoreboard.remake
          ? t("profile.remake")
          : team.win
            ? t("profile.victory")
            : t("profile.defeat");

        const voidgrubs = team.voidgrubs ?? 0;
        const objectives: { label: string; count: number }[] = [{ label: t("profile.objTowers"), count: team.towers }];
        if (isSrMode || team.dragons > 0) objectives.push({ label: t("profile.objDragons"), count: team.dragons });
        if (isSrMode || team.barons > 0) objectives.push({ label: t("profile.objBarons"), count: team.barons });
        if (team.heralds > 0) objectives.push({ label: t("profile.objHeralds"), count: team.heralds });
        if (voidgrubs > 0) objectives.push({ label: t("profile.objVoidgrubs"), count: voidgrubs });
        if (team.inhibitors > 0) objectives.push({ label: t("profile.objInhibitors"), count: team.inhibitors });

        return (
          <section
            key={team.team_id}
            className={`sb-team-block ${sideClass}`}
            data-testid={`scoreboard-team-${team.team_id}`}
          >
            <header className="sb-team-header">
              <div className="sb-team-title-group">
                <span className={`sb-team-dot ${sideClass}`} />
                <span className="sb-team-name">{teamLabel}</span>
                <span className={`sb-team-result ${resultClass}`}>{resultLabel}</span>
              </div>

              <div className="sb-team-meta-group">
                <span className="sb-team-kda">
                  {team.total_kills} / {team.total_deaths} / {team.total_assists}
                </span>
                <span className="sb-team-gold">{formatCompactNumber(team.total_gold)}g</span>
                <div className="sb-objectives-list">
                  {objectives.map((obj) => (
                    <span key={obj.label} className="sb-obj-pill">
                      {obj.label} <strong>{obj.count}</strong>
                    </span>
                  ))}
                </div>
              </div>
            </header>

            <div className="sb-team-columns" aria-hidden="true">
              <span>{t("profile.colPlayer")}</span>
              <span className="is-center">{t("profile.colKda")}</span>
              <span>{t("profile.colDamage")}</span>
              <span className="is-right">{t("profile.colCs")}</span>
              <span className="is-right">{t("profile.colVision")}</span>
              <span className="is-right">{t("profile.colItems")}</span>
            </div>

            <div className="sb-team-rows">
              {team.participants.map((p) => {
                const ratio =
                  p.deaths === 0
                    ? t("profile.perfectKda")
                    : ((p.kills + p.assists) / p.deaths).toFixed(1);
                const csPerMin = (p.cs / mins).toFixed(1);

                return (
                  <div
                    key={p.participant_id}
                    className={`sb-player-row${p.is_local_player ? " is-local-player" : ""}`}
                    data-testid={`scoreboard-player-${p.participant_id}`}
                  >
                    <div className="sb-player-identity">
                      <ScoreboardChampCell
                        participant={p}
                        onOpenChampionBuild={onOpenChampionBuild}
                      />
                      <ScoreboardLoadoutCell
                        participant={p}
                        runeMetadata={scoreboard.rune_metadata}
                        spellMetadata={scoreboard.spell_metadata}
                        augmentMetadata={scoreboard.augment_metadata}
                      />
                      <ScoreboardPlayerName
                        participant={p}
                        onOpenChampionBuild={onOpenChampionBuild}
                      />
                    </div>

                    <div className="sb-kda-cell">
                      <span className="sb-kda-line">
                        {p.kills} / <span className="deaths">{p.deaths}</span> / {p.assists}
                      </span>
                      <span className="sb-kda-sub">
                        {ratio} · {t("profile.kpShort", { kp: p.kp_percent })}
                      </span>
                    </div>

                    <ScoreboardDamageBar
                      participantId={p.participant_id}
                      damage={p.total_damage_dealt_to_champions}
                      maxDamage={scoreboard.max_damage_dealt_to_champions}
                    />

                    <div className="sb-metric-cell">
                      <strong>{p.cs}</strong>
                      <span>{t("profile.csPerMinShort", { rate: csPerMin })}</span>
                    </div>

                    <div className="sb-metric-cell">
                      <strong>{p.vision_score}</strong>
                      <span>{t("profile.wardsPlacedShort", { count: p.wards_placed })}</span>
                    </div>

                    <ScoreboardItemsStrip
                      itemIds={p.item_ids}
                      itemIconUrls={p.item_icon_urls}
                      trinketId={p.trinket_id ?? 0}
                      trinketIconUrl={p.trinket_icon_url ?? ""}
                      itemMetadata={scoreboard.item_metadata}
                      showTrinket={true}
                    />
                  </div>
                );
              })}
            </div>
          </section>
        );
      })}
    </div>
  );
}

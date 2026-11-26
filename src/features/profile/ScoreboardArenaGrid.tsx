import { MatchScoreboardDetail } from "../../types";
import { useTranslation } from "../../i18n";
import { formatArenaPlacement, getArenaPlacementTone } from "./profileHelpers";
import {
  ScoreboardChampCell,
  ScoreboardDamageBar,
  ScoreboardItemsStrip,
  ScoreboardPlayerName,
} from "./ScoreboardSharedCells";
import { ScoreboardAugmentsStrip } from "./ScoreboardLoadoutCell";

interface ScoreboardArenaGridProps {
  scoreboard: MatchScoreboardDetail;
}

export function ScoreboardArenaGrid({ scoreboard }: ScoreboardArenaGridProps) {
  const { t, locale, formatCompactNumber } = useTranslation();
  const arenaTeams = scoreboard.arena_teams ?? [];

  return (
    <div className="sb-arena-grid">
      {arenaTeams.map((team, idx) => {
        const placement = team.placement > 0 ? team.placement : idx + 1;
        const hasLocal = team.participants.some((p) => p.is_local_player);
        const tone = getArenaPlacementTone(placement);

        return (
          <section
            key={`${team.subteam_id}-${placement}`}
            className={`sb-arena-card${hasLocal ? " is-local-duo" : ""}`}
            data-testid={`scoreboard-arena-team-${placement}`}
          >
            <header className="sb-arena-header">
              <div className="sb-arena-title">
                <span className={`match-placement-pill tone-${tone}`}>
                  {formatArenaPlacement(placement, locale)}
                </span>
                <span className="sb-arena-duo-label">{t("profile.arenaDuoLabel")}</span>
              </div>
              <div className="sb-arena-stats">
                <span>
                  {team.total_kills} / {team.total_deaths} / {team.total_assists}
                </span>
                <span>{formatCompactNumber(team.total_damage)}</span>
              </div>
            </header>

            <div className="sb-arena-rows">
              {team.participants.map((p) => (
                <div
                  key={p.participant_id}
                  className={`sb-arena-player-row${p.is_local_player ? " is-local-player" : ""}`}
                  data-testid={`scoreboard-player-${p.participant_id}`}
                >
                  <div className="sb-arena-identity">
                    <ScoreboardChampCell participant={p} />
                    <ScoreboardPlayerName participant={p} />
                  </div>

                  <div className="sb-kda-cell">
                    <span className="sb-kda-line">
                      {p.kills} / <span className="deaths">{p.deaths}</span> / {p.assists}
                    </span>
                  </div>

                  <ScoreboardDamageBar
                    participantId={p.participant_id}
                    damage={p.total_damage_dealt_to_champions}
                    maxDamage={scoreboard.max_damage_dealt_to_champions}
                  />

                  <ScoreboardAugmentsStrip
                    augmentIds={p.augment_ids ?? []}
                    augmentMetadata={scoreboard.augment_metadata}
                  />

                  <ScoreboardItemsStrip
                    itemIds={p.item_ids}
                    itemIconUrls={p.item_icon_urls}
                    trinketId={p.trinket_id ?? 0}
                    trinketIconUrl={p.trinket_icon_url ?? ""}
                    itemMetadata={scoreboard.item_metadata}
                    showTrinket={(p.trinket_id ?? 0) > 0 && Boolean(p.trinket_icon_url)}
                  />
                </div>
              ))}
            </div>
          </section>
        );
      })}
    </div>
  );
}

import { LaneMatchupEntry, LaneMatchupsSummary } from "../../types";
import { useTranslation } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";

interface LaneMatchupsCardProps {
  matchups: LaneMatchupsSummary;
  championName?: string;
  laneLabel?: string;
  onSelectChampion?: (championId: number) => void;
  onClose?: () => void;
}

/**
 * Pure visual card displaying Top 5 Hard counters and Top 5 Favorable lane matchups.
 * Interactive champion rows are rendered only when `onSelectChampion` is provided.
 */
export function LaneMatchupsCard({
  matchups,
  championName,
  laneLabel,
  onSelectChampion,
  onClose,
}: LaneMatchupsCardProps) {
  const { t, formatNumber } = useTranslation();

  const hardList = (matchups.hard ?? []).slice(0, 5);
  const favorableList = (matchups.favorable ?? []).slice(0, 5);

  const renderColumn = (
    title: string,
    items: LaneMatchupEntry[],
    side: "hard" | "favorable",
    colTestId: string
  ) => (
    <div className="lane-matchups-col" data-testid={colTestId}>
      <div className={`lane-matchups-col-header is-${side}`}>
        <h3 className="lane-matchups-col-title">{title}</h3>
        <span className="lane-matchups-col-count">{items.length}</span>
      </div>

      {items.length === 0 ? (
        <div className="lane-matchups-empty">{t("matchups.emptySide")}</div>
      ) : (
        <div className="lane-matchups-list">
          {items.map((entry) => {
            const name = entry.name?.trim() || `#${entry.champion_id}`;
            const winRate = typeof entry.win_rate === "number" ? entry.win_rate : 0;
            const play = typeof entry.play === "number" ? entry.play : 0;
            const wrToneClass = winRate < 50 ? "is-hard" : "is-favorable";
            const gamesLabel = t(play === 1 ? "common.gamesSingular" : "common.gamesPlural");

            const rowBody = (
              <>
                <div className="dd-icon-frame lane-matchup-avatar-frame">
                  {entry.icon_url ? (
                    <img
                      src={resolveCachedAssetUrl(entry.icon_url)}
                      alt={name}
                      className="lane-matchup-avatar"
                      loading="lazy"
                      decoding="async"
                      onError={(e) => {
                        (e.currentTarget as HTMLElement).style.display = "none";
                      }}
                    />
                  ) : (
                    <span className="lane-matchup-avatar-fallback" aria-hidden="true">
                      {name.slice(0, 2).toUpperCase()}
                    </span>
                  )}
                </div>

                <span className="lane-matchup-name" title={name}>
                  {name}
                </span>

                <div className="lane-matchup-metrics">
                  <span className={`lane-matchup-wr ${wrToneClass}`}>
                    {winRate.toFixed(1)}%
                  </span>
                  <span className="lane-matchup-games">
                    {formatNumber(play)} {gamesLabel}
                  </span>
                </div>
              </>
            );

            if (onSelectChampion) {
              return (
                <button
                  key={`${side}-${entry.champion_id}`}
                  type="button"
                  className="lane-matchup-row is-interactive"
                  data-testid={`lane-matchup-row-${entry.champion_id}`}
                  title={t("matchups.openBuildHint", { name })}
                  onClick={() => onSelectChampion(entry.champion_id)}
                >
                  {rowBody}
                </button>
              );
            }

            return (
              <div
                key={`${side}-${entry.champion_id}`}
                className="lane-matchup-row is-readonly"
                data-testid={`lane-matchup-row-${entry.champion_id}`}
              >
                {rowBody}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );

  return (
    <section className="card lane-matchups-card" data-testid="lane-matchups-card">
      <div className="lane-matchups-header">
        <div className="lane-matchups-title-group">
          <h2>{t("matchups.title")}</h2>
          {laneLabel && <span className="lane-matchups-lane-pill">{laneLabel}</span>}
        </div>

        <div className="lane-matchups-header-actions">
          {championName && (
            <span className="lane-matchups-hint">
              {t("matchups.laneWinRateHint", { name: championName })}
            </span>
          )}
          {onClose && (
            <button
              type="button"
              className="skill-order-close-btn"
              data-testid="lane-matchups-close-btn"
              onClick={onClose}
              aria-label={t("matchups.closePopoverAria")}
            >
              ×
            </button>
          )}
        </div>
      </div>

      <div className="lane-matchups-columns">
        {renderColumn(t("matchups.hardTitle"), hardList, "hard", "lane-matchups-hard")}
        {renderColumn(
          t("matchups.favorableTitle"),
          favorableList,
          "favorable",
          "lane-matchups-favorable"
        )}
      </div>
    </section>
  );
}

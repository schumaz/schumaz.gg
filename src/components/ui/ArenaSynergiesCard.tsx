import type { ArenaSynergyEntry } from "../../types";
import { useTranslation } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";

export interface ArenaSynergiesCardProps {
  synergies: ArenaSynergyEntry[];
  maxVisible?: number;
  onSelectChampion?: (championId: number) => void;
}

/**
 * Shared Arena (`CHERRY`) duo partner synergies card displaying top paired champions
 * with Top 4 and 1st place win metrics.
 */
export function ArenaSynergiesCard({
  synergies,
  maxVisible = 8,
  onSelectChampion,
}: ArenaSynergiesCardProps) {
  const { t } = useTranslation();
  const visibleSynergies = synergies.slice(0, maxVisible);

  return (
    <section className="card arena-synergies-card" data-testid="arena-synergies-card">
      <div className="arena-synergies-header">
        <h2>{t("arena.synergiesTitle")}</h2>
        {visibleSynergies.length > 0 && (
          <span className="arena-count-pill">
            {t("arena.synergiesCount", { count: visibleSynergies.length })}
          </span>
        )}
      </div>

      {visibleSynergies.length === 0 ? (
        <div className="arena-empty-box is-compact" data-testid="arena-synergies-empty">
          <span>{t("arena.synergiesEmpty")}</span>
        </div>
      ) : (
        <div className="arena-synergies-grid" data-testid="arena-synergies-grid">
          {visibleSynergies.map((syn) => {
            const hasTop4 = typeof syn.win_rate === "number" && syn.win_rate > 0;
            const hasFirst = typeof syn.first_place_rate === "number" && syn.first_place_rate > 0;
            const content = (
              <>
                <div className="dd-icon-frame arena-synergy-frame">
                  {syn.icon_url ? (
                    <img
                      src={resolveCachedAssetUrl(syn.icon_url)}
                      alt={syn.name}
                      className="arena-synergy-avatar"
                      loading="lazy"
                      onError={(e) => {
                        (e.target as HTMLElement).style.display = "none";
                      }}
                    />
                  ) : (
                    <span className="arena-icon-fallback">
                      {syn.name.slice(0, 2).toUpperCase()}
                    </span>
                  )}
                </div>

                <div className="arena-synergy-info">
                  <span className="arena-synergy-name" title={syn.name}>
                    {syn.name}
                  </span>
                  <div className="arena-synergy-stats">
                    {hasTop4 && (
                      <span
                        className={`arena-synergy-metric ${(syn.win_rate ?? 0) >= 55 ? "is-high" : ""}`}
                      >
                        {(syn.win_rate ?? 0).toFixed(1)}% {t("arena.top4Short")}
                      </span>
                    )}
                    {hasFirst && (
                      <span className="arena-synergy-metric is-accent">
                        {(syn.first_place_rate ?? 0).toFixed(1)}% {t("arena.firstPlaceShort")}
                      </span>
                    )}
                  </div>
                </div>
              </>
            );

            if (onSelectChampion) {
              return (
                <button
                  key={syn.champion_id}
                  type="button"
                  className="arena-synergy-item is-interactive"
                  data-testid={`arena-synergy-${syn.champion_id}`}
                  onClick={() => onSelectChampion(syn.champion_id)}
                >
                  {content}
                </button>
              );
            }

            return (
              <div
                key={syn.champion_id}
                className="arena-synergy-item"
                data-testid={`arena-synergy-${syn.champion_id}`}
              >
                {content}
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}

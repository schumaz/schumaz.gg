import { CSSProperties, Fragment } from "react";
import { ChampionAbilityMetadata, SkillOrderRecommendation } from "../../types";
import { useTranslation } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { SkillTooltipContent } from "./BuildTooltips";
import { Tooltip } from "./Tooltip";

const ABILITY_ROWS = ["Q", "W", "E", "R"] as const;

interface SkillOrderCardProps {
  skillOrder?: SkillOrderRecommendation | null;
  readOnly?: boolean;
}

/**
 * Pure visual card displaying basic skill maxing priority (`Q > E > W`) and level `1–18` grid.
 */
export function SkillOrderCard({ skillOrder, readOnly = false }: SkillOrderCardProps) {
  const { t, formatNumber } = useTranslation();

  if (!skillOrder) {
    return null;
  }

  const priority = (skillOrder.priority ?? [])
    .map((key) => key.trim().toUpperCase())
    .filter(Boolean);
  const order = (skillOrder.order ?? [])
    .map((key) => key.trim().toUpperCase())
    .filter(Boolean);

  if (priority.length === 0 && order.length === 0) {
    return null;
  }

  const abilitiesByKey = new Map<string, ChampionAbilityMetadata>();
  for (const ability of skillOrder.abilities ?? []) {
    const normalized = ability.key.trim().toUpperCase();
    if (normalized) {
      abilitiesByKey.set(normalized, ability);
    }
  }

  const winRate =
    typeof skillOrder.win_rate === "number" && skillOrder.win_rate > 0
      ? skillOrder.win_rate
      : null;
  const pickRate =
    typeof skillOrder.pick_rate === "number" && skillOrder.pick_rate > 0
      ? skillOrder.pick_rate
      : null;
  const playCount =
    typeof skillOrder.play === "number" && skillOrder.play > 0 ? skillOrder.play : null;

  return (
    <section className="card skill-order-card" data-testid="skill-order-card">
      <div className="skill-order-header">
        <div className="skill-order-title-group">
          <h2>{t("skills.title")}</h2>

          {priority.length > 0 && (
            <div
              className="skill-priority-chain"
              data-testid="skill-priority-chain"
              aria-label={t("skills.priorityAriaLabel")}
            >
              {priority.map((key, idx) => {
                const ability = abilitiesByKey.get(key);
                const label = ability?.name?.trim() || t("skills.abilityFallback", { key });
                return (
                  <Fragment key={`prio-${key}-${idx}`}>
                    {idx > 0 && (
                      <span className="skill-priority-sep" aria-hidden="true">
                        &gt;
                      </span>
                    )}
                    <Tooltip
                      content={<SkillTooltipContent skillKey={key} ability={ability} />}
                      ariaLabel={label}
                    >
                      <span
                        className={`skill-priority-pill skill-key-${key.toLowerCase()}${
                          ability?.icon_url ? "" : " is-text-only"
                        }`}
                      >
                        {ability?.icon_url && (
                          <span className="dd-icon-frame skill-priority-icon-frame">
                            <img
                              src={resolveCachedAssetUrl(ability.icon_url)}
                              alt={label}
                              className="skill-ability-icon"
                              onError={(e) => {
                                (e.currentTarget as HTMLElement).style.display = "none";
                              }}
                            />
                          </span>
                        )}
                        <span className="skill-priority-key">{key}</span>
                      </span>
                    </Tooltip>
                  </Fragment>
                );
              })}
            </div>
          )}
        </div>

        {(winRate !== null || pickRate !== null || playCount !== null) && (
          <div className="skill-order-stats" data-testid="skill-order-stats">
            {winRate !== null && (
              <span className={`skill-stat-pill ${winRate >= 52 ? "is-high" : ""}`}>
                {t("skills.winRateShort", { rate: winRate.toFixed(1) })}
              </span>
            )}
            {pickRate !== null ? (
              <span className="skill-stat-pill">
                {`${pickRate.toFixed(1)}% ${
                  readOnly ? t("items.usageLabel") : t("items.pickLabel")
                }`}
              </span>
            ) : (
              playCount !== null && (
                <span className="skill-stat-pill">
                  {`${formatNumber(playCount)} ${t(
                    playCount === 1 ? "common.gamesSingular" : "common.gamesPlural"
                  )}`}
                </span>
              )
            )}
          </div>
        )}
      </div>

      {order.length > 0 && (
        <div className="skill-order-matrix-wrap">
          <div
            className="skill-order-grid"
            data-testid="skill-order-grid"
            aria-label={t("skills.gridAriaLabel")}
            style={{ "--skill-levels": order.length } as CSSProperties}
          >
            <span className="skill-level-corner" aria-hidden="true">
              {t("skills.levelHeader")}
            </span>
            {order.map((_pick, idx) => (
              <span key={`lvl-hdr-${idx + 1}`} className="skill-level-num" aria-hidden="true">
                {idx + 1}
              </span>
            ))}

            {ABILITY_ROWS.map((rowKey) => {
              const ability = abilitiesByKey.get(rowKey);
              const abilityLabel =
                ability?.name?.trim() || t("skills.abilityFallback", { key: rowKey });

              return (
                <Fragment key={`row-${rowKey}`}>
                  <Tooltip
                    content={<SkillTooltipContent skillKey={rowKey} ability={ability} />}
                    ariaLabel={abilityLabel}
                  >
                    <span className={`skill-row-label skill-key-${rowKey.toLowerCase()}`}>
                      {ability?.icon_url ? (
                        <span className="dd-icon-frame skill-row-icon-frame">
                          <img
                            src={resolveCachedAssetUrl(ability.icon_url)}
                            alt={abilityLabel}
                            className="skill-ability-icon"
                            onError={(e) => {
                              (e.currentTarget as HTMLElement).style.display = "none";
                            }}
                          />
                        </span>
                      ) : (
                        <span className="skill-row-key-fallback">{rowKey}</span>
                      )}
                    </span>
                  </Tooltip>

                  {order.map((pickedKey, lvlIdx) => {
                    const level = lvlIdx + 1;
                    const isActive = pickedKey === rowKey;
                    if (!isActive) {
                      return (
                        <span
                          key={`${rowKey}-${level}`}
                          className="skill-cell"
                          aria-hidden="true"
                        />
                      );
                    }
                    return (
                      <span
                        key={`${rowKey}-${level}`}
                        className={`skill-cell is-active skill-key-${rowKey.toLowerCase()}`}
                        data-testid={`skill-level-${level}`}
                        data-skill={rowKey}
                        aria-label={t("skills.levelLabel", { level, skill: abilityLabel })}
                      >
                        {rowKey}
                      </span>
                    );
                  })}
                </Fragment>
              );
            })}
          </div>
        </div>
      )}
    </section>
  );
}

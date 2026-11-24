import { Fragment } from "react";
import { SpellMetadata } from "../../types";
import { useTranslation } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { SpellTooltipContent } from "./BuildTooltips";
import { Tooltip } from "./Tooltip";

interface SpellsBarProps {
  orderedSpells: SpellMetadata[];
  spellChoices?: SpellMetadata[];
  selectedSecondarySpellId: number | null;
  onInvertSpells?: () => void;
  onSelectSecondarySpell?: (spellId: number) => void;
  readOnly?: boolean;
}

/**
 * Compact Summoner Spells bar with single squircle icons, natural-width names, and inline options.
 */
export function SpellsBar({
  orderedSpells,
  spellChoices,
  selectedSecondarySpellId,
  onInvertSpells,
  onSelectSecondarySpell,
  readOnly = false,
}: SpellsBarProps) {
  const { t } = useTranslation();

  const renderSwapButton = () => {
    if (readOnly || !onInvertSpells) return null;
    return (
      <button
        type="button"
        className="invert-spells-btn"
        data-testid="invert-spells-btn"
        onClick={onInvertSpells}
        title={t("spells.invertTitle")}
        aria-label={t("spells.invertBtn")}
      >
        <span className="invert-spells-icon" aria-hidden="true">
          ⇄
        </span>
      </button>
    );
  };

  return (
    <section className="card spells-card" data-testid="spells-card">
      <div className="spells-bar-row">
        <div className="spells-primary-group">
          <span className="spells-inline-title">{t("spells.shortTitle")}</span>

          <div className="spells-list">
            {orderedSpells && orderedSpells.length >= 2 ? (
              orderedSpells.slice(0, 2).map((spell, index) => (
                <Fragment key={`spell-${index}-${spell.id}`}>
                  {index === 1 && renderSwapButton()}
                  <Tooltip
                    content={<SpellTooltipContent spell={spell} />}
                    ariaLabel={spell.name}
                  >
                    <div className="spell-item">
                      {spell.icon_url && (
                        <div className="spell-icon-anchor">
                          <div className="dd-icon-frame spell-icon-frame">
                            <img
                              src={resolveCachedAssetUrl(spell.icon_url)}
                              alt={spell.name}
                              className="spell-icon"
                            />
                          </div>
                          {!readOnly && (
                            <span className="spell-key-indicator">
                              {index === 0 ? "D" : "F"}
                            </span>
                          )}
                        </div>
                      )}
                      {!spell.icon_url && !readOnly && (
                        <span className="spell-key-inline">{index === 0 ? "D" : "F"}</span>
                      )}
                      <span className="spell-name">{spell.name}</span>
                    </div>
                  </Tooltip>
                </Fragment>
              ))
            ) : (
              <>
                <div className="spell-item">
                  {!readOnly && <span className="spell-key-inline">D</span>}
                  <span className="spell-name">{t("spells.spell1Fallback")}</span>
                </div>
                {renderSwapButton()}
                <div className="spell-item">
                  {!readOnly && <span className="spell-key-inline">F</span>}
                  <span className="spell-name">{t("spells.spell2Fallback")}</span>
                </div>
              </>
            )}
          </div>
        </div>

        {spellChoices && spellChoices.length > 0 && (
          <div
            className="secondary-spell-selector"
            data-testid="secondary-spell-selector"
            aria-label={readOnly ? t("spells.secondaryReadOnlyTitle") : t("spells.secondaryEditTitle")}
          >
            <span
              className="secondary-spell-title"
              title={readOnly ? t("spells.secondaryReadOnlyTitle") : t("spells.secondaryEditTitle")}
            >
              {readOnly ? t("spells.secondaryShortReadOnly") : t("spells.secondaryShortEdit")}
            </span>

            <div className="secondary-spell-options">
              {spellChoices.map((choice, idx) => {
                if (readOnly) {
                  const subtitle = idx === 0 ? t("spells.mostUsed") : t("spells.alternative");
                  return (
                    <Tooltip
                      key={`sec-spell-${choice.id}-${idx}`}
                      content={<SpellTooltipContent spell={choice} subtitle={subtitle} />}
                      ariaLabel={choice.name}
                    >
                      <div
                        className="dd-icon-frame secondary-spell-pill"
                        data-testid={`sec-spell-info-${choice.id}`}
                      >
                        {choice.icon_url && (
                          <img
                            src={resolveCachedAssetUrl(choice.icon_url)}
                            alt={choice.name}
                            className="secondary-spell-icon"
                          />
                        )}
                      </div>
                    </Tooltip>
                  );
                }

                const currentSecondaryId = selectedSecondarySpellId ?? spellChoices[0]?.id;
                const isSelected = currentSecondaryId === choice.id;
                const subtitle = idx === 0 ? t("spells.mostUsed") : t("spells.secondMostUsed");

                return (
                  <Tooltip
                    key={`sec-spell-${choice.id}-${idx}`}
                    content={<SpellTooltipContent spell={choice} subtitle={subtitle} />}
                    focusable={false}
                  >
                    <button
                      type="button"
                      className={`dd-icon-frame secondary-spell-btn ${isSelected ? "active" : ""}`}
                      data-testid={`sec-spell-btn-${choice.id}`}
                      aria-label={choice.name}
                      aria-pressed={isSelected}
                      onClick={() => onSelectSecondarySpell?.(choice.id)}
                    >
                      {choice.icon_url && (
                        <img
                          src={resolveCachedAssetUrl(choice.icon_url)}
                          alt={choice.name}
                          className="secondary-spell-icon"
                        />
                      )}
                    </button>
                  </Tooltip>
                );
              })}
            </div>
          </div>
        )}
      </div>
    </section>
  );
}

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

export function SpellsBar({
  orderedSpells,
  spellChoices,
  selectedSecondarySpellId,
  onInvertSpells,
  onSelectSecondarySpell,
  readOnly = false,
}: SpellsBarProps) {
  const { t } = useTranslation();

  return (
    <section className="card spells-card" data-testid="spells-card">
      <div className="spells-card-header">
        <h2>{t("spells.title")}</h2>
        {!readOnly && onInvertSpells && (
          <button
            type="button"
            className="invert-spells-btn"
            data-testid="invert-spells-btn"
            onClick={onInvertSpells}
            title={t("spells.invertTitle")}
          >
            <span className="invert-spells-icon">⇄</span> {t("spells.invertBtn")}
          </button>
        )}
      </div>

      <div className="spells-content-row">
        <div className="spells-list">
          {orderedSpells && orderedSpells.length >= 2 ? (
            orderedSpells.map((spell, index) => (
              <Tooltip
                key={`spell-${index}-${spell.id}`}
                content={<SpellTooltipContent spell={spell} />}
                triggerClassName="ui-tooltip-trigger-flex"
                ariaLabel={spell.name}
              >
                <div className="spell-item">
                  {!readOnly && (
                    <span className="spell-key-indicator">{index === 0 ? "D" : "F"}</span>
                  )}
                  {spell.icon_url && (
                    <div className="dd-icon-frame spell-icon-frame">
                      <img
                        src={resolveCachedAssetUrl(spell.icon_url)}
                        alt={spell.name}
                        className="spell-icon"
                      />
                    </div>
                  )}
                  <div className="spell-info">
                    <span className="spell-name">{spell.name}</span>
                  </div>
                </div>
              </Tooltip>
            ))
          ) : (
            <div className="spells-fallback">
              <div className="spell-item">
                {!readOnly && <span className="spell-key-indicator">D</span>}
                <span className="spell-name">{t("spells.spell1Fallback")}</span>
              </div>
              <div className="spell-item">
                {!readOnly && <span className="spell-key-indicator">F</span>}
                <span className="spell-name">{t("spells.spell2Fallback")}</span>
              </div>
            </div>
          )}
        </div>

        {spellChoices && spellChoices.length > 0 && (
          <div className="secondary-spell-selector" data-testid="secondary-spell-selector">
            <h4 className="secondary-spell-title">
              {readOnly ? t("spells.secondaryReadOnlyTitle") : t("spells.secondaryEditTitle")}
            </h4>
            <div className="secondary-spell-options">
              {spellChoices.map((choice, idx) => {
                if (readOnly) {
                  const subtitle = idx === 0 ? t("spells.mostUsed") : t("spells.alternative");
                  return (
                    <Tooltip
                      key={`sec-spell-${choice.id}-${idx}`}
                      content={<SpellTooltipContent spell={choice} subtitle={subtitle} />}
                      triggerClassName="ui-tooltip-trigger-flex"
                      ariaLabel={choice.name}
                    >
                      <div
                        className="secondary-spell-pill"
                        data-testid={`sec-spell-info-${choice.id}`}
                      >
                        {choice.icon_url && (
                          <div className="dd-icon-frame secondary-spell-frame">
                            <img
                              src={resolveCachedAssetUrl(choice.icon_url)}
                              alt={choice.name}
                              className="secondary-spell-icon"
                            />
                          </div>
                        )}
                        <span className="secondary-spell-name">{choice.name}</span>
                      </div>
                    </Tooltip>
                  );
                }

                const currentSecondaryId =
                  selectedSecondarySpellId ?? spellChoices[0]?.id;
                const isSelected = currentSecondaryId === choice.id;
                const subtitle = idx === 0 ? t("spells.mostUsed") : t("spells.secondMostUsed");
                return (
                  <Tooltip
                    key={`sec-spell-${choice.id}-${idx}`}
                    content={<SpellTooltipContent spell={choice} subtitle={subtitle} />}
                    triggerClassName="ui-tooltip-trigger-flex"
                    focusable={false}
                  >
                    <button
                      type="button"
                      className={`secondary-spell-btn ${isSelected ? "active" : ""}`}
                      data-testid={`sec-spell-btn-${choice.id}`}
                      onClick={() => onSelectSecondarySpell?.(choice.id)}
                    >
                      {choice.icon_url && (
                        <div className="dd-icon-frame secondary-spell-frame">
                          <img
                            src={resolveCachedAssetUrl(choice.icon_url)}
                            alt={choice.name}
                            className="secondary-spell-icon"
                          />
                        </div>
                      )}
                      <span className="secondary-spell-name">{choice.name}</span>
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

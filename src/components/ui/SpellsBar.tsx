import { FlashKeyPreference, SpellMetadata, SpellPairOption } from "../../types";
import { useTranslation } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { orderSpellPair } from "../../utils/spells";
import { SpellPairTooltipContent } from "./BuildTooltips";
import { Tooltip } from "./Tooltip";

interface SpellsBarProps {
  orderedSpells: SpellMetadata[];
  spellOptions?: SpellPairOption[];
  selectedSpellPairIndex?: number;
  flashKey?: FlashKeyPreference;
  onInvertSpells?: () => void;
  onSelectSpellPair?: (pairIndex: number) => void;
  readOnly?: boolean;
}

/**
 * Compact Summoner Spells bar displaying up to 2 spell pairs with unified layout.
 * Interactive `<button>` controls are only rendered when actions can actually execute.
 */
export function SpellsBar({
  orderedSpells,
  spellOptions,
  selectedSpellPairIndex = 0,
  flashKey = "D",
  onInvertSpells,
  onSelectSpellPair,
  readOnly = false,
}: SpellsBarProps) {
  const { t } = useTranslation();

  const canSelectPair = !readOnly && Boolean(onSelectSpellPair);
  const canInvertSpells = !readOnly && Boolean(onInvertSpells);

  const validOptions = (spellOptions ?? [])
    .filter((opt) => opt.spells && opt.spells.length >= 2)
    .slice(0, 2);

  const displayOptions: SpellPairOption[] =
    validOptions.length > 0
      ? validOptions
      : orderedSpells && orderedSpells.length >= 2
        ? [
            {
              spell1_id: orderedSpells[0].id,
              spell2_id: orderedSpells[1].id,
              play: 0,
              win: 0,
              pick_rate: null,
              spells: orderedSpells.slice(0, 2),
            },
          ]
        : [];

  const activeIndex =
    canSelectPair && selectedSpellPairIndex >= 0 && selectedSpellPairIndex < displayOptions.length
      ? selectedSpellPairIndex
      : 0;

  const renderSwapControl = () => {
    if (canInvertSpells) {
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
    }

    return (
      <Tooltip
        triggerClassName="invert-spells-tooltip-trigger"
        ariaLabel={t("spells.invertReadOnlyHint")}
        content={
          <>
            <div className="rich-tooltip-header">
              <span className="rich-tooltip-title">{t("spells.invertBtn")}</span>
            </div>
            <p className="rich-tooltip-desc">{t("spells.invertReadOnlyHint")}</p>
          </>
        }
      >
        <span
          className="invert-spells-btn is-readonly"
          data-testid="invert-spells-readonly"
          aria-disabled="true"
        >
          <span className="invert-spells-icon" aria-hidden="true">
            ⇄
          </span>
        </span>
      </Tooltip>
    );
  };

  return (
    <section className="card spells-card" data-testid="spells-card">
      <div className="spells-bar-row">
        <span className="spells-inline-title">{t("spells.shortTitle")}</span>

        {displayOptions.length > 0 ? (
          <div
            className={`spell-pair-grid ${displayOptions.length === 1 ? "is-single" : ""}`}
            data-testid="spell-pair-selector"
            aria-label={readOnly ? t("spells.optionsReadOnlyTitle") : t("spells.optionsEditTitle")}
          >
            {displayOptions.map((option, idx) => {
              const isSelected = activeIndex === idx;
              const displayPair =
                isSelected && orderedSpells && orderedSpells.length >= 2
                  ? orderedSpells.slice(0, 2)
                  : orderSpellPair(option.spells, flashKey, false);
              const pairLabel = `${displayPair[0].name} + ${displayPair[1].name}`;
              const pickLabel =
                typeof option.pick_rate === "number" && option.pick_rate > 0
                  ? `${option.pick_rate.toFixed(1)}%`
                  : null;
              const subtitle =
                idx === 0
                  ? t("spells.mostUsedPair")
                  : readOnly
                    ? t("spells.alternativePair")
                    : t("spells.secondMostUsedPair");

              const cardBody = (
                <>
                  <span className="spell-pair-items">
                    {displayPair.slice(0, 2).map((spell, sIdx) => (
                      <span key={`spell-${idx}-${sIdx}-${spell.id}`} className="spell-card-item">
                        {spell.icon_url ? (
                          <span className="spell-icon-anchor">
                            <span className="dd-icon-frame spell-card-icon-frame">
                              <img
                                src={resolveCachedAssetUrl(spell.icon_url)}
                                alt={spell.name}
                                className="spell-icon"
                                decoding="async"
                              />
                            </span>
                            {isSelected && (
                              <span className="spell-key-indicator">{sIdx === 0 ? "D" : "F"}</span>
                            )}
                          </span>
                        ) : (
                          isSelected && (
                            <span className="spell-key-inline">{sIdx === 0 ? "D" : "F"}</span>
                          )
                        )}
                        <span className="spell-name">{spell.name}</span>
                      </span>
                    ))}
                  </span>
                  {pickLabel && <span className="spell-pair-stat">{pickLabel}</span>}
                </>
              );

              const tooltipContent = (
                <SpellPairTooltipContent
                  spells={displayPair}
                  subtitle={subtitle}
                  play={option.play}
                  pickRate={option.pick_rate ?? undefined}
                />
              );

              if (canSelectPair) {
                return (
                  <Tooltip
                    key={`spell-pair-${idx}-${option.spell1_id}-${option.spell2_id}`}
                    triggerClassName="ui-tooltip-trigger-block"
                    focusable={false}
                    content={tooltipContent}
                  >
                    <button
                      type="button"
                      className={`spell-pair-card ${isSelected ? "active" : ""}`}
                      data-testid={`spell-pair-btn-${idx}`}
                      aria-label={pairLabel}
                      aria-pressed={isSelected}
                      onClick={() => onSelectSpellPair?.(idx)}
                    >
                      {cardBody}
                    </button>
                  </Tooltip>
                );
              }

              return (
                <Tooltip
                  key={`spell-pair-${idx}-${option.spell1_id}-${option.spell2_id}`}
                  triggerClassName="ui-tooltip-trigger-block"
                  focusable={true}
                  ariaLabel={pairLabel}
                  content={tooltipContent}
                >
                  <div
                    className={`spell-pair-card is-readonly ${isSelected ? "active" : ""}`}
                    data-testid={`spell-pair-info-${idx}`}
                  >
                    {cardBody}
                  </div>
                </Tooltip>
              );
            })}
          </div>
        ) : (
          <div className="spell-pair-fallback">
            <span className="spell-card-item">
              <span className="spell-key-inline">D</span>
              <span className="spell-name">{t("spells.spell1Fallback")}</span>
            </span>
            <span className="spell-card-item">
              <span className="spell-key-inline">F</span>
              <span className="spell-name">{t("spells.spell2Fallback")}</span>
            </span>
          </div>
        )}

        {renderSwapControl()}
      </div>
    </section>
  );
}

import { ItemMetadata, RuneMetadata, SpellMetadata } from "../../types";
import { useTranslation } from "../../i18n";

export type ItemMetadataMap = Partial<Record<number, ItemMetadata>>;

export function lookupItemMetadata(
  itemMetadata: ItemMetadataMap | undefined | null,
  itemId: number
): ItemMetadata | undefined {
  if (!itemMetadata || itemId <= 0) return undefined;
  return itemMetadata[itemId];
}

interface ItemTooltipContentProps {
  itemId: number;
  metadata?: ItemMetadata;
}

/**
 * Pure visual rich tooltip body for a League item (name, gold cost, stats, clean summary).
 */
export function ItemTooltipContent({ itemId, metadata }: ItemTooltipContentProps) {
  const { t, formatNumber } = useTranslation();
  const title = metadata?.name || t("common.itemAlt", { id: itemId });
  const gold = metadata?.gold_total ?? 0;
  const stats = metadata?.stats ?? [];
  const summary = metadata?.plaintext?.trim() ?? "";

  return (
    <>
      <div className="rich-tooltip-header">
        <span className="rich-tooltip-title">{title}</span>
        {gold > 0 && (
          <span className="rich-tooltip-gold">
            {t("items.goldCost", { gold: formatNumber(gold) })}
          </span>
        )}
      </div>
      {stats.length > 0 && (
        <ul className="rich-tooltip-stats">
          {stats.map((stat, idx) => (
            <li key={`${itemId}-stat-${idx}`} className="rich-tooltip-stat-line">
              {stat}
            </li>
          ))}
        </ul>
      )}
      {summary && <p className="rich-tooltip-desc">{summary}</p>}
    </>
  );
}

interface RuneTooltipContentProps {
  rune: RuneMetadata;
}

/**
 * Pure visual rich tooltip body for a rune or stat shard (localized name + sanitized shortDesc).
 */
export function RuneTooltipContent({ rune }: RuneTooltipContentProps) {
  const desc = rune.short_desc?.trim() ?? "";
  return (
    <>
      <div className="rich-tooltip-header">
        <span className="rich-tooltip-title">{rune.name}</span>
      </div>
      {desc && <p className="rich-tooltip-desc">{desc}</p>}
    </>
  );
}

interface SpellTooltipContentProps {
  spell: SpellMetadata;
  subtitle?: string;
}

/**
 * Pure visual rich tooltip body for a summoner spell (name, cooldown, optional tag, description).
 */
export function SpellTooltipContent({ spell, subtitle }: SpellTooltipContentProps) {
  const { t } = useTranslation();
  const cd = typeof spell.cooldown_secs === "number" && spell.cooldown_secs > 0 ? spell.cooldown_secs : null;
  const desc = spell.description?.trim() ?? "";

  return (
    <>
      <div className="rich-tooltip-header">
        <span className="rich-tooltip-title">{spell.name}</span>
        {cd !== null && (
          <span className="rich-tooltip-badge">
            {t("spells.cooldownSeconds", { secs: cd })}
          </span>
        )}
      </div>
      {subtitle && <span className="rich-tooltip-subtitle">{subtitle}</span>}
      {desc && <p className="rich-tooltip-desc">{desc}</p>}
    </>
  );
}

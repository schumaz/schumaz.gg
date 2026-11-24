import {
  ChampionAbilityMetadata,
  ItemMetadata,
  RuneMetadata,
  SpellMetadata,
} from "../../types";
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

interface SpellPairTooltipContentProps {
  spells: SpellMetadata[];
  subtitle?: string;
  play?: number;
  pickRate?: number;
}

/**
 * Pure visual rich tooltip body for a 2-spell summoner combination option.
 */
export function SpellPairTooltipContent({
  spells,
  subtitle,
  play,
  pickRate,
}: SpellPairTooltipContentProps) {
  const { t, formatNumber } = useTranslation();
  const pairTitle =
    spells.length >= 2 ? `${spells[0].name} + ${spells[1].name}` : spells[0]?.name || "";
  const pctBadge =
    typeof pickRate === "number" && pickRate > 0 ? `${pickRate.toFixed(1)}%` : null;
  const metaParts: string[] = [];
  if (subtitle) metaParts.push(subtitle);
  if (typeof play === "number" && play > 0) {
    metaParts.push(
      `${formatNumber(play)} ${t(play === 1 ? "common.gamesSingular" : "common.gamesPlural")}`
    );
  }
  const metaLine = metaParts.join(" · ");

  return (
    <>
      <div className="rich-tooltip-header">
        <span className="rich-tooltip-title">{pairTitle}</span>
        {pctBadge && <span className="rich-tooltip-badge">{pctBadge}</span>}
      </div>
      {metaLine && <span className="rich-tooltip-subtitle">{metaLine}</span>}
      <div className="rich-tooltip-pair-list">
        {spells.slice(0, 2).map((spell, idx) => {
          const cd =
            typeof spell.cooldown_secs === "number" && spell.cooldown_secs > 0
              ? spell.cooldown_secs
              : null;
          const desc = spell.description?.trim() ?? "";
          return (
            <div key={`pair-tip-${spell.id}-${idx}`} className="rich-tooltip-pair-item">
              <div className="rich-tooltip-pair-row">
                <span className="rich-tooltip-pair-name">{spell.name}</span>
                {cd !== null && (
                  <span className="rich-tooltip-pair-cd">
                    {t("spells.cooldownSeconds", { secs: cd })}
                  </span>
                )}
              </div>
              {desc && <p className="rich-tooltip-pair-desc">{desc}</p>}
            </div>
          );
        })}
      </div>
    </>
  );
}

interface SkillTooltipContentProps {
  skillKey: string;
  ability?: ChampionAbilityMetadata;
}

/**
 * Pure visual rich tooltip body for a champion ability (`Q`, `W`, `E`, `R`).
 */
export function SkillTooltipContent({ skillKey, ability }: SkillTooltipContentProps) {
  const { t } = useTranslation();
  const normalizedKey = skillKey.trim().toUpperCase();
  const title = ability?.name?.trim() || t("skills.abilityFallback", { key: normalizedKey });
  const cd = ability?.cooldown_burn?.trim() ?? "";
  const desc = ability?.description?.trim() ?? "";

  return (
    <>
      <div className="rich-tooltip-header">
        <span className="rich-tooltip-skill-title-wrap">
          <span className={`rich-tooltip-skill-key skill-key-${normalizedKey.toLowerCase()}`}>
            {normalizedKey}
          </span>
          <span className="rich-tooltip-title">{title}</span>
        </span>
        {cd && cd !== "0" && (
          <span className="rich-tooltip-badge">
            {t("skills.cooldownLabel", { cd })}
          </span>
        )}
      </div>
      {desc && <p className="rich-tooltip-desc">{desc}</p>}
    </>
  );
}

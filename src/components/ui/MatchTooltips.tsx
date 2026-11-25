import {
  ArenaAugmentMetadata,
  RuneMetadata,
  SpellMetadata,
} from "../../types";
import { useTranslation } from "../../i18n";

export type RuneMetadataMap = Partial<Record<number, RuneMetadata>>;
export type SpellMetadataMap = Partial<Record<number, SpellMetadata>>;
export type AugmentMetadataMap = Partial<Record<number, ArenaAugmentMetadata>>;

export function lookupRuneMetadata(
  runeMetadata: RuneMetadataMap | undefined | null,
  runeId: number
): RuneMetadata | undefined {
  if (!runeMetadata || runeId <= 0) return undefined;
  return runeMetadata[runeId];
}

export function lookupSpellMetadata(
  spellMetadata: SpellMetadataMap | undefined | null,
  spellId: number
): SpellMetadata | undefined {
  if (!spellMetadata || spellId <= 0) return undefined;
  return spellMetadata[spellId];
}

export function lookupAugmentMetadata(
  augmentMetadata: AugmentMetadataMap | undefined | null,
  augmentId: number
): ArenaAugmentMetadata | undefined {
  if (!augmentMetadata || augmentId <= 0) return undefined;
  return augmentMetadata[augmentId];
}

interface AugmentTooltipContentProps {
  augmentId: number;
  metadata?: ArenaAugmentMetadata;
}

function getAugmentRarityMeta(
  rarity: number | undefined,
  t: ReturnType<typeof useTranslation>["t"]
): { className: string; label: string } | null {
  if (rarity === 1) return { className: "rarity-silver", label: t("arena.raritySilver") };
  if (rarity === 4) return { className: "rarity-gold", label: t("arena.rarityGold") };
  if (rarity === 8) return { className: "rarity-prismatic", label: t("arena.rarityPrismatic") };
  return null;
}

/**
 * Pure visual rich tooltip body for an Arena / ARAM Mayhem augment (name, rarity badge, description).
 */
export function AugmentTooltipContent({ augmentId, metadata }: AugmentTooltipContentProps) {
  const { t } = useTranslation();
  const title = metadata?.name?.trim() || t("profile.augmentFallback", { id: augmentId });
  const rarityMeta = getAugmentRarityMeta(metadata?.rarity, t);
  const desc = metadata?.desc?.trim() ?? "";

  return (
    <>
      <div className="rich-tooltip-header">
        <span className="rich-tooltip-title">{title}</span>
        {rarityMeta && (
          <span className={`rich-tooltip-rarity ${rarityMeta.className}`}>
            {rarityMeta.label}
          </span>
        )}
      </div>
      {desc && <p className="rich-tooltip-desc">{desc}</p>}
    </>
  );
}

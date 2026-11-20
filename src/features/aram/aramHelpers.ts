import { TranslationKey, TranslationParams, translate } from "../../i18n";
import { AramAugmentEntry } from "../../types";

export type AugmentRarityFilter = "all" | 1 | 4 | 8;

type TranslatorFn = (key: TranslationKey, params?: TranslationParams) => string;
const defaultTranslate: TranslatorFn = (key, params) => translate("pt_BR", key, params);

export const LEAGUE_ARAM_ICON_URL =
  "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/content/src/leagueclient/gamemodeassets/aram/img/game-select-icon-active.png";

export const LEAGUE_ARAM_MAYHEM_ICON_URL =
  "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/kiwi/augments/icons/poroking_large.png";

export function normalizeAramSearch(value: string): string {
  return value
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/\s+/g, "");
}

export function getChampionTierLabel(tier: number): string {
  if (tier <= 0) return "OP";
  return `Tier ${tier}`;
}

export function getChampionTierClass(tier: number): string {
  if (tier <= 0) return "tier-op";
  if (tier === 1) return "tier-1";
  if (tier === 2) return "tier-2";
  if (tier === 3) return "tier-3";
  if (tier === 4) return "tier-4";
  return "tier-5";
}

export function getAugmentTierLabel(tier: number): string {
  switch (tier) {
    case 0:
      return "S";
    case 1:
      return "A";
    case 2:
      return "B";
    case 3:
      return "C";
    case 4:
      return "D";
    default:
      return "E";
  }
}

export function getAugmentTierClass(tier: number): string {
  switch (tier) {
    case 0:
      return "aug-tier-s";
    case 1:
      return "aug-tier-a";
    case 2:
      return "aug-tier-b";
    case 3:
      return "aug-tier-c";
    default:
      return "aug-tier-low";
  }
}

export function getAugmentRarityInfo(
  rarity: number | null | undefined,
  t: TranslatorFn = defaultTranslate
): {
  label: string;
  className: string;
} {
  switch (rarity) {
    case 1:
      return { label: t("aram.raritySilver"), className: "rarity-silver" };
    case 4:
      return { label: t("aram.rarityGold"), className: "rarity-gold" };
    case 8:
      return { label: t("aram.rarityPrismatic"), className: "rarity-prismatic" };
    default:
      return { label: t("aram.raritySpecial"), className: "rarity-neutral" };
  }
}

export function sortAugmentsBestToWorst(augments: AramAugmentEntry[]): AramAugmentEntry[] {
  return [...augments].sort((a, b) => {
    if (a.tier !== b.tier) return a.tier - b.tier;
    const popDiff = (b.popular ?? 0) - (a.popular ?? 0);
    if (Math.abs(popDiff) > 0.0001) return popDiff;
    const perfDiff = (b.performance ?? 0) - (a.performance ?? 0);
    if (Math.abs(perfDiff) > 0.0001) return perfDiff;
    return a.name.localeCompare(b.name);
  });
}

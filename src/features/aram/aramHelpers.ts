import { TranslationKey, TranslationParams, translate } from "../../i18n";
import {
  AramAugmentEntry,
  AramChampionRank,
  AramMode,
  ChampionBuild,
  FlashKeyPreference,
  SpellMetadata,
} from "../../types";

export { getChampionTierClass, getChampionTierLabel } from "../../components/ui/championTier";

export type AugmentRarityFilter = "all" | 1 | 4 | 8;

export type AramMayhemPanel = "augments" | "build";

export interface AramClassicBuildFilters {
  tier: string | null;
  region: string;
  patch: string | null;
}

export const DEFAULT_ARAM_CLASSIC_FILTERS: AramClassicBuildFilters = {
  tier: null,
  region: "global",
  patch: null,
};

export interface AramTabProps {
  isActive: boolean;
  savedMode: AramMode | null;
  showChooser: boolean;
  tierLists: Partial<Record<AramMode, AramChampionRank[]>>;
  tierLoading: Partial<Record<AramMode, boolean>>;
  tierErrors: Partial<Record<AramMode, string | null>>;
  selectedChampionKey: string | null;
  augmentsByChampion: Record<string, AramAugmentEntry[]>;
  augmentsLoading: boolean;
  augmentsError: string | null;
  onSelectMode: (mode: AramMode) => void;
  onSelectChampion: (championKey: string) => void;
  onRetryTierList: (mode: AramMode) => void;
  onRetryAugments: () => void;
  selectedClassicChampion?: AramChampionRank | null;
  classicBuild?: ChampionBuild | null;
  classicBuildLoading?: boolean;
  classicBuildError?: string | null;
  classicOrderedSpells?: SpellMetadata[];
  classicRunePageIndex?: number;
  classicBuildIndex?: number;
  onSelectClassicChampion?: (champion: AramChampionRank) => void;
  onBackToClassicList?: () => void;
  onClassicDetailFilterChange?: (
    filterType: "position" | "tier" | "region" | "patch",
    value: string
  ) => void;
  onRetryClassicBuild?: () => void;
  onSelectClassicRunePage?: (pageIndex: number) => void;
  onSelectClassicBuild?: (index: number) => void;
  mayhemPanel?: AramMayhemPanel;
  onSelectMayhemPanel?: (panel: AramMayhemPanel) => void;
  mayhemBuildsByChampion?: Record<string, ChampionBuild>;
  mayhemBuildLoading?: boolean;
  mayhemBuildError?: string | null;
  mayhemOrderedSpells?: SpellMetadata[];
  mayhemBuildIndex?: number;
  onSelectMayhemBuild?: (index: number) => void;
  onRetryMayhemBuild?: () => void;
  flashKey?: FlashKeyPreference;
}

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

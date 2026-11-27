import { TranslationKey, TranslationParams, translate } from "../../i18n";
import { RankedChampionEntry, RankedLane } from "../../types";
import { FilterDropdownOption } from "../../components/ui/FilterDropdown";

type TranslatorFn = (key: TranslationKey, params?: TranslationParams) => string;
const defaultTranslate: TranslatorFn = (key, params) => translate("pt_BR", key, params);

const CDRAGON_POSITION_SVG_BASE =
  "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/svg";

export const LANE_ORDER: RankedLane[] = ["top", "jungle", "mid", "adc", "support"];

export interface LaneMetaItem {
  label: string;
  shortLabel: string;
  iconUrl: string;
}

export function getLaneMeta(
  t: TranslatorFn = defaultTranslate
): Record<RankedLane, LaneMetaItem> {
  return {
    top: {
      label: t("filters.lanes.top"),
      shortLabel: t("filters.lanes.topShort"),
      iconUrl: `${CDRAGON_POSITION_SVG_BASE}/position-top-light.svg`,
    },
    jungle: {
      label: t("filters.lanes.jungle"),
      shortLabel: t("filters.lanes.jungleShort"),
      iconUrl: `${CDRAGON_POSITION_SVG_BASE}/position-jungle-light.svg`,
    },
    mid: {
      label: t("filters.lanes.mid"),
      shortLabel: t("filters.lanes.midShort"),
      iconUrl: `${CDRAGON_POSITION_SVG_BASE}/position-middle-light.svg`,
    },
    adc: {
      label: t("filters.lanes.adc"),
      shortLabel: t("filters.lanes.adcShort"),
      iconUrl: `${CDRAGON_POSITION_SVG_BASE}/position-bottom-light.svg`,
    },
    support: {
      label: t("filters.lanes.support"),
      shortLabel: t("filters.lanes.supportShort"),
      iconUrl: `${CDRAGON_POSITION_SVG_BASE}/position-utility-light.svg`,
    },
  };
}

export const LANE_META = getLaneMeta();

export function getChampionTierFilterOptions(
  t: TranslatorFn = defaultTranslate
): FilterDropdownOption[] {
  return [
    {
      value: "all",
      label: t("ranked.allTiers"),
      icon: <span className="tier-filter-dot tier-all" />,
    },
    {
      value: "0",
      label: "OP",
      icon: <span className="tier-filter-dot tier-op" />,
    },
    {
      value: "1",
      label: "Tier 1",
      icon: <span className="tier-filter-dot tier-1" />,
    },
    {
      value: "2",
      label: "Tier 2",
      icon: <span className="tier-filter-dot tier-2" />,
    },
    {
      value: "3",
      label: "Tier 3",
      icon: <span className="tier-filter-dot tier-3" />,
    },
    {
      value: "4",
      label: "Tier 4",
      icon: <span className="tier-filter-dot tier-4" />,
    },
  ];
}

export const CHAMPION_TIER_FILTER_OPTIONS = getChampionTierFilterOptions();

export function normalizeRankedSearch(value: string): string {
  return value
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/\s+/g, "");
}

function compareAllLanesEntries(a: RankedChampionEntry, b: RankedChampionEntry): number {
  const aOverall = a.overall_rank ?? 0;
  const bOverall = b.overall_rank ?? 0;
  const aHasOverall = aOverall > 0;
  const bHasOverall = bOverall > 0;
  if (aHasOverall && bHasOverall && aOverall !== bOverall) return aOverall - bOverall;
  if (aHasOverall !== bHasOverall) return aHasOverall ? -1 : 1;

  const aWr = a.win_rate ?? -1;
  const bWr = b.win_rate ?? -1;
  if (aWr !== bWr) return bWr - aWr;

  const aPr = a.pick_rate ?? -1;
  const bPr = b.pick_rate ?? -1;
  if (aPr !== bPr) return bPr - aPr;

  const aValid = a.rank > 0 ? 0 : 1;
  const bValid = b.rank > 0 ? 0 : 1;
  if (aValid !== bValid) return aValid - bValid;
  if (a.rank !== b.rank) return a.rank - b.rank;

  return a.name.localeCompare(b.name);
}

function compareSingleLaneEntries(a: RankedChampionEntry, b: RankedChampionEntry): number {
  const aValid = a.rank > 0 ? 0 : 1;
  const bValid = b.rank > 0 ? 0 : 1;
  if (aValid !== bValid) return aValid - bValid;
  if (a.rank !== b.rank) return a.rank - b.rank;
  return compareAllLanesEntries(a, b);
}

export interface TierGroup {
  tier: number;
  items: RankedChampionEntry[];
}

export function groupByTier(
  entries: RankedChampionEntry[],
  mode: "all" | "lane"
): TierGroup[] {
  const tierMap = new Map<number, RankedChampionEntry[]>();
  for (const item of entries) {
    const normalizedTier = Math.max(0, Math.min(4, item.tier));
    const list = tierMap.get(normalizedTier);
    if (list) {
      list.push(item);
    } else {
      tierMap.set(normalizedTier, [item]);
    }
  }

  return Array.from(tierMap.entries())
    .sort((a, b) => a[0] - b[0])
    .map(([tier, items]) => ({
      tier,
      items: [...items].sort(mode === "all" ? compareAllLanesEntries : compareSingleLaneEntries),
    }));
}

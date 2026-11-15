import { TranslationKey, TranslationParams, translate } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { FilterDropdownOption } from "../../components/ui/FilterDropdown";
import { RegionFlag } from "../../components/ui/RegionFlag";

export type TranslatorFn = (key: TranslationKey, params?: TranslationParams) => string;

export const defaultTranslate: TranslatorFn = (key, params) =>
  translate("pt_BR", key, params);

const CDRAGON_MINI_CRESTS_BASE =
  "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/images/ranked-mini-crests";
const CDRAGON_POSITION_SVG_BASE =
  "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/svg";

export function buildIconImg(src: string, className = "filter-option-icon") {
  return (
    <img
      src={resolveCachedAssetUrl(src)}
      alt=""
      aria-hidden="true"
      className={className}
      onError={(e) => {
        (e.currentTarget as HTMLImageElement).style.display = "none";
      }}
    />
  );
}

export function getTierOptions(t: TranslatorFn = defaultTranslate): FilterDropdownOption[] {
  return [
    {
      value: "all",
      label: t("filters.tiers.all"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/unranked.svg`),
    },
    {
      value: "challenger",
      label: t("filters.tiers.challenger"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/challenger.svg`),
    },
    {
      value: "grandmaster",
      label: t("filters.tiers.grandmaster"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/grandmaster.svg`),
    },
    {
      value: "master_plus",
      label: t("filters.tiers.master_plus"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/master.svg`),
    },
    {
      value: "diamond_plus",
      label: t("filters.tiers.diamond_plus"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/diamond.svg`),
    },
    {
      value: "emerald_plus",
      label: t("filters.tiers.emerald_plus"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/emerald.svg`),
    },
    {
      value: "platinum_plus",
      label: t("filters.tiers.platinum_plus"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/platinum.svg`),
    },
    {
      value: "gold_plus",
      label: t("filters.tiers.gold_plus"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/gold.svg`),
    },
    {
      value: "silver",
      label: t("filters.tiers.silver"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/silver.svg`),
    },
    {
      value: "bronze",
      label: t("filters.tiers.bronze"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/bronze.svg`),
    },
    {
      value: "iron",
      label: t("filters.tiers.iron"),
      icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/iron.svg`),
    },
  ];
}

export function getPositionOptions(t: TranslatorFn = defaultTranslate): FilterDropdownOption[] {
  return [
    {
      value: "top",
      label: t("filters.lanes.top"),
      icon: buildIconImg(`${CDRAGON_POSITION_SVG_BASE}/position-top-light.svg`),
    },
    {
      value: "jungle",
      label: t("filters.lanes.jungle"),
      icon: buildIconImg(`${CDRAGON_POSITION_SVG_BASE}/position-jungle-light.svg`),
    },
    {
      value: "mid",
      label: t("filters.lanes.mid"),
      icon: buildIconImg(`${CDRAGON_POSITION_SVG_BASE}/position-middle-light.svg`),
    },
    {
      value: "adc",
      label: t("filters.lanes.adc"),
      icon: buildIconImg(`${CDRAGON_POSITION_SVG_BASE}/position-bottom-light.svg`),
    },
    {
      value: "support",
      label: t("filters.lanes.support"),
      icon: buildIconImg(`${CDRAGON_POSITION_SVG_BASE}/position-utility-light.svg`),
    },
  ];
}

export function getRegionOptions(t: TranslatorFn = defaultTranslate): FilterDropdownOption[] {
  const regions = [
    ["br", "BR", "filters.regions.brTitle"],
    ["kr", "KR", "filters.regions.krTitle"],
    ["na", "NA", "filters.regions.naTitle"],
    ["euw", "EUW", "filters.regions.euwTitle"],
    ["eune", "EUNE", "filters.regions.euneTitle"],
    ["lan", "LAN", "filters.regions.lanTitle"],
    ["las", "LAS", "filters.regions.lasTitle"],
    ["oce", "OCE", "filters.regions.oceTitle"],
    ["tr", "TR", "filters.regions.trTitle"],
    ["ru", "RU", "filters.regions.ruTitle"],
    ["jp", "JP", "filters.regions.jpTitle"],
    ["me", "ME", "filters.regions.meTitle"],
    ["sg", "SG", "filters.regions.sgTitle"],
    ["tw", "TW", "filters.regions.twTitle"],
    ["vn", "VN", "filters.regions.vnTitle"],
  ] as const;

  return [
    {
      value: "global",
      label: t("filters.regions.globalLabel"),
      title: t("filters.regions.globalTitle"),
      icon: <RegionFlag region="global" />,
    },
    ...regions.map(([value, label, titleKey]) => ({
      value,
      label,
      title: t(titleKey),
      icon: <RegionFlag region={value} />,
    })),
  ];
}

export const TIER_OPTIONS: FilterDropdownOption[] = getTierOptions();
export const POSITION_OPTIONS: FilterDropdownOption[] = getPositionOptions();
export const REGION_OPTIONS: FilterDropdownOption[] = getRegionOptions();

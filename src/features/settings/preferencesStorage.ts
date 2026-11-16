import {
  AppSettings,
  NavigationState,
  NavigationStatePatch,
} from "../../types";

export const DEFAULT_PROFILE_BLUR = 10;
export const MAX_PROFILE_BLUR_PX = 32;

export const DEFAULT_NAVIGATION_STATE: NavigationState = {
  savedAramMode: null,
  profileMatchModeFilter: "all",
  aramMayhemRarityFilter: "all",
  aramMayhemLastChampion: null,
  rankedFilterRegion: "global",
  rankedFilterTier: "emerald_plus",
  rankedFilterLane: "all",
  rankedFilterPatch: null,
  rankedChampionTierFilter: "all",
  lastSummonerId: null,
};

export function isValidAppSettings(value: unknown): value is AppSettings {
  if (!value || typeof value !== "object") {
    return false;
  }
  const candidate = value as Partial<AppSettings>;
  return Boolean(
    candidate.general &&
      typeof candidate.general === "object" &&
      candidate.automation &&
      typeof candidate.automation === "object" &&
      candidate.profileAppearance &&
      typeof candidate.profileAppearance === "object" &&
      candidate.graphics &&
      typeof candidate.graphics === "object" &&
      candidate.navigation &&
      typeof candidate.navigation === "object"
  );
}

export function clampProfileBlur(val: number): number {
  if (!Number.isFinite(val)) return DEFAULT_PROFILE_BLUR;
  return Math.min(100, Math.max(0, Math.round(val)));
}

export function profileBlurPercentToPx(percent: number): number {
  const clamped = clampProfileBlur(percent);
  return Math.round((clamped / 100) * MAX_PROFILE_BLUR_PX * 10) / 10;
}

export function mergeNavigationState(
  prev: NavigationState | null,
  patch: NavigationStatePatch
): NavigationState {
  const base: NavigationState = prev ?? DEFAULT_NAVIGATION_STATE;

  return {
    savedAramMode:
      patch.savedAramMode !== undefined ? patch.savedAramMode : base.savedAramMode ?? null,
    profileMatchModeFilter:
      patch.profileMatchModeFilter !== undefined && patch.profileMatchModeFilter !== null
        ? patch.profileMatchModeFilter
        : base.profileMatchModeFilter,
    aramMayhemRarityFilter:
      patch.aramMayhemRarityFilter !== undefined && patch.aramMayhemRarityFilter !== null
        ? patch.aramMayhemRarityFilter
        : base.aramMayhemRarityFilter,
    aramMayhemLastChampion:
      patch.aramMayhemLastChampion !== undefined
        ? patch.aramMayhemLastChampion === ""
          ? null
          : patch.aramMayhemLastChampion
        : base.aramMayhemLastChampion ?? null,
    rankedFilterRegion:
      patch.rankedFilterRegion !== undefined && patch.rankedFilterRegion !== null
        ? patch.rankedFilterRegion
        : base.rankedFilterRegion,
    rankedFilterTier:
      patch.rankedFilterTier !== undefined && patch.rankedFilterTier !== null
        ? patch.rankedFilterTier
        : base.rankedFilterTier,
    rankedFilterLane:
      patch.rankedFilterLane !== undefined && patch.rankedFilterLane !== null
        ? patch.rankedFilterLane
        : base.rankedFilterLane,
    rankedFilterPatch:
      patch.rankedFilterPatch !== undefined
        ? patch.rankedFilterPatch === ""
          ? null
          : patch.rankedFilterPatch
        : base.rankedFilterPatch ?? null,
    rankedChampionTierFilter:
      patch.rankedChampionTierFilter !== undefined && patch.rankedChampionTierFilter !== null
        ? patch.rankedChampionTierFilter
        : base.rankedChampionTierFilter,
    lastSummonerId:
      patch.lastSummonerId !== undefined ? patch.lastSummonerId : base.lastSummonerId ?? null,
  };
}

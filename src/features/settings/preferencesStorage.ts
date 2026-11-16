import {
  ARAM_SUBMODE_STORAGE_KEY,
  AppLocale,
  AppSettings,
  DEFAULT_PREFERENCES,
  FLASH_ONBOARDING_STORAGE_KEY,
  LANGUAGE_STORAGE_KEY,
  NavigationState,
  NavigationStatePatch,
  PREFERENCES_STORAGE_KEY,
  RANKED_FILTERS_STORAGE_KEY,
  UserPreferences,
} from "../../types";
import { DEFAULT_ONBOARDING_LOCALE, isValidAppLocale } from "../../i18n";

export const PROFILE_BLUR_STORAGE_KEY = "schumaz_profile_ui_blur";
export const WALLPAPER_VISIBLE_STORAGE_KEY = "schumaz_profile_wallpaper_visible";
export const DEFAULT_PROFILE_BLUR = 10;
export const MAX_PROFILE_BLUR_PX = 32;

const LEGACY_LOCAL_STORAGE_KEYS = [
  PREFERENCES_STORAGE_KEY,
  LANGUAGE_STORAGE_KEY,
  PROFILE_BLUR_STORAGE_KEY,
  WALLPAPER_VISIBLE_STORAGE_KEY,
  ARAM_SUBMODE_STORAGE_KEY,
  RANKED_FILTERS_STORAGE_KEY,
] as const;

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

export function readLegacyStorageKey(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

export function buildLegacyMigrationPayload() {
  return {
    schumazUserPreferences: readLegacyStorageKey(PREFERENCES_STORAGE_KEY),
    schumazLanguage: readLegacyStorageKey(LANGUAGE_STORAGE_KEY),
    schumazProfileUiBlur: readLegacyStorageKey(PROFILE_BLUR_STORAGE_KEY),
    schumazProfileWallpaperVisible: readLegacyStorageKey(WALLPAPER_VISIBLE_STORAGE_KEY),
    schumazAramSubmode: readLegacyStorageKey(ARAM_SUBMODE_STORAGE_KEY),
    schumazRankedFilters: readLegacyStorageKey(RANKED_FILTERS_STORAGE_KEY),
  };
}

export function clearLegacyLocalStorageKeys(): void {
  try {
    for (const key of LEGACY_LOCAL_STORAGE_KEYS) {
      localStorage.removeItem(key);
    }
    localStorage.removeItem(FLASH_ONBOARDING_STORAGE_KEY);
  } catch {
    // Ignore storage access errors
  }
}

export interface InitialLanguageState {
  locale: AppLocale;
  needsOnboarding: boolean;
}

export function loadInitialLanguageState(): InitialLanguageState {
  try {
    const raw = localStorage.getItem(LANGUAGE_STORAGE_KEY);
    if (isValidAppLocale(raw)) {
      return { locale: raw, needsOnboarding: false };
    }
  } catch {
    // Ignore storage access errors
  }
  return { locale: DEFAULT_ONBOARDING_LOCALE, needsOnboarding: true };
}

export function saveLanguageToStorage(locale: AppLocale): void {
  try {
    localStorage.setItem(LANGUAGE_STORAGE_KEY, locale);
  } catch {
    // Ignore localStorage quota errors
  }
}

export function loadInitialFlashOnboardingNeeded(): boolean {
  try {
    return localStorage.getItem(FLASH_ONBOARDING_STORAGE_KEY) !== "true";
  } catch {
    return true;
  }
}

export function saveFlashOnboardingCompletedToStorage(): void {
  try {
    localStorage.setItem(FLASH_ONBOARDING_STORAGE_KEY, "true");
  } catch {
    // Ignore localStorage quota errors
  }
}

export function clampProfileBlur(val: number): number {
  if (!Number.isFinite(val)) return DEFAULT_PROFILE_BLUR;
  return Math.min(100, Math.max(0, Math.round(val)));
}

export function profileBlurPercentToPx(percent: number): number {
  const clamped = clampProfileBlur(percent);
  return Math.round((clamped / 100) * MAX_PROFILE_BLUR_PX * 10) / 10;
}

export function loadInitialProfileBlur(): number {
  try {
    const raw = localStorage.getItem(PROFILE_BLUR_STORAGE_KEY);
    if (raw !== null) {
      return clampProfileBlur(Number(raw));
    }
  } catch {
    // Fallback to default blur
  }
  return DEFAULT_PROFILE_BLUR;
}

export function loadInitialWallpaperVisible(): boolean {
  try {
    const raw = localStorage.getItem(WALLPAPER_VISIBLE_STORAGE_KEY);
    if (raw === "false") return false;
  } catch {
    // Ignore storage errors
  }
  return true;
}

export function loadInitialPreferences(resolvedLocale?: AppLocale): UserPreferences {
  const fallbackLanguage = resolvedLocale ?? loadInitialLanguageState().locale;
  try {
    const raw = localStorage.getItem(PREFERENCES_STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return {
        autoInjectRunes:
          typeof parsed.autoInjectRunes === "boolean"
            ? parsed.autoInjectRunes
            : DEFAULT_PREFERENCES.autoInjectRunes,
        autoInjectSpells:
          typeof parsed.autoInjectSpells === "boolean"
            ? parsed.autoInjectSpells
            : DEFAULT_PREFERENCES.autoInjectSpells,
        autoInjectItems:
          typeof parsed.autoInjectItems === "boolean"
            ? parsed.autoInjectItems
            : DEFAULT_PREFERENCES.autoInjectItems,
        flashKey: parsed.flashKey === "F" ? "F" : "D",
        patch48hProtection:
          typeof parsed.patch48hProtection === "boolean"
            ? parsed.patch48hProtection
            : DEFAULT_PREFERENCES.patch48hProtection,
        autoMarkDailyMissionsRead:
          typeof parsed.autoMarkDailyMissionsRead === "boolean"
            ? parsed.autoMarkDailyMissionsRead
            : DEFAULT_PREFERENCES.autoMarkDailyMissionsRead,
        language: resolvedLocale
          ? resolvedLocale
          : isValidAppLocale(parsed.language)
          ? parsed.language
          : fallbackLanguage,
      };
    }
  } catch {
    // Fallback to default preferences if reading localStorage fails
  }
  return { ...DEFAULT_PREFERENCES, language: fallbackLanguage };
}

export function mergeNavigationState(
  prev: NavigationState | null,
  patch: NavigationStatePatch
): NavigationState {
  const base: NavigationState = prev ?? {
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

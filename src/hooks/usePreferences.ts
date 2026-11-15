import { useCallback, useEffect, useRef, useState } from "react";
import {
  ARAM_SUBMODE_STORAGE_KEY,
  AppLocale,
  AppSettings,
  DEFAULT_PREFERENCES,
  FlashKeyPreference,
  LANGUAGE_STORAGE_KEY,
  NavigationState,
  NavigationStatePatch,
  PREFERENCES_STORAGE_KEY,
  RANKED_FILTERS_STORAGE_KEY,
  UserPreferences,
  commands,
} from "../types";
import { DEFAULT_ONBOARDING_LOCALE, isValidAppLocale } from "../i18n";

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

function readLegacyStorageKey(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

export function clearLegacyLocalStorageKeys(): void {
  try {
    for (const key of LEGACY_LOCAL_STORAGE_KEYS) {
      localStorage.removeItem(key);
    }
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

function mergeNavigationState(
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

export function usePreferences(
  onFlashKeyChange?: (newKey: FlashKeyPreference) => void,
  onPatchProtectionChange?: (enabled: boolean) => void,
  onLocaleChange?: (locale: AppLocale) => void
) {
  const [initialLangState] = useState<InitialLanguageState>(loadInitialLanguageState);
  const [locale, setLocale] = useState<AppLocale>(initialLangState.locale);
  const [needsLanguageOnboarding, setNeedsLanguageOnboarding] = useState<boolean>(
    initialLangState.needsOnboarding
  );
  const [preferences, setPreferences] = useState<UserPreferences>(() =>
    loadInitialPreferences(initialLangState.locale)
  );
  const [profileBlur, setProfileBlur] = useState<number>(loadInitialProfileBlur);
  const [wallpaperVisible, setWallpaperVisible] = useState<boolean>(loadInitialWallpaperVisible);
  const [hardwareAcceleration, setHardwareAcceleration] = useState<boolean>(true);
  const [gpuRestartRequired, setGpuRestartRequired] = useState<boolean>(false);
  const [navigationState, setNavigationState] = useState<NavigationState | null>(null);
  const [iniBacked, setIniBacked] = useState<boolean>(false);

  const gpuActiveAtStartupRef = useRef<boolean>(true);
  const usingIniRef = useRef<boolean>(false);
  const profileBlurRef = useRef<number>(profileBlur);
  const wallpaperVisibleRef = useRef<boolean>(wallpaperVisible);

  useEffect(() => {
    document.documentElement.lang = locale === "pt_BR" ? "pt-BR" : "en-US";
  }, [locale]);

  useEffect(() => {
    let active = true;

    const applyIniSettings = (settings: AppSettings) => {
      usingIniRef.current = true;
      setIniBacked(true);
      clearLegacyLocalStorageKeys();

      const resolvedLocale: AppLocale = isValidAppLocale(settings.general.language)
        ? settings.general.language
        : isValidAppLocale(settings.automation.language)
        ? settings.automation.language
        : DEFAULT_ONBOARDING_LOCALE;

      setLocale(resolvedLocale);
      setNeedsLanguageOnboarding(!settings.general.languageOnboardingCompleted);
      setPreferences({
        ...settings.automation,
        language: resolvedLocale,
      });

      const blur = clampProfileBlur(settings.profileAppearance.profileBlurPercent);
      profileBlurRef.current = blur;
      setProfileBlur(blur);

      const wpVisible = Boolean(settings.profileAppearance.wallpaperVisible);
      wallpaperVisibleRef.current = wpVisible;
      setWallpaperVisible(wpVisible);

      setHardwareAcceleration(Boolean(settings.graphics.hardwareAcceleration));
      setNavigationState(settings.navigation);
    };

    const initialize = async () => {
      let hydratedFromIni = false;

      if (typeof commands.migrateLegacyLocalStorage === "function") {
        try {
          const payload = {
            schumazUserPreferences: readLegacyStorageKey(PREFERENCES_STORAGE_KEY),
            schumazLanguage: readLegacyStorageKey(LANGUAGE_STORAGE_KEY),
            schumazProfileUiBlur: readLegacyStorageKey(PROFILE_BLUR_STORAGE_KEY),
            schumazProfileWallpaperVisible: readLegacyStorageKey(WALLPAPER_VISIBLE_STORAGE_KEY),
            schumazAramSubmode: readLegacyStorageKey(ARAM_SUBMODE_STORAGE_KEY),
            schumazRankedFilters: readLegacyStorageKey(RANKED_FILTERS_STORAGE_KEY),
          };
          const migrated = await commands.migrateLegacyLocalStorage(payload);
          if (active && isValidAppSettings(migrated)) {
            applyIniSettings(migrated);
            hydratedFromIni = true;
          }
        } catch {
          // Fallback below
        }
      }

      if (!hydratedFromIni && typeof commands.getAppSettings === "function") {
        try {
          const loaded = await commands.getAppSettings();
          if (active && isValidAppSettings(loaded)) {
            applyIniSettings(loaded);
            hydratedFromIni = true;
          }
        } catch {
          // Fallback to legacy localStorage mode in non-Tauri environments
        }
      }

      if (!hydratedFromIni && active) {
        commands.syncUserPreferences(preferences).catch(() => {});
        if (typeof commands.setLocale === "function") {
          Promise.resolve(commands.setLocale(locale)).catch(() => {});
        }
      }

      if (typeof commands.getHardwareAccelerationStatus === "function") {
        commands
          .getHardwareAccelerationStatus()
          .then((status) => {
            if (active && status && typeof status.enabled === "boolean") {
              gpuActiveAtStartupRef.current = status.active_at_startup;
              setHardwareAcceleration(status.enabled);
              setGpuRestartRequired(status.enabled !== status.active_at_startup);
            }
          })
          .catch(() => {});
      }
    };

    initialize();

    return () => {
      active = false;
    };
  }, []);

  const persistPreferences = useCallback((next: UserPreferences) => {
    if (!usingIniRef.current) {
      try {
        localStorage.setItem(PREFERENCES_STORAGE_KEY, JSON.stringify(next));
      } catch {
        // Ignore localStorage quota errors
      }
    }
    commands.syncUserPreferences(next).catch(() => {});
  }, []);

  const updatePreference = <K extends keyof UserPreferences>(
    key: K,
    value: UserPreferences[K]
  ) => {
    if (key === "language" && isValidAppLocale(value)) {
      changeLanguage(value);
      return;
    }

    setPreferences((prev) => {
      const next = { ...prev, [key]: value };
      if (!usingIniRef.current) {
        try {
          localStorage.setItem(PREFERENCES_STORAGE_KEY, JSON.stringify(next));
        } catch {
          // Ignore localStorage quota errors
        }
      }
      commands
        .syncUserPreferences(next)
        .then(() => {
          if (key === "patch48hProtection" && onPatchProtectionChange) {
            onPatchProtectionChange(Boolean(value));
          }
        })
        .catch(() => {});
      return next;
    });

    if (key === "flashKey" && onFlashKeyChange) {
      onFlashKeyChange(value as FlashKeyPreference);
    }
  };

  const previewOnboardingLocale = useCallback((nextLocale: AppLocale) => {
    setLocale(nextLocale);
    setPreferences((prev) => ({ ...prev, language: nextLocale }));
  }, []);

  const confirmLanguageOnboarding = useCallback(() => {
    setNeedsLanguageOnboarding(false);
    setPreferences((prev) => {
      const next = { ...prev, language: locale };
      persistPreferences(next);
      return next;
    });

    if (usingIniRef.current && typeof commands.completeLanguageOnboarding === "function") {
      Promise.resolve(commands.completeLanguageOnboarding(locale))
        .then(() => {
          onLocaleChange?.(locale);
        })
        .catch(() => {
          onLocaleChange?.(locale);
        });
      return;
    }

    saveLanguageToStorage(locale);
    if (typeof commands.setLocale === "function") {
      Promise.resolve(commands.setLocale(locale))
        .then(() => {
          onLocaleChange?.(locale);
        })
        .catch(() => {
          onLocaleChange?.(locale);
        });
    } else {
      onLocaleChange?.(locale);
    }
  }, [locale, persistPreferences, onLocaleChange]);

  const changeLanguage = useCallback(
    (nextLocale: AppLocale) => {
      setLocale(nextLocale);
      if (!usingIniRef.current) {
        saveLanguageToStorage(nextLocale);
      }
      setNeedsLanguageOnboarding(false);
      setPreferences((prev) => {
        const next = { ...prev, language: nextLocale };
        persistPreferences(next);
        return next;
      });
      if (typeof commands.setLocale === "function") {
        Promise.resolve(commands.setLocale(nextLocale))
          .then(() => {
            onLocaleChange?.(nextLocale);
          })
          .catch(() => {
            onLocaleChange?.(nextLocale);
          });
      } else {
        onLocaleChange?.(nextLocale);
      }
    },
    [persistPreferences, onLocaleChange]
  );

  const updateProfileBlur = useCallback((blurPercent: number) => {
    const clamped = clampProfileBlur(blurPercent);
    profileBlurRef.current = clamped;
    setProfileBlur(clamped);
    if (usingIniRef.current && typeof commands.updateProfileAppearance === "function") {
      commands
        .updateProfileAppearance(clamped, wallpaperVisibleRef.current)
        .catch(() => {});
    } else {
      try {
        localStorage.setItem(PROFILE_BLUR_STORAGE_KEY, String(clamped));
      } catch {
        // Ignore localStorage quota errors
      }
    }
  }, []);

  const updateWallpaperVisible = useCallback((visible: boolean) => {
    wallpaperVisibleRef.current = visible;
    setWallpaperVisible(visible);
    if (usingIniRef.current && typeof commands.updateProfileAppearance === "function") {
      commands
        .updateProfileAppearance(profileBlurRef.current, visible)
        .catch(() => {});
    } else {
      try {
        localStorage.setItem(WALLPAPER_VISIBLE_STORAGE_KEY, String(visible));
      } catch {
        // Ignore localStorage quota errors
      }
    }
  }, []);

  const updateNavigationState = useCallback((patch: NavigationStatePatch) => {
    setNavigationState((prev) => mergeNavigationState(prev, patch));
    if (usingIniRef.current && typeof commands.updateNavigationState === "function") {
      commands
        .updateNavigationState(patch)
        .then((updated) => {
          if (isValidAppSettings(updated)) {
            setNavigationState(updated.navigation);
          }
        })
        .catch(() => {});
    }
  }, []);

  const updateHardwareAcceleration = (enabled: boolean) => {
    setHardwareAcceleration(enabled);
    setGpuRestartRequired(enabled !== gpuActiveAtStartupRef.current);

    if (typeof commands.setHardwareAcceleration === "function") {
      commands
        .setHardwareAcceleration(enabled)
        .then((status) => {
          if (status && typeof status.enabled === "boolean") {
            gpuActiveAtStartupRef.current = status.active_at_startup;
            setHardwareAcceleration(status.enabled);
            setGpuRestartRequired(status.enabled !== status.active_at_startup);
          }
        })
        .catch(() => {});
    }
  };

  const restartApp = () => {
    if (typeof commands.restartApp === "function") {
      commands.restartApp().catch(() => {});
    }
  };

  return {
    locale,
    needsLanguageOnboarding,
    previewOnboardingLocale,
    confirmLanguageOnboarding,
    changeLanguage,
    preferences,
    updatePreference,
    profileBlur,
    updateProfileBlur,
    wallpaperVisible,
    updateWallpaperVisible,
    hardwareAcceleration,
    updateHardwareAcceleration,
    gpuRestartRequired,
    restartApp,
    navigationState,
    updateNavigationState,
    iniBacked,
  };
}

import { useCallback, useEffect, useRef, useState } from "react";
import {
  AppLocale,
  DEFAULT_PREFERENCES,
  FlashKeyPreference,
  LANGUAGE_STORAGE_KEY,
  PREFERENCES_STORAGE_KEY,
  UserPreferences,
  commands,
} from "../types";
import { DEFAULT_ONBOARDING_LOCALE, isValidAppLocale } from "../i18n";

export const PROFILE_BLUR_STORAGE_KEY = "schumaz_profile_ui_blur";
export const WALLPAPER_VISIBLE_STORAGE_KEY = "schumaz_profile_wallpaper_visible";
export const DEFAULT_PROFILE_BLUR = 10;
export const MAX_PROFILE_BLUR_PX = 32;

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
  const gpuActiveAtStartupRef = useRef<boolean>(true);

  useEffect(() => {
    document.documentElement.lang = locale === "pt_BR" ? "pt-BR" : "en-US";
  }, [locale]);

  useEffect(() => {
    // Sync initial preferences with backend
    commands.syncUserPreferences(preferences).catch(() => {});
    if (typeof commands.setLocale === "function") {
      Promise.resolve(commands.setLocale(locale)).catch(() => {});
    }

    if (typeof commands.getHardwareAccelerationStatus === "function") {
      commands
        .getHardwareAccelerationStatus()
        .then((status) => {
          if (status && typeof status.enabled === "boolean") {
            gpuActiveAtStartupRef.current = status.active_at_startup;
            setHardwareAcceleration(status.enabled);
            setGpuRestartRequired(status.enabled !== status.active_at_startup);
          }
        })
        .catch(() => {});
    }
  }, []);

  const persistPreferences = useCallback((next: UserPreferences) => {
    try {
      localStorage.setItem(PREFERENCES_STORAGE_KEY, JSON.stringify(next));
    } catch {
      // Ignore localStorage quota errors
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
      try {
        localStorage.setItem(PREFERENCES_STORAGE_KEY, JSON.stringify(next));
      } catch {
        // Ignore localStorage quota errors
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
    saveLanguageToStorage(locale);
    setNeedsLanguageOnboarding(false);
    setPreferences((prev) => {
      const next = { ...prev, language: locale };
      persistPreferences(next);
      return next;
    });
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
      saveLanguageToStorage(nextLocale);
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

  const updateProfileBlur = (blurPercent: number) => {
    const clamped = clampProfileBlur(blurPercent);
    setProfileBlur(clamped);
    try {
      localStorage.setItem(PROFILE_BLUR_STORAGE_KEY, String(clamped));
    } catch {
      // Ignore localStorage quota errors
    }
  };

  const updateWallpaperVisible = (visible: boolean) => {
    setWallpaperVisible(visible);
    try {
      localStorage.setItem(WALLPAPER_VISIBLE_STORAGE_KEY, String(visible));
    } catch {
      // Ignore localStorage quota errors
    }
  };

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
  };
}

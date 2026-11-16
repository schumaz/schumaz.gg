import { useCallback, useEffect, useRef, useState } from "react";
import {
  AppLocale,
  FlashKeyPreference,
  NavigationState,
  NavigationStatePatch,
  PREFERENCES_STORAGE_KEY,
  UserPreferences,
} from "../../types";
import { settingsIpc, windowIpc } from "../../services/ipc";
import { isValidAppLocale } from "../../i18n";
import {
  InitialLanguageState,
  PROFILE_BLUR_STORAGE_KEY,
  WALLPAPER_VISIBLE_STORAGE_KEY,
  clampProfileBlur,
  isValidAppSettings,
  loadInitialFlashOnboardingNeeded,
  loadInitialLanguageState,
  loadInitialPreferences,
  loadInitialProfileBlur,
  loadInitialWallpaperVisible,
  mergeNavigationState,
  saveFlashOnboardingCompletedToStorage,
  saveLanguageToStorage,
} from "./preferencesStorage";
import { extractHydratedIniSettings, fetchInitialIniSettings } from "./preferencesHydration";

export * from "./preferencesStorage";

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
  const [needsFlashOnboarding, setNeedsFlashOnboarding] = useState<boolean>(
    loadInitialFlashOnboardingNeeded
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
  const [preferencesReady, setPreferencesReady] = useState<boolean>(false);

  const gpuActiveAtStartupRef = useRef<boolean>(true);
  const usingIniRef = useRef<boolean>(false);
  const profileBlurRef = useRef<number>(profileBlur);
  const wallpaperVisibleRef = useRef<boolean>(wallpaperVisible);

  useEffect(() => {
    document.documentElement.lang = locale === "pt_BR" ? "pt-BR" : "en-US";
  }, [locale]);

  useEffect(() => {
    let active = true;
    const initialize = async () => {
      try {
        const settings = await fetchInitialIniSettings();
        if (active && settings) {
          usingIniRef.current = true;
          setIniBacked(true);
          const hydrated = extractHydratedIniSettings(settings);
          setLocale(hydrated.resolvedLocale);
          setNeedsLanguageOnboarding(hydrated.needsOnboarding);
          setNeedsFlashOnboarding(hydrated.needsFlashOnboarding);
          setPreferences(hydrated.preferences);
          profileBlurRef.current = hydrated.blur;
          setProfileBlur(hydrated.blur);
          wallpaperVisibleRef.current = hydrated.wallpaperVisible;
          setWallpaperVisible(hydrated.wallpaperVisible);
          setHardwareAcceleration(hydrated.hardwareAcceleration);
          setNavigationState(hydrated.navigation);
        } else if (active) {
          settingsIpc.syncUserPreferences(preferences).catch(() => {});
          settingsIpc.setLocale(locale).catch(() => {});
        }

        try {
          const status = await settingsIpc.getHardwareAccelerationStatus();
          if (active && status && typeof status.enabled === "boolean") {
            gpuActiveAtStartupRef.current = status.active_at_startup;
            setHardwareAcceleration(status.enabled);
            setGpuRestartRequired(status.enabled !== status.active_at_startup);
          }
        } catch {
          // Ignore in non-Tauri environments
        }
      } finally {
        if (active) {
          setPreferencesReady(true);
        }
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
      } catch {}
    }
    settingsIpc.syncUserPreferences(next).catch(() => {});
  }, []);

  const changeLanguage = useCallback(
    (nextLocale: AppLocale) => {
      setLocale(nextLocale);
      if (!usingIniRef.current) saveLanguageToStorage(nextLocale);
      setNeedsLanguageOnboarding(false);
      setPreferences((prev) => {
        const next = { ...prev, language: nextLocale };
        persistPreferences(next);
        return next;
      });
      settingsIpc
        .setLocale(nextLocale)
        .then(() => onLocaleChange?.(nextLocale))
        .catch(() => onLocaleChange?.(nextLocale));
    },
    [persistPreferences, onLocaleChange]
  );

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
        } catch {}
      }
      settingsIpc
        .syncUserPreferences(next)
        .then(() => {
          if (key === "patch48hProtection") onPatchProtectionChange?.(Boolean(value));
        })
        .catch(() => {});
      return next;
    });
    if (key === "flashKey") onFlashKeyChange?.(value as FlashKeyPreference);
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
    const notify = () => onLocaleChange?.(locale);
    if (usingIniRef.current) {
      settingsIpc.completeLanguageOnboarding(locale).then(notify).catch(notify);
      return;
    }
    saveLanguageToStorage(locale);
    settingsIpc.setLocale(locale).then(notify).catch(notify);
  }, [locale, persistPreferences, onLocaleChange]);

  const previewOnboardingFlashKey = useCallback((nextKey: FlashKeyPreference) => {
    setPreferences((prev) => ({ ...prev, flashKey: nextKey }));
  }, []);

  const confirmFlashOnboarding = useCallback(() => {
    const selectedFlashKey = preferences.flashKey;
    setNeedsFlashOnboarding(false);
    setPreferences((prev) => {
      const next = { ...prev, flashKey: selectedFlashKey };
      if (!usingIniRef.current) {
        try {
          localStorage.setItem(PREFERENCES_STORAGE_KEY, JSON.stringify(next));
        } catch {}
      }
      return next;
    });
    if (usingIniRef.current) {
      settingsIpc.completeFlashOnboarding(selectedFlashKey).catch(() => {});
    } else {
      saveFlashOnboardingCompletedToStorage();
      settingsIpc
        .syncUserPreferences({ ...preferences, flashKey: selectedFlashKey })
        .catch(() => {});
    }
    onFlashKeyChange?.(selectedFlashKey);
  }, [preferences, onFlashKeyChange]);

  const updateProfileBlur = useCallback((blurPercent: number) => {
    const clamped = clampProfileBlur(blurPercent);
    profileBlurRef.current = clamped;
    setProfileBlur(clamped);
    if (usingIniRef.current) {
      settingsIpc.updateProfileAppearance(clamped, wallpaperVisibleRef.current).catch(() => {});
    } else {
      try {
        localStorage.setItem(PROFILE_BLUR_STORAGE_KEY, String(clamped));
      } catch {}
    }
  }, []);

  const updateWallpaperVisible = useCallback((visible: boolean) => {
    wallpaperVisibleRef.current = visible;
    setWallpaperVisible(visible);
    if (usingIniRef.current) {
      settingsIpc.updateProfileAppearance(profileBlurRef.current, visible).catch(() => {});
    } else {
      try {
        localStorage.setItem(WALLPAPER_VISIBLE_STORAGE_KEY, String(visible));
      } catch {}
    }
  }, []);

  const updateNavigationState = useCallback((patch: NavigationStatePatch) => {
    setNavigationState((prev) => mergeNavigationState(prev, patch));
    if (usingIniRef.current) {
      settingsIpc
        .updateNavigationState(patch)
        .then((updated) => {
          if (isValidAppSettings(updated)) setNavigationState(updated.navigation);
        })
        .catch(() => {});
    }
  }, []);

  const updateHardwareAcceleration = (enabled: boolean) => {
    setHardwareAcceleration(enabled);
    setGpuRestartRequired(enabled !== gpuActiveAtStartupRef.current);
    settingsIpc
      .setHardwareAcceleration(enabled)
      .then((status) => {
        if (status && typeof status.enabled === "boolean") {
          gpuActiveAtStartupRef.current = status.active_at_startup;
          setHardwareAcceleration(status.enabled);
          setGpuRestartRequired(status.enabled !== status.active_at_startup);
        }
      })
      .catch(() => {});
  };

  const restartApp = () => {
    windowIpc.restartApp().catch(() => {});
  };

  return {
    locale,
    needsLanguageOnboarding,
    previewOnboardingLocale,
    confirmLanguageOnboarding,
    needsFlashOnboarding,
    previewOnboardingFlashKey,
    confirmFlashOnboarding,
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
    preferencesReady,
  };
}

import { useCallback, useEffect, useRef, useState } from "react";
import {
  AppLocale,
  DEFAULT_PREFERENCES,
  FlashKeyPreference,
  NavigationState,
  NavigationStatePatch,
  UserPreferences,
} from "../../types";
import { settingsIpc, windowIpc } from "../../services/ipc";
import { DEFAULT_ONBOARDING_LOCALE, isValidAppLocale } from "../../i18n";
import {
  DEFAULT_PROFILE_BLUR,
  clampProfileBlur,
  isValidAppSettings,
  mergeNavigationState,
} from "./preferencesStorage";
import { extractHydratedIniSettings, fetchInitialIniSettings } from "./preferencesHydration";

export * from "./preferencesStorage";

export function usePreferences(
  onFlashKeyChange?: (newKey: FlashKeyPreference) => void,
  onPatchProtectionChange?: (enabled: boolean) => void,
  onLocaleChange?: (locale: AppLocale) => void
) {
  const [locale, setLocale] = useState<AppLocale>(DEFAULT_ONBOARDING_LOCALE);
  const [needsLanguageOnboarding, setNeedsLanguageOnboarding] = useState<boolean>(false);
  const [needsFlashOnboarding, setNeedsFlashOnboarding] = useState<boolean>(false);
  const [preferences, setPreferences] = useState<UserPreferences>(() => ({
    ...DEFAULT_PREFERENCES,
    language: DEFAULT_ONBOARDING_LOCALE,
  }));
  const [profileBlur, setProfileBlur] = useState<number>(DEFAULT_PROFILE_BLUR);
  const [wallpaperVisible, setWallpaperVisible] = useState<boolean>(true);
  const [hardwareAcceleration, setHardwareAcceleration] = useState<boolean>(true);
  const [gpuRestartRequired, setGpuRestartRequired] = useState<boolean>(false);
  const [navigationState, setNavigationState] = useState<NavigationState | null>(null);
  const [preferencesReady, setPreferencesReady] = useState<boolean>(false);

  const gpuActiveAtStartupRef = useRef<boolean>(true);
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
    settingsIpc.syncUserPreferences(next).catch(() => {});
  }, []);

  const changeLanguage = useCallback(
    (nextLocale: AppLocale) => {
      setLocale(nextLocale);
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
      persistPreferences(next);
      if (key === "patch48hProtection") onPatchProtectionChange?.(Boolean(value));
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
    settingsIpc.completeLanguageOnboarding(locale).then(notify).catch(notify);
  }, [locale, persistPreferences, onLocaleChange]);

  const previewOnboardingFlashKey = useCallback((nextKey: FlashKeyPreference) => {
    setPreferences((prev) => ({ ...prev, flashKey: nextKey }));
  }, []);

  const confirmFlashOnboarding = useCallback(() => {
    const selectedFlashKey = preferences.flashKey;
    setNeedsFlashOnboarding(false);
    setPreferences((prev) => {
      const next = { ...prev, flashKey: selectedFlashKey };
      persistPreferences(next);
      return next;
    });
    settingsIpc.completeFlashOnboarding(selectedFlashKey).catch(() => {});
    onFlashKeyChange?.(selectedFlashKey);
  }, [preferences.flashKey, persistPreferences, onFlashKeyChange]);

  const updateProfileBlur = useCallback((blurPercent: number) => {
    const clamped = clampProfileBlur(blurPercent);
    profileBlurRef.current = clamped;
    setProfileBlur(clamped);
    settingsIpc.updateProfileAppearance(clamped, wallpaperVisibleRef.current).catch(() => {});
  }, []);

  const updateWallpaperVisible = useCallback((visible: boolean) => {
    wallpaperVisibleRef.current = visible;
    setWallpaperVisible(visible);
    settingsIpc.updateProfileAppearance(profileBlurRef.current, visible).catch(() => {});
  }, []);

  const updateNavigationState = useCallback((patch: NavigationStatePatch) => {
    setNavigationState((prev) => mergeNavigationState(prev, patch));
    settingsIpc
      .updateNavigationState(patch)
      .then((updated) => {
        if (isValidAppSettings(updated)) setNavigationState(updated.navigation);
      })
      .catch(() => {});
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
    iniBacked: preferencesReady,
    preferencesReady,
  };
}

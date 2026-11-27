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
import { DEFAULT_ONBOARDING_LOCALE, getSavedAppLocale, isValidAppLocale } from "../../i18n";
import {
  isValidAppSettings,
  mergeNavigationState,
} from "./preferencesStorage";
import { extractHydratedIniSettings, fetchInitialIniSettings } from "./preferencesHydration";
import {
  PROFILE_BLUR_PERSIST_DEBOUNCE_MS,
  type ProfileBlurUpdateOptions,
  useProfileAppearance,
} from "./useProfileAppearance";

export * from "./preferencesStorage";
export { PROFILE_BLUR_PERSIST_DEBOUNCE_MS, type ProfileBlurUpdateOptions };

export function usePreferences(
  onFlashKeyChange?: (newKey: FlashKeyPreference) => void,
  onPatchProtectionChange?: (enabled: boolean) => void,
  onLocaleChange?: (locale: AppLocale) => void
) {
  const [locale, setLocale] = useState<AppLocale>(
    () => getSavedAppLocale() ?? DEFAULT_ONBOARDING_LOCALE
  );
  const [needsLanguageOnboarding, setNeedsLanguageOnboarding] = useState<boolean>(false);
  const [needsFlashOnboarding, setNeedsFlashOnboarding] = useState<boolean>(false);
  const [closeToTray, setCloseToTray] = useState<boolean>(false);
  const [preferences, setPreferences] = useState<UserPreferences>(() => ({
    ...DEFAULT_PREFERENCES,
    language: getSavedAppLocale() ?? DEFAULT_ONBOARDING_LOCALE,
  }));
  const [hardwareAcceleration, setHardwareAcceleration] = useState<boolean>(true);
  const [gpuRestartRequired, setGpuRestartRequired] = useState<boolean>(false);
  const [navigationState, setNavigationState] = useState<NavigationState | null>(null);
  const [preferencesReady, setPreferencesReady] = useState<boolean>(false);

  const {
    profileBlur,
    wallpaperVisible,
    hydrateProfileAppearance,
    updateProfileBlur,
    updateWallpaperVisible,
  } = useProfileAppearance();

  const gpuActiveAtStartupRef = useRef<boolean>(true);
  const onFlashKeyChangeRef = useRef(onFlashKeyChange);
  onFlashKeyChangeRef.current = onFlashKeyChange;
  const onPatchProtectionChangeRef = useRef(onPatchProtectionChange);
  onPatchProtectionChangeRef.current = onPatchProtectionChange;
  const onLocaleChangeRef = useRef(onLocaleChange);
  onLocaleChangeRef.current = onLocaleChange;

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
          setCloseToTray(hydrated.closeToTray);
          setPreferences(hydrated.preferences);
          hydrateProfileAppearance(hydrated.blur, hydrated.wallpaperVisible);
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
        if (active) setPreferencesReady(true);
      }
    };

    initialize();
    return () => {
      active = false;
    };
  }, [hydrateProfileAppearance]);

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
      const notify = () => onLocaleChangeRef.current?.(nextLocale);
      settingsIpc.setLocale(nextLocale).then(notify).catch(notify);
    },
    [persistPreferences]
  );

  const updatePreference = useCallback(
    <K extends keyof UserPreferences>(key: K, value: UserPreferences[K]) => {
      if (key === "language" && isValidAppLocale(value)) {
        changeLanguage(value);
        return;
      }
      setPreferences((prev) => {
        const next = { ...prev, [key]: value };
        persistPreferences(next);
        if (key === "patch48hProtection") onPatchProtectionChangeRef.current?.(Boolean(value));
        return next;
      });
      if (key === "flashKey") onFlashKeyChangeRef.current?.(value as FlashKeyPreference);
    },
    [changeLanguage, persistPreferences]
  );

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
    const notify = () => onLocaleChangeRef.current?.(locale);
    settingsIpc.completeLanguageOnboarding(locale).then(notify).catch(notify);
  }, [locale, persistPreferences]);

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
    onFlashKeyChangeRef.current?.(selectedFlashKey);
  }, [preferences.flashKey, persistPreferences]);

  const updateCloseToTray = useCallback((enabled: boolean) => {
    setCloseToTray(enabled);
    settingsIpc
      .setCloseToTray(enabled)
      .then((updated) => {
        if (isValidAppSettings(updated)) setCloseToTray(Boolean(updated.general.closeToTray));
      })
      .catch(() => {});
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

  const updateHardwareAcceleration = useCallback((enabled: boolean) => {
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
  }, []);

  const restartApp = useCallback(() => {
    windowIpc.restartApp().catch(() => {});
  }, []);

  return {
    locale,
    needsLanguageOnboarding,
    previewOnboardingLocale,
    confirmLanguageOnboarding,
    needsFlashOnboarding,
    previewOnboardingFlashKey,
    confirmFlashOnboarding,
    changeLanguage,
    closeToTray,
    updateCloseToTray,
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

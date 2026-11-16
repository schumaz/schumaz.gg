import { AppLocale, AppSettings, UserPreferences } from "../../types";
import { settingsIpc } from "../../services/ipc";
import { DEFAULT_ONBOARDING_LOCALE, isValidAppLocale } from "../../i18n";
import {
  buildLegacyMigrationPayload,
  clampProfileBlur,
  clearLegacyLocalStorageKeys,
  isValidAppSettings,
} from "./preferencesStorage";

export interface HydratedIniSettings {
  resolvedLocale: AppLocale;
  needsOnboarding: boolean;
  needsFlashOnboarding: boolean;
  preferences: UserPreferences;
  blur: number;
  wallpaperVisible: boolean;
  hardwareAcceleration: boolean;
  navigation: AppSettings["navigation"];
}

export function extractHydratedIniSettings(settings: AppSettings): HydratedIniSettings {
  clearLegacyLocalStorageKeys();

  const resolvedLocale: AppLocale = isValidAppLocale(settings.general.language)
    ? settings.general.language
    : isValidAppLocale(settings.automation.language)
    ? settings.automation.language
    : DEFAULT_ONBOARDING_LOCALE;

  return {
    resolvedLocale,
    needsOnboarding: !settings.general.languageOnboardingCompleted,
    needsFlashOnboarding: !settings.general.flashOnboardingCompleted,
    preferences: {
      ...settings.automation,
      language: resolvedLocale,
    },
    blur: clampProfileBlur(settings.profileAppearance.profileBlurPercent),
    wallpaperVisible: Boolean(settings.profileAppearance.wallpaperVisible),
    hardwareAcceleration: Boolean(settings.graphics.hardwareAcceleration),
    navigation: settings.navigation,
  };
}

export async function fetchInitialIniSettings(): Promise<AppSettings | null> {
  try {
    const migrated = await settingsIpc.migrateLegacyLocalStorage(buildLegacyMigrationPayload());
    if (isValidAppSettings(migrated)) {
      return migrated;
    }
  } catch {
    // Fallback below
  }

  try {
    const loaded = await settingsIpc.getAppSettings();
    if (isValidAppSettings(loaded)) {
      return loaded;
    }
  } catch {
    // Fallback to legacy localStorage mode in non-Tauri environments
  }

  return null;
}

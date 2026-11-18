import {
  commands,
  type AppLocale,
  type AppSettings,
  type CacheClearScope,
  type FlashKeyPreference,
  type HardwareAccelerationStatus,
  type LocalCacheInfo,
  type NavigationStatePatch,
  type UserPreferences,
} from "../../types/bindings";

/**
 * Typed IPC adapter for unified `UserSettings.ini`, preferences, locale, navigation state, cache, and GPU settings.
 */
export const settingsIpc = {
  async getAppSettings(): Promise<AppSettings> {
    return commands.getAppSettings();
  },

  async syncUserPreferences(preferences: UserPreferences): Promise<void> {
    return commands.syncUserPreferences(preferences);
  },

  async setLocale(locale: AppLocale): Promise<void> {
    return commands.setLocale(locale);
  },

  async updateProfileAppearance(
    blurPercent: number,
    wallpaperVisible: boolean
  ): Promise<AppSettings> {
    return commands.updateProfileAppearance(blurPercent, wallpaperVisible);
  },

  async updateNavigationState(patch: NavigationStatePatch): Promise<AppSettings> {
    return commands.updateNavigationState(patch);
  },

  async completeLanguageOnboarding(locale: AppLocale): Promise<AppSettings> {
    return commands.completeLanguageOnboarding(locale);
  },

  async completeFlashOnboarding(flashKey: FlashKeyPreference): Promise<AppSettings> {
    return commands.completeFlashOnboarding(flashKey);
  },

  async getHardwareAccelerationStatus(): Promise<HardwareAccelerationStatus> {
    return commands.getHardwareAccelerationStatus();
  },

  async setHardwareAcceleration(enabled: boolean): Promise<HardwareAccelerationStatus> {
    return commands.setHardwareAcceleration(enabled);
  },

  async getLocalCacheInfo(): Promise<LocalCacheInfo> {
    return commands.getLocalCacheInfo();
  },

  async clearLocalCache(scope: CacheClearScope): Promise<LocalCacheInfo> {
    return commands.clearLocalCache(scope);
  },
};

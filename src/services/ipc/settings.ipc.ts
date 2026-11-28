import {
  commands,
  events,
  type AppLocale,
  type AppSettings,
  type AutostartStatus,
  type CacheClearScope,
  type FlashKeyPreference,
  type HardwareAccelerationStatus,
  type LocalCacheInfo,
  type NavigationStatePatch,
  type UserPreferences,
} from "../../types/bindings";
import type { UnlistenFn } from "./lcu.ipc";

export type { AutostartStatus };

const cacheClearedListeners = new Set<(scope: CacheClearScope) => void>();

/**
 * Typed IPC adapter for unified `UserSettings.ini`, preferences, locale, tray/autostart, cache, and GPU settings.
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

  async setCloseToTray(enabled: boolean): Promise<AppSettings> {
    return commands.setCloseToTray(enabled);
  },

  async getAutostartStatus(): Promise<AutostartStatus> {
    return commands.getAutostartStatus();
  },

  async setAutostartEnabled(enabled: boolean): Promise<AutostartStatus> {
    return commands.setAutostartEnabled(enabled);
  },

  async getLocalCacheInfo(): Promise<LocalCacheInfo> {
    return commands.getLocalCacheInfo();
  },

  async clearLocalCache(scope: CacheClearScope): Promise<LocalCacheInfo> {
    const info = await commands.clearLocalCache(scope);
    for (const listener of cacheClearedListeners) {
      listener(scope);
    }
    return info;
  },

  async onCacheCleared(handler: (scope: CacheClearScope) => void): Promise<UnlistenFn> {
    cacheClearedListeners.add(handler);
    let unlistenEvent: UnlistenFn | null = null;
    if (typeof events?.cacheCleared?.listen === "function") {
      unlistenEvent = await events.cacheCleared.listen((event) => handler(event.payload));
    }
    return () => {
      cacheClearedListeners.delete(handler);
      unlistenEvent?.();
    };
  },
};

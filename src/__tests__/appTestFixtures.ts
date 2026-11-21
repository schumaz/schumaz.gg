import type { AppSettings } from "../types";
import { mockAhriBuild } from "./mockAhriBuildFixture";

export { mockAhriBuild };

export function makeDefaultMockAppSettings(): AppSettings {
  return {
    general: {
      language: "pt_BR",
      languageOnboardingCompleted: true,
      flashOnboardingCompleted: true,
    },
    automation: {
      autoInjectRunes: true,
      autoInjectSpells: true,
      autoInjectItems: true,
      flashKey: "D",
      patch48hProtection: true,
      autoMarkDailyMissionsRead: false,
      language: "pt_BR",
    },
    profileAppearance: {
      profileBlurPercent: 10,
      wallpaperVisible: true,
    },
    graphics: {
      hardwareAcceleration: true,
    },
    window: {
      windowWidth: 1280,
      windowHeight: 720,
      windowX: null,
      windowY: null,
      windowMaximized: false,
      windowMonitor: null,
    },
    navigation: {
      savedAramMode: null,
      profileMatchModeFilter: "all",
      aramMayhemLastChampion: null,
      rankedFilterRegion: "global",
      rankedFilterTier: "emerald_plus",
      rankedFilterLane: "all",
      rankedFilterPatch: null,
      rankedChampionTierFilter: "all",
      lastSummonerId: null,
    },
  };
}

export let activeMockAppSettings: AppSettings = makeDefaultMockAppSettings();

export function resetMockAppSettings(overrides?: Partial<AppSettings>): void {
  activeMockAppSettings = makeDefaultMockAppSettings();
  if (overrides) {
    if (overrides.general) {
      activeMockAppSettings.general = { ...activeMockAppSettings.general, ...overrides.general };
    }
    if (overrides.automation) {
      activeMockAppSettings.automation = { ...activeMockAppSettings.automation, ...overrides.automation };
    }
    if (overrides.profileAppearance) {
      activeMockAppSettings.profileAppearance = { ...activeMockAppSettings.profileAppearance, ...overrides.profileAppearance };
    }
    if (overrides.graphics) {
      activeMockAppSettings.graphics = { ...activeMockAppSettings.graphics, ...overrides.graphics };
    }
    if (overrides.window) {
      activeMockAppSettings.window = { ...activeMockAppSettings.window, ...overrides.window };
    }
    if (overrides.navigation) {
      activeMockAppSettings.navigation = { ...activeMockAppSettings.navigation, ...overrides.navigation };
    }
  }
}

export function defaultAppInvokeMock(cmd: string, args?: Record<string, unknown>) {
  if (cmd === "get_app_settings") {
    return Promise.resolve(activeMockAppSettings);
  }
  if (cmd === "sync_user_preferences") {
    if (args?.preferences) {
      activeMockAppSettings.automation = args.preferences as any;
    }
    return Promise.resolve(null);
  }
  if (cmd === "set_locale") {
    if (args?.locale) {
      activeMockAppSettings.general.language = args.locale as any;
      activeMockAppSettings.automation.language = args.locale as any;
    }
    return Promise.resolve(null);
  }
  if (cmd === "complete_language_onboarding") {
    if (args?.locale) {
      activeMockAppSettings.general.language = args.locale as any;
      activeMockAppSettings.general.languageOnboardingCompleted = true;
      activeMockAppSettings.automation.language = args.locale as any;
    }
    return Promise.resolve(activeMockAppSettings);
  }
  if (cmd === "complete_flash_onboarding") {
    if (args?.flashKey) {
      activeMockAppSettings.general.flashOnboardingCompleted = true;
      activeMockAppSettings.automation.flashKey = args.flashKey as any;
    }
    return Promise.resolve(activeMockAppSettings);
  }
  if (cmd === "update_profile_appearance") {
    activeMockAppSettings.profileAppearance.profileBlurPercent = Number(args?.blurPercent ?? 10);
    activeMockAppSettings.profileAppearance.wallpaperVisible = Boolean(args?.wallpaperVisible ?? true);
    return Promise.resolve(activeMockAppSettings);
  }
  if (cmd === "update_navigation_state") {
    activeMockAppSettings.navigation = {
      ...activeMockAppSettings.navigation,
      ...(args?.patch as any),
    };
    return Promise.resolve(activeMockAppSettings);
  }
  if (cmd === "get_hardware_acceleration_status") {
    return Promise.resolve({
      enabled: activeMockAppSettings.graphics.hardwareAcceleration,
      active_at_startup: true,
    });
  }
  if (
    cmd === "get_summoner_status" ||
    cmd === "get_player_profile" ||
    cmd === "get_cached_player_profile"
  ) {
    return Promise.resolve(null);
  }
  if (cmd === "get_active_champ_select_snapshot") {
    return Promise.resolve({ gameflow_phase: null, champ_selected: null, build: null });
  }
  if (cmd === "get_recommended_build_for_champion") {
    if (args?.queueId === 2400) {
      return Promise.resolve({
        ...mockAhriBuild,
        active_position: "",
        supports_runes: false,
        rune_pages: [],
      });
    }
    if (args?.queueId === 450) {
      return Promise.resolve({
        ...mockAhriBuild,
        active_position: "",
        supports_runes: true,
      });
    }
    return Promise.resolve(mockAhriBuild);
  }
  if (cmd === "update_build_filters") {
    return Promise.resolve({
      ...mockAhriBuild,
      active_position: (args?.position as string) || mockAhriBuild.active_position,
      active_tier: (args?.tier as string) || mockAhriBuild.active_tier,
      active_region: (args?.region as string) || mockAhriBuild.active_region,
      active_patch: (args?.patch as string) || mockAhriBuild.active_patch,
    });
  }
  return Promise.resolve({});
}

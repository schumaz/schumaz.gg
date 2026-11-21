import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { waitFor, act, renderHook } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { usePreferences } from "../features/settings/usePreferences";
import { useAram } from "../features/aram/useAram";
import { useRanked } from "../features/ranked/useRanked";
import { buildAssetSegment, isAllowedAssetUrl, resolveCachedAssetUrl } from "../utils/assets";
import type { AppSettings } from "../types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue({}),
  convertFileSrc: vi.fn(
    (filePath: string, protocol = "asset") => `${protocol}://localhost/${filePath}`
  ),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

function makeIniSettings(overrides: Partial<AppSettings> = {}): AppSettings {
  return {
    general: {
      language: "pt_BR",
      languageOnboardingCompleted: true,
      flashOnboardingCompleted: true,
      closeToTray: false,
    },
    automation: {
      autoInjectRunes: true,
      autoInjectSpells: false,
      autoInjectItems: true,
      flashKey: "F",
      patch48hProtection: true,
      autoMarkDailyMissionsRead: true,
      language: "pt_BR",
    },
    profileAppearance: {
      profileBlurPercent: 42,
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
      savedAramMode: "aram_mayhem",
      profileMatchModeFilter: "420",
      aramMayhemLastChampion: "ahri",
      rankedFilterRegion: "kr",
      rankedFilterTier: "master_plus",
      rankedFilterLane: "mid",
      rankedFilterPatch: "16.22",
      rankedChampionTierFilter: "0",
      lastSummonerId: 123456,
    },
    ...overrides,
  };
}

describe("P9.4 Persistent Disk Cache & UserSettings.ini Integration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  afterEach(() => {
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  describe("Asset URI Resolver (utils/assets.ts)", () => {
    it("returns remote URL verbatim outside Tauri runtime (jsdom)", () => {
      const url = "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Ahri.png";
      expect(resolveCachedAssetUrl(url)).toBe(url);
    });

    it("rewrites allowed domains to schumaz-asset:// inside Tauri runtime and blocks untrusted hosts", () => {
      (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};

      const ddUrl = "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_15.jpg";
      const cdUrl =
        "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/svg/position-top-light.svg";
      const opggUrl = "https://opgg-static.akamaized.net/meta/images/lol/16.22.1/perk/8112.png";
      const untrustedUrl = "https://evil.example.com/payload.png";

      expect(isAllowedAssetUrl(ddUrl)).toBe(true);
      expect(isAllowedAssetUrl(cdUrl)).toBe(true);
      expect(isAllowedAssetUrl(opggUrl)).toBe(false);
      expect(isAllowedAssetUrl(untrustedUrl)).toBe(false);

      expect(resolveCachedAssetUrl(ddUrl)).toBe(
        `schumaz-asset://localhost/${buildAssetSegment(ddUrl)}`
      );
      expect(resolveCachedAssetUrl(cdUrl)).toBe(
        `schumaz-asset://localhost/${buildAssetSegment(cdUrl)}`
      );
      expect(resolveCachedAssetUrl(opggUrl)).toBe(opggUrl);
      expect(resolveCachedAssetUrl(untrustedUrl)).toBe(untrustedUrl);
      expect(buildAssetSegment(ddUrl)).toMatch(/^v1_[A-Za-z0-9_-]+\.jpg$/);
    });
  });

  describe("usePreferences UserSettings.ini hydration & updates", () => {
    it("hydrates settings directly from get_app_settings IPC without writing localStorage", async () => {
      const ini = makeIniSettings();
      vi.mocked(invoke).mockImplementation((cmd: string) => {
        if (cmd === "get_app_settings") return Promise.resolve(ini);
        if (cmd === "get_hardware_acceleration_status") {
          return Promise.resolve({ enabled: true, active_at_startup: true });
        }
        return Promise.resolve({});
      });

      const { result } = renderHook(() => usePreferences());

      await waitFor(() => {
        expect(result.current.preferencesReady).toBe(true);
      });

      expect(invoke).toHaveBeenCalledWith("get_app_settings");
      expect(result.current.profileBlur).toBe(42);
      expect(result.current.closeToTray).toBe(false);
      expect(result.current.preferences.flashKey).toBe("F");
      expect(result.current.navigationState?.rankedFilterRegion).toBe("kr");

      act(() => {
        result.current.updateProfileBlur(75);
        result.current.updateCloseToTray(true);
        result.current.updateNavigationState({ rankedChampionTierFilter: "1" });
      });

      expect(result.current.closeToTray).toBe(true);
      expect(invoke).toHaveBeenCalledWith("set_close_to_tray", { enabled: true });
      expect(invoke).toHaveBeenCalledWith("update_profile_appearance", {
        blurPercent: 75,
        wallpaperVisible: true,
      });
      expect(invoke).toHaveBeenCalledWith("update_navigation_state", {
        patch: { rankedChampionTierFilter: "1" },
      });
      expect(localStorage.getItem("schumaz_profile_ui_blur")).toBeNull();
    });
  });

  describe("useAram and useRanked INI navigation persistence", () => {
    it("persists ARAM submode and Ranked filters through onNavigationPatch without writing localStorage", () => {
      const onNavigationPatch = vi.fn();

      const aramHook = renderHook(() =>
        useAram(false, null, "pt_BR", {
          savedAramMode: "aram",
          savedLastChampion: "jinx",
          onNavigationPatch,
        })
      );

      act(() => {
        aramHook.result.current.selectMode("aram_mayhem");
      });

      expect(onNavigationPatch).toHaveBeenCalledWith({ savedAramMode: "aram_mayhem" });
      expect(localStorage.getItem("schumaz_aram_submode")).toBeNull();

      const rankedHook = renderHook(() =>
        useRanked(false, "D", true, "pt_BR", {
          savedFilters: { region: "kr", tier: "diamond_plus", lane: "mid", patch: "16.22" },
          onNavigationPatch,
        })
      );

      expect(rankedHook.result.current.filters.region).toBe("kr");

      act(() => {
        rankedHook.result.current.handleTierListFilterChange("region", "br");
      });

      expect(onNavigationPatch).toHaveBeenCalledWith({
        rankedFilterRegion: "br",
        rankedFilterTier: "diamond_plus",
        rankedFilterLane: "mid",
        rankedFilterPatch: "16.22",
      });
      expect(localStorage.getItem("schumaz_ranked_filters")).toBeNull();
    });
  });
});

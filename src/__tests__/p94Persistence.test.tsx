import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, act, renderHook } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import App from "../App";
import { usePreferences } from "../hooks/usePreferences";
import { usePlayerProfile } from "../hooks/usePlayerProfile";
import { useAram } from "../hooks/useAram";
import { useRanked } from "../hooks/useRanked";
import { buildAssetSegment, resolveCachedAssetUrl } from "../utils/assets";
import type { AppSettings, CachedProfileResult, PlayerProfile, SummonerInfo } from "../types";

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
      legacyStorageMigrated: true,
    },
    automation: {
      autoInjectRunes: true,
      autoInjectSpells: false,
      autoInjectItems: true,
      flashKey: "F",
      patch48hProtection: true,
      autoMarkDailyMissionsRead: true,
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
      windowHeight: 800,
      windowX: null,
      windowY: null,
      windowMaximized: false,
    },
    navigation: {
      savedAramMode: "aram_mayhem",
      profileMatchModeFilter: "420",
      aramMayhemRarityFilter: "8",
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

function makeProfile(summonerId: number, gameName = "Schuma"): PlayerProfile {
  return {
    summoner: {
      summonerId,
      gameName,
      tagLine: "BR1",
      displayName: gameName,
      summonerLevel: 400,
      profileIconId: 548,
    },
    profile_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/profileicon/548.png",
    background_skin_id: 103015,
    background_skin_name: "Ahri Florescer Espiritual",
    background_splash_url: "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_15.jpg",
    background_splash_fallback_url:
      "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_0.jpg",
    ranked_stats: [],
    top_masteries: [],
    recent_matches: [],
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

      expect(resolveCachedAssetUrl(ddUrl)).toBe(
        `schumaz-asset://localhost/${buildAssetSegment(ddUrl)}`
      );
      expect(resolveCachedAssetUrl(cdUrl)).toBe(
        `schumaz-asset://localhost/${buildAssetSegment(cdUrl)}`
      );
      expect(resolveCachedAssetUrl(opggUrl)).toBe(
        `schumaz-asset://localhost/${buildAssetSegment(opggUrl)}`
      );
      expect(resolveCachedAssetUrl(untrustedUrl)).toBe(untrustedUrl);
      expect(buildAssetSegment(ddUrl)).toMatch(/^v1_[A-Za-z0-9_-]+\.jpg$/);
    });
  });

  describe("usePreferences legacy 6-key migration & UserSettings.ini hydration", () => {
    it("migrates all 6 legacy localStorage keys to INI and clears localStorage", async () => {
      localStorage.setItem("schumaz_user_preferences", JSON.stringify({ flashKey: "F" }));
      localStorage.setItem("schumaz_language", "pt_BR");
      localStorage.setItem("schumaz_profile_ui_blur", "42");
      localStorage.setItem("schumaz_profile_wallpaper_visible", "true");
      localStorage.setItem("schumaz_aram_submode", "aram_mayhem");
      localStorage.setItem("schumaz_ranked_filters", JSON.stringify({ region: "kr" }));

      const ini = makeIniSettings();
      vi.mocked(invoke).mockImplementation((cmd: string) => {
        if (cmd === "migrate_legacy_local_storage") return Promise.resolve(ini);
        if (cmd === "get_hardware_acceleration_status") {
          return Promise.resolve({ enabled: true, active_at_startup: true });
        }
        return Promise.resolve({});
      });

      const { result } = renderHook(() => usePreferences());

      await waitFor(() => {
        expect(result.current.iniBacked).toBe(true);
      });

      expect(invoke).toHaveBeenCalledWith("migrate_legacy_local_storage", {
        payload: {
          schumazUserPreferences: JSON.stringify({ flashKey: "F" }),
          schumazLanguage: "pt_BR",
          schumazProfileUiBlur: "42",
          schumazProfileWallpaperVisible: "true",
          schumazAramSubmode: "aram_mayhem",
          schumazRankedFilters: JSON.stringify({ region: "kr" }),
        },
      });

      // All 6 legacy localStorage keys must be cleared after INI migration
      expect(localStorage.getItem("schumaz_user_preferences")).toBeNull();
      expect(localStorage.getItem("schumaz_language")).toBeNull();
      expect(localStorage.getItem("schumaz_profile_ui_blur")).toBeNull();
      expect(localStorage.getItem("schumaz_profile_wallpaper_visible")).toBeNull();
      expect(localStorage.getItem("schumaz_aram_submode")).toBeNull();
      expect(localStorage.getItem("schumaz_ranked_filters")).toBeNull();

      // INI values are hydrated
      expect(result.current.profileBlur).toBe(42);
      expect(result.current.preferences.flashKey).toBe("F");
      expect(result.current.navigationState?.rankedFilterRegion).toBe("kr");

      // Appearance & navigation updates persist via INI commands, not localStorage
      act(() => {
        result.current.updateProfileBlur(75);
        result.current.updateNavigationState({ aramMayhemRarityFilter: "4" });
      });

      expect(invoke).toHaveBeenCalledWith("update_profile_appearance", {
        blurPercent: 75,
        wallpaperVisible: true,
      });
      expect(invoke).toHaveBeenCalledWith("update_navigation_state", {
        patch: { aramMayhemRarityFilter: "4" },
      });
      expect(localStorage.getItem("schumaz_profile_ui_blur")).toBeNull();
    });
  });

  describe("usePlayerProfile startup cache & Stale-While-Revalidate", () => {
    it("displays cached last summoner and wallpaper on App startup before LCU connects", async () => {
      const cachedProfile = makeProfile(999111, "CachedPlayer");
      const cachedResult: CachedProfileResult = {
        profile: cachedProfile,
        createdAtUnix: 1700000000,
        isStale: false,
      };
      const ini = makeIniSettings();

      vi.mocked(invoke).mockImplementation((cmd: string) => {
        if (cmd === "get_summoner_status") return Promise.resolve(null);
        if (cmd === "migrate_legacy_local_storage") return Promise.resolve(ini);
        if (cmd === "get_cached_player_profile") return Promise.resolve(cachedResult);
        if (cmd === "get_hardware_acceleration_status") {
          return Promise.resolve({ enabled: true, active_at_startup: true });
        }
        return Promise.resolve({});
      });

      render(<App />);

      await waitFor(() => {
        expect(screen.getByTestId("profile-wallpaper")).toBeInTheDocument();
      });

      expect(screen.getByText("CachedPlayer")).toBeInTheDocument();
      expect(screen.getByTestId("refresh-profile-btn")).toBeDisabled();
      expect(invoke).not.toHaveBeenCalledWith("get_player_profile", expect.anything());
    });

    it("skips LCU network fetch when connected cache is fresh and triggers SWR when stale (> 5 min)", async () => {
      const summoner: SummonerInfo = {
        summonerId: 777,
        gameName: "LiveUser",
        tagLine: "BR1",
        displayName: "LiveUser",
        summonerLevel: 200,
        profileIconId: 10,
      };
      const freshCached: CachedProfileResult = {
        profile: makeProfile(777, "LiveUser"),
        createdAtUnix: 1700000000,
        isStale: false,
      };

      vi.mocked(invoke).mockImplementation((cmd: string) => {
        if (cmd === "get_cached_player_profile") return Promise.resolve(freshCached);
        if (cmd === "get_player_profile") return Promise.resolve(makeProfile(777, "RefreshedUser"));
        return Promise.resolve(null);
      });

      const { result, rerender } = renderHook(
        ({ s }: { s: SummonerInfo | null }) => usePlayerProfile(s, "pt_BR"),
        { initialProps: { s: summoner as SummonerInfo | null } }
      );

      await waitFor(() => {
        expect(result.current.profile?.summoner.gameName).toBe("LiveUser");
      });
      expect(invoke).not.toHaveBeenCalledWith("get_player_profile", expect.anything());

      // Now connect a second account with a stale cache -> triggers silent SWR forceRefresh: true
      const staleSummoner: SummonerInfo = { ...summoner, summonerId: 888, gameName: "StaleUser" };
      const staleCached: CachedProfileResult = {
        profile: makeProfile(888, "StaleUserCached"),
        createdAtUnix: 1699999000,
        isStale: true,
      };
      vi.mocked(invoke).mockImplementation((cmd: string) => {
        if (cmd === "get_cached_player_profile") return Promise.resolve(staleCached);
        if (cmd === "get_player_profile") return Promise.resolve(makeProfile(888, "StaleUserLive"));
        return Promise.resolve(null);
      });

      rerender({ s: staleSummoner });

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith("get_player_profile", { forceRefresh: true });
      });
      await waitFor(() => {
        expect(result.current.profile?.summoner.gameName).toBe("StaleUserLive");
      });

      // Manual refresh uses forceRefresh: true
      act(() => {
        result.current.refreshProfile();
      });
      expect(invoke).toHaveBeenCalledWith("get_player_profile", { forceRefresh: true });
    });
  });

  describe("useAram and useRanked INI navigation persistence", () => {
    it("persists ARAM submode and Ranked filters through onNavigationPatch without writing localStorage", () => {
      const onNavigationPatch = vi.fn();

      const aramHook = renderHook(() =>
        useAram(false, null, "pt_BR", {
          iniBacked: true,
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
          iniBacked: true,
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

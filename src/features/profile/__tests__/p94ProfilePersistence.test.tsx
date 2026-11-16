import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, act, renderHook } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import App from "../../../App";
import { usePlayerProfile } from "../usePlayerProfile";
import type { AppSettings, CachedProfileResult, PlayerProfile, SummonerInfo } from "../../../types";

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

describe("P9.4 usePlayerProfile startup cache & Stale-While-Revalidate", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  afterEach(() => {
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

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

    act(() => {
      result.current.refreshProfile();
    });
    expect(invoke).toHaveBeenCalledWith("get_player_profile", { forceRefresh: true });
  });
});

import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  lcuIpc,
  buildIpc,
  diagnosticsIpc,
  injectorIpc,
  profileIpc,
  rankedIpc,
  arenaIpc,
  aramIpc,
  settingsIpc,
  updaterIpc,
  windowIpc,
  assetsIpc,
} from "../services/ipc";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string) => {
    if (cmd === "get_gameflow_phase") return Promise.resolve("Lobby");
    if (cmd === "get_active_champ_select_snapshot") {
      return Promise.resolve({ gameflow_phase: "Lobby", champ_selected: null, build: null });
    }
    if (cmd === "get_app_distribution_info") {
      return Promise.resolve({ isPortable: false, currentVersion: "0.1.0", releasesUrl: "", lastCheckEpochSecs: null, checkCooldownSecs: 86400 });
    }
    if (cmd === "get_aram_tier_list" || cmd === "get_aram_mayhem_augments") return Promise.resolve([]);
    if (cmd === "get_hardware_acceleration_status") return Promise.resolve({ enabled: true, active_at_startup: true });
    if (cmd === "get_diagnostic_summary") return Promise.resolve({ appVersion: "0.4.0", formattedText: "ok" });
    return Promise.resolve(null);
  }),
  convertFileSrc: vi.fn(
    (filePath: string, protocol = "asset") => `${protocol}://localhost/${filePath}`
  ),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

describe("Frontend IPC Adapters (src/services/ipc/)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("delegates lcu, build, diagnostics, injector, profile, ranked, arena, aram, settings, and window commands", async () => {
    await lcuIpc.getSummonerStatus();
    expect(invoke).toHaveBeenCalledWith("get_summoner_status");

    await lcuIpc.getGameflowPhase();
    expect(invoke).toHaveBeenCalledWith("get_gameflow_phase");

    const snap = await lcuIpc.getActiveChampSelectSnapshot();
    expect(invoke).toHaveBeenCalledWith("get_active_champ_select_snapshot");
    expect(snap).toEqual({ gameflow_phase: "Lobby", champ_selected: null, build: null });

    await lcuIpc.syncDisplayedChampion(103);
    expect(invoke).toHaveBeenCalledWith("sync_displayed_champion", { championId: 103 });

    await buildIpc.getRecommendedBuildForChampion(103, 420);
    expect(invoke).toHaveBeenCalledWith("get_recommended_build_for_champion", {
      championId: 103,
      queueId: 420,
    });

    await buildIpc.updateBuildFilters(103, 420, "mid", "emerald_plus", "br", "16.22");
    expect(invoke).toHaveBeenCalledWith("update_build_filters", {
      championId: 103,
      queueId: 420,
      position: "mid",
      tier: "emerald_plus",
      region: "br",
      patch: "16.22",
    });

    await injectorIpc.injectSummonerSpells(4, 14, 103);
    expect(invoke).toHaveBeenCalledWith("inject_summoner_spells", {
      championId: 103,
      spell1Id: 4,
      spell2Id: 14,
    });

    await injectorIpc.injectRunePage(103, "Ahri", 0, 8100, 8200, [8112]);
    expect(invoke).toHaveBeenCalledWith("inject_rune_page", {
      championId: 103,
      championName: "Ahri",
      pageIndex: 0,
      primaryStyleId: 8100,
      subStyleId: 8200,
      selectedPerkIds: [8112],
    });

    await injectorIpc.injectItemBuild(103, 420, 0);
    expect(invoke).toHaveBeenCalledWith("inject_item_build", {
      championId: 103,
      queueId: 420,
      buildIndex: 0,
    });

    await profileIpc.getPlayerProfile(true);
    expect(invoke).toHaveBeenCalledWith("get_player_profile", { forceRefresh: true });

    await profileIpc.getCachedPlayerProfile(null);
    expect(invoke).toHaveBeenCalledWith("get_cached_player_profile", { summonerId: null });

    await rankedIpc.getRankedTierList("br", "emerald_plus", "16.22");
    expect(invoke).toHaveBeenCalledWith("get_ranked_tier_list", {
      region: "br",
      tier: "emerald_plus",
      patch: "16.22",
    });

    await rankedIpc.getRankedChampionBuild(103, "mid", "emerald_plus", "br", "16.22");
    expect(invoke).toHaveBeenCalledWith("get_ranked_champion_build", {
      championId: 103,
      position: "mid",
      tier: "emerald_plus",
      region: "br",
      patch: "16.22",
    });

    await arenaIpc.getArenaTierList("16.22");
    expect(invoke).toHaveBeenCalledWith("get_arena_tier_list", { patch: "16.22" });

    await arenaIpc.getArenaChampionBuild(350, "16.22");
    expect(invoke).toHaveBeenCalledWith("get_arena_champion_build", {
      championId: 350,
      patch: "16.22",
    });

    await aramIpc.getAramTierList("aram_mayhem");
    expect(invoke).toHaveBeenCalledWith("get_aram_tier_list", { mode: "aram_mayhem" });

    await aramIpc.getAramMayhemAugments("ahri");
    expect(invoke).toHaveBeenCalledWith("get_aram_mayhem_augments", { championKey: "ahri" });

    await settingsIpc.getAppSettings();
    expect(invoke).toHaveBeenCalledWith("get_app_settings");

    await settingsIpc.syncUserPreferences({
      autoInjectRunes: true,
      autoInjectSpells: false,
      autoInjectItems: true,
      flashKey: "F",
      patch48hProtection: true,
      autoMarkDailyMissionsRead: true,
      language: "pt_BR",
    });
    expect(invoke).toHaveBeenCalledWith("sync_user_preferences", {
      preferences: {
        autoInjectRunes: true,
        autoInjectSpells: false,
        autoInjectItems: true,
        flashKey: "F",
        patch48hProtection: true,
        autoMarkDailyMissionsRead: true,
        language: "pt_BR",
      },
    });

    await settingsIpc.setLocale("pt_BR");
    expect(invoke).toHaveBeenCalledWith("set_locale", { locale: "pt_BR" });

    await settingsIpc.completeLanguageOnboarding("pt_BR");
    expect(invoke).toHaveBeenCalledWith("complete_language_onboarding", { locale: "pt_BR" });

    await settingsIpc.completeFlashOnboarding("F");
    expect(invoke).toHaveBeenCalledWith("complete_flash_onboarding", { flashKey: "F" });

    await settingsIpc.updateProfileAppearance(45, true);
    expect(invoke).toHaveBeenCalledWith("update_profile_appearance", {
      blurPercent: 45,
      wallpaperVisible: true,
    });

    await settingsIpc.updateNavigationState({ savedAramMode: "aram_mayhem" });
    expect(invoke).toHaveBeenCalledWith("update_navigation_state", {
      patch: { savedAramMode: "aram_mayhem" },
    });

    await settingsIpc.getHardwareAccelerationStatus();
    expect(invoke).toHaveBeenCalledWith("get_hardware_acceleration_status");

    await settingsIpc.setHardwareAcceleration(false);
    expect(invoke).toHaveBeenCalledWith("set_hardware_acceleration", { enabled: false });

    await settingsIpc.getLocalCacheInfo();
    expect(invoke).toHaveBeenCalledWith("get_local_cache_info");

    await settingsIpc.clearLocalCache("builds");
    expect(invoke).toHaveBeenCalledWith("clear_local_cache", { scope: "builds" });

    const diag = await diagnosticsIpc.getDiagnosticSummary();
    expect(invoke).toHaveBeenCalledWith("get_diagnostic_summary");
    expect(diag).toEqual({ appVersion: "0.4.0", formattedText: "ok" });

    await diagnosticsIpc.openLogsFolder();
    expect(invoke).toHaveBeenCalledWith("open_logs_folder");

    await windowIpc.getBootstrapStatus();
    expect(invoke).toHaveBeenCalledWith("get_bootstrap_status");

    await windowIpc.restartApp();
    expect(invoke).toHaveBeenCalledWith("restart_app");

    expect(await updaterIpc.getDistributionInfo()).toBeNull();
    expect(await updaterIpc.probeForUpdate(false)).toBeNull();
    expect(await updaterIpc.recordUpdateCheckCompleted()).toBeNull();
    expect(await updaterIpc.checkForUpdate()).toBeNull();
  });

  it("subscribes to domain events, unwraps payloads, and resolves asset URIs", async () => {
    let capturedHandler: ((event: { payload: unknown }) => void) | undefined;
    vi.mocked(listen).mockImplementationOnce((event: string, handler: unknown) => {
      expect(event).toBe("lcu-connected");
      capturedHandler = handler as (event: { payload: unknown }) => void;
      return Promise.resolve(() => {});
    });

    const received: unknown[] = [];
    const unlisten = await lcuIpc.onLcuConnected((summoner) => {
      received.push(summoner);
    });
    expect(typeof unlisten).toBe("function");
    expect(capturedHandler).toBeDefined();

    const mockSummoner = {
      summonerId: 100,
      gameName: "Tester",
      tagLine: "NA1",
      displayName: "Tester",
      summonerLevel: 50,
      profileIconId: 1,
    };
    capturedHandler?.({ payload: mockSummoner });

    expect(received).toEqual([mockSummoner]);
    expect(assetsIpc.isTauriRuntime()).toBe(false);
    expect(assetsIpc.convertAssetFileSrc("v1_xyz.png")).toBe("schumaz-asset://localhost/v1_xyz.png");
  });
});

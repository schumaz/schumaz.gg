import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import {
  aramIpc,
  assetsIpc,
  buildIpc,
  injectorIpc,
  lcuIpc,
  profileIpc,
  rankedIpc,
  settingsIpc,
  windowIpc,
} from "../services/ipc";

const listeners: Record<string, (event: { payload: unknown }) => void> = {};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(null),
  convertFileSrc: vi.fn((path: string, protocol = "asset") => `${protocol}://localhost/${path}`),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockImplementation((name: string, cb: (event: { payload: unknown }) => void) => {
    listeners[name] = cb;
    return Promise.resolve(() => {
      delete listeners[name];
    });
  }),
}));

describe("Frontend IPC Adapters (src/services/ipc/)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Object.keys(listeners).forEach((key) => delete listeners[key]);
  });

  it("delegates lcu, build, injector, profile, ranked, aram, settings, and window commands", async () => {
    await lcuIpc.getSummonerStatus();
    expect(invoke).toHaveBeenCalledWith("get_summoner_status");

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

    await injectorIpc.injectSummonerSpells(4, 14, 103, 14);
    expect(invoke).toHaveBeenCalledWith("inject_summoner_spells", {
      spell1Id: 4,
      spell2Id: 14,
      championId: 103,
      secondarySpellId: 14,
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

    await injectorIpc.injectItemBuild(103, 420, 0, 0, 0);
    expect(invoke).toHaveBeenCalledWith("inject_item_build", {
      championId: 103,
      queueId: 420,
      buildIndex: 0,
      coreIndex: 0,
      finalIndex: 0,
    });

    await profileIpc.getCachedPlayerProfile(123);
    expect(invoke).toHaveBeenCalledWith("get_cached_player_profile", { summonerId: 123 });

    await profileIpc.getPlayerProfile(true);
    expect(invoke).toHaveBeenCalledWith("get_player_profile", { forceRefresh: true });

    await rankedIpc.getRankedTierList("br", "emerald_plus", null);
    expect(invoke).toHaveBeenCalledWith("get_ranked_tier_list", {
      region: "br",
      tier: "emerald_plus",
      patch: null,
    });

    await rankedIpc.getRankedChampionBuild(103, "mid", "emerald_plus", "br", null);
    expect(invoke).toHaveBeenCalledWith("get_ranked_champion_build", {
      championId: 103,
      position: "mid",
      tier: "emerald_plus",
      region: "br",
      patch: null,
    });

    await aramIpc.getAramTierList("aram_mayhem");
    expect(invoke).toHaveBeenCalledWith("get_aram_tier_list", { mode: "aram_mayhem" });

    await aramIpc.getAramMayhemAugments("ahri");
    expect(invoke).toHaveBeenCalledWith("get_aram_mayhem_augments", { championKey: "ahri" });

    await settingsIpc.getAppSettings();
    expect(invoke).toHaveBeenCalledWith("get_app_settings");

    await settingsIpc.setLocale("pt_BR");
    expect(invoke).toHaveBeenCalledWith("set_locale", { locale: "pt_BR" });

    await windowIpc.restartApp();
    expect(invoke).toHaveBeenCalledWith("restart_app");
  });

  it("subscribes to domain events, unwraps payloads, and resolves asset URIs", async () => {
    const onConnected = vi.fn();
    const unlistenConnected = await lcuIpc.onLcuConnected(onConnected);
    listeners["lcu-connected"]({ payload: { summonerId: 42 } });
    expect(onConnected).toHaveBeenCalledWith({ summonerId: 42 });
    unlistenConnected();
    expect(listeners["lcu-connected"]).toBeUndefined();

    const onBuild = vi.fn();
    const unlistenBuild = await buildIpc.onRecommendedBuild(onBuild);
    listeners["recommended-build"]({ payload: { champion_id: 103 } });
    expect(onBuild).toHaveBeenCalledWith({ champion_id: 103 });
    unlistenBuild();

    const onRefresh = vi.fn();
    const unlistenRefresh = await profileIpc.onProfileRefreshRequested(onRefresh);
    listeners["profile-refresh-requested"]({ payload: null });
    expect(onRefresh).toHaveBeenCalledTimes(1);
    unlistenRefresh();

    const onMinimized = vi.fn();
    const unlistenMinimized = await windowIpc.onWindowMinimizedChanged(onMinimized);
    listeners["window-minimized-changed"]({ payload: true });
    expect(onMinimized).toHaveBeenCalledWith(true);
    unlistenMinimized();

    expect(assetsIpc.convertAssetFileSrc("v1_abc.png")).toBe("schumaz-asset://localhost/v1_abc.png");
    expect(convertFileSrc).toHaveBeenCalledWith("v1_abc.png", "schumaz-asset");
  });
});

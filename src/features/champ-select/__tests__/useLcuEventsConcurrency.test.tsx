import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useLcuEvents } from "../useLcuEvents";
import type { ActiveChampSelectSnapshot, ChampionBuild, ChampSelectedPayload } from "../../../types";
import { baseBuild } from "./lcuEventsFixtures";

let listeners: Record<string, (event: { payload: unknown }) => void> = {};
let updateFiltersHandler: (args?: Record<string, unknown>) => Promise<ChampionBuild>;
let lobbySlotHandler: (slotIndex: number) => Promise<ActiveChampSelectSnapshot | null>;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args?: Record<string, unknown>) => {
    if (cmd === "update_build_filters") {
      return updateFiltersHandler(args);
    }
    if (cmd === "select_lobby_preselect_slot") {
      return lobbySlotHandler(Number(args?.slotIndex ?? 0));
    }
    if (cmd === "get_active_champ_select_snapshot") {
      return Promise.resolve({ gameflow_phase: null, champ_selected: null, build: null });
    }
    return Promise.resolve(null);
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockImplementation((event: string, cb: (payload: unknown) => void) => {
    listeners[event] = cb;
    return Promise.resolve(() => {
      delete listeners[event];
    });
  }),
}));

describe("useLcuEvents concurrency, loading lifecycle, and slot guards (MAX-04)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listeners = {};
    updateFiltersHandler = () => Promise.resolve({ ...baseBuild });
    lobbySlotHandler = () => Promise.resolve(null);
  });

  it("resets rune/spell/build indices and clears loading when recommended-build arrives before update_build_filters resolves", async () => {
    let resolveFilterCmd!: (b: ChampionBuild) => void;
    updateFiltersHandler = () =>
      new Promise<ChampionBuild>((resolve) => {
        resolveFilterCmd = resolve;
      });

    const ahriBuild: ChampionBuild = {
      ...baseBuild,
      champion_id: 103,
      champion_name: "Ahri",
      active_position: "mid",
      active_tier: "emerald_plus",
      supports_runes: true,
      supports_spells: true,
      rune_pages: [
        { primary_style_id: 8100, sub_style_id: 8200, selected_perk_ids: [8112], play: 100, win: 55, pick_rate: 50 },
        { primary_style_id: 8300, sub_style_id: 8100, selected_perk_ids: [8351], play: 50, win: 25, pick_rate: 25 },
      ],
      spell_options: [
        { spell1_id: 4, spell2_id: 14, play: 100, win: 55, pick_rate: 70, spells: [] },
        { spell1_id: 4, spell2_id: 12, play: 40, win: 22, pick_rate: 20, spells: [] },
      ],
      build_options: [
        { name: "Build A", core_item_ids: [6655], final_item_ids: [6655], situational_item_ids: [], play: 100, win: 55, pick_rate: 60 },
        { name: "Build B", core_item_ids: [3157], final_item_ids: [3157], situational_item_ids: [], play: 50, win: 26, pick_rate: 30 },
      ],
    };

    const { result } = renderHook(() => useLcuEvents("D"));

    await waitFor(() => {
      expect(listeners["champ-selected"]).toBeDefined();
      expect(listeners["recommended-build"]).toBeDefined();
    });

    await act(async () => {
      listeners["gameflow-phase-changed"]({ payload: "ChampSelect" });
      listeners["champ-selected"]({
        payload: {
          champion_id: 103,
          queue_id: 420,
          queue_mode: "SummonersRift",
          locked_in: true,
          assigned_position: "mid",
        },
      });
      listeners["recommended-build"]({ payload: ahriBuild });
    });

    await act(async () => {
      await result.current.handleSelectRunePage(1);
      await result.current.handleSelectSpellPair(1);
      await result.current.handleSelectBuild(1);
    });

    expect(result.current.selectedRunePageIndex).toBe(1);
    expect(result.current.selectedSpellPairIndex).toBe(1);
    expect(result.current.selectedBuildIndex).toBe(1);

    const filteredBuild: ChampionBuild = { ...ahriBuild, active_tier: "diamond_plus" };

    let filterPromise!: Promise<void>;
    await act(async () => {
      filterPromise = result.current.handleFilterChange("tier", "diamond_plus");
    });

    expect(result.current.loading).toBe(true);
    expect(result.current.selectedRunePageIndex).toBe(0);
    expect(result.current.selectedSpellPairIndex).toBe(0);
    expect(result.current.selectedBuildIndex).toBe(0);

    // Backend emits recommended-build BEFORE returning from update_build_filters IPC
    await act(async () => {
      listeners["recommended-build"]({ payload: filteredBuild });
    });

    expect(result.current.loading).toBe(false);
    expect(result.current.build?.active_tier).toBe("diamond_plus");

    await act(async () => {
      resolveFilterCmd(filteredBuild);
      await filterPromise;
    });

    expect(result.current.loading).toBe(false);
    expect(result.current.build?.active_tier).toBe("diamond_plus");
  });

  it("ignores stale out-of-order Swiftplay slot responses and responses arriving after lobby exit", async () => {
    const resolvers: Record<number, (snap: ActiveChampSelectSnapshot | null) => void> = {};
    lobbySlotHandler = (slotIndex) =>
      new Promise<ActiveChampSelectSnapshot | null>((resolve) => {
        resolvers[slotIndex] = resolve;
      });

    const primaryPayload: ChampSelectedPayload = {
      champion_id: 103,
      queue_id: 480,
      queue_mode: "SummonersRift",
      locked_in: true,
      assigned_position: "mid",
      is_lobby_preselect: true,
      active_slot_index: 0,
    };

    const { result } = renderHook(() => useLcuEvents("D"));
    await waitFor(() => {
      expect(listeners["champ-selected"]).toBeDefined();
      expect(listeners["champ-select-ended"]).toBeDefined();
    });

    await act(async () => {
      listeners["gameflow-phase-changed"]({ payload: "Lobby" });
      listeners["champ-selected"]({ payload: primaryPayload });
      listeners["recommended-build"]({ payload: { ...baseBuild, champion_id: 103, champion_name: "Ahri" } });
    });

    let pSlot1!: Promise<void>;
    let pSlot0!: Promise<void>;
    await act(async () => {
      pSlot1 = result.current.handleSelectLobbySlot(1);
      pSlot0 = result.current.handleSelectLobbySlot(0);
    });

    // Slot 0 resolves fast first; slow Slot 1 resolves later and MUST be ignored
    await act(async () => {
      resolvers[0]({
        gameflow_phase: "Lobby",
        champ_selected: primaryPayload,
        build: { ...baseBuild, champion_id: 103, champion_name: "Ahri" },
      });
      await pSlot0;
      resolvers[1]({
        gameflow_phase: "Lobby",
        champ_selected: { ...primaryPayload, champion_id: 222, active_slot_index: 1 },
        build: { ...baseBuild, champion_id: 222, champion_name: "Jinx" },
      });
      await pSlot1;
    });

    expect(result.current.champSelect?.champion_id).toBe(103);
    expect(result.current.build?.champion_id).toBe(103);

    // Late slot response arriving after lobby exit must NOT resurrect session
    let pAfterExit!: Promise<void>;
    await act(async () => {
      pAfterExit = result.current.handleSelectLobbySlot(1);
    });
    await act(async () => {
      listeners["champ-select-ended"]({ payload: null });
      resolvers[1]({
        gameflow_phase: "Lobby",
        champ_selected: { ...primaryPayload, champion_id: 222, active_slot_index: 1 },
        build: { ...baseBuild, champion_id: 222, champion_name: "Jinx" },
      });
      await pAfterExit;
    });

    expect(result.current.champSelect).toBeNull();
    expect(result.current.build).toBeNull();
  });
});

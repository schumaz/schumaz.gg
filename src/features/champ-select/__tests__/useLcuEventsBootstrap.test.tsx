import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { useLcuEvents } from "../useLcuEvents";
import type { ActiveChampSelectSnapshot, ChampionBuild, QueueMode } from "../../../types";
import { baseBuild } from "./lcuEventsFixtures";

let listeners: Record<string, (event: { payload: unknown }) => void> = {};
let snapshotHandler: () => Promise<ActiveChampSelectSnapshot | null>;
let buildFetchHandler: (cmd: string, args?: Record<string, unknown>) => Promise<ChampionBuild>;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args?: Record<string, unknown>) => {
    if (cmd === "get_summoner_status") {
      return Promise.resolve({
        summonerId: 42,
        gameName: "Schumaz",
        tagLine: "BR1",
        displayName: "Schumaz",
        summonerLevel: 150,
        profileIconId: 10,
      });
    }
    if (cmd === "get_active_champ_select_snapshot") {
      return snapshotHandler();
    }
    if (cmd === "get_recommended_build_for_champion" || cmd === "update_build_filters") {
      return buildFetchHandler(cmd, args);
    }
    if (cmd === "sync_displayed_champion") {
      return Promise.resolve(null);
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

describe("useLcuEvents mid-ChampSelect bootstrap hydration (P11.10)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listeners = {};
    snapshotHandler = () =>
      Promise.resolve({ gameflow_phase: null, champ_selected: null, build: null });
    buildFetchHandler = () => Promise.resolve({ ...baseBuild, supports_runes: true });
  });

  const modeCases: Array<{
    label: string;
    queueId: number;
    queueMode: QueueMode;
    assignedPosition: string | null;
    supportsRunes: boolean;
    supportsSpells: boolean;
  }> = [
    {
      label: "Summoner's Rift",
      queueId: 420,
      queueMode: "SummonersRift",
      assignedPosition: "mid",
      supportsRunes: true,
      supportsSpells: true,
    },
    {
      label: "ARAM",
      queueId: 450,
      queueMode: "Aram",
      assignedPosition: null,
      supportsRunes: true,
      supportsSpells: true,
    },
    {
      label: "ARAM Mayhem",
      queueId: 2400,
      queueMode: "AramMayhem",
      assignedPosition: null,
      supportsRunes: false,
      supportsSpells: true,
    },
    {
      label: "Arena",
      queueId: 1700,
      queueMode: "Arena",
      assignedPosition: null,
      supportsRunes: false,
      supportsSpells: false,
    },
  ];

  it.each(modeCases)(
    "hydrates $label from pre-populated snapshot in 0ms with zero duplicate fetch and single Runes auto-navigation",
    async ({ queueId, queueMode, assignedPosition, supportsRunes, supportsSpells }) => {
      const cachedBuild: ChampionBuild = {
        ...baseBuild,
        champion_id: 103,
        champion_name: "Ahri",
        active_position: assignedPosition ?? "",
        supports_runes: supportsRunes,
        supports_spells: supportsSpells,
      };
      snapshotHandler = () =>
        Promise.resolve({
          gameflow_phase: "ChampSelect",
          champ_selected: {
            champion_id: 103,
            queue_id: queueId,
            queue_mode: queueMode,
            locked_in: true,
            assigned_position: assignedPosition,
          },
          build: cachedBuild,
        });

      const onChampionPulled = vi.fn();
      const { result } = renderHook(() => useLcuEvents("D", { onChampionPulled }));

      await waitFor(() => {
        expect(result.current.lcuInitialChecked).toBe(true);
        expect(result.current.build?.champion_id).toBe(103);
      });

      expect(result.current.gameflowPhase).toBe("ChampSelect");
      expect(result.current.champSelect?.queue_mode).toBe(queueMode);
      expect(onChampionPulled).toHaveBeenCalledTimes(1);

      // Must NOT issue any redundant build fetch when snapshot.build is already present
      const fetchCalls = vi
        .mocked(invoke)
        .mock.calls.filter(
          ([cmd]) => cmd === "get_recommended_build_for_champion" || cmd === "update_build_filters"
        );
      expect(fetchCalls).toHaveLength(0);

      // A late recommended-build event must not trigger duplicate auto-navigation
      await act(async () => {
        listeners["recommended-build"]({ payload: cachedBuild });
      });
      expect(onChampionPulled).toHaveBeenCalledTimes(1);
    }
  );

  it("fetches build once when snapshot has active champSelect without cached build and no WS events arrive", async () => {
    snapshotHandler = () =>
      Promise.resolve({
        gameflow_phase: "ChampSelect",
        champ_selected: {
          champion_id: 18,
          queue_id: 420,
          queue_mode: "SummonersRift",
          locked_in: true,
          assigned_position: "adc",
        },
        build: null,
      });

    const onChampionPulled = vi.fn();
    const { result } = renderHook(() => useLcuEvents("D", { onChampionPulled }));

    await waitFor(() => {
      expect(result.current.lcuInitialChecked).toBe(true);
      expect(result.current.build?.champion_id).toBe(18);
    });

    expect(result.current.gameflowPhase).toBe("ChampSelect");
    expect(onChampionPulled).toHaveBeenCalledTimes(1);

    const fetchCalls = vi
      .mocked(invoke)
      .mock.calls.filter(([cmd]) => cmd === "update_build_filters");
    expect(fetchCalls).toHaveLength(1);
  });

  it("registers listeners before requesting snapshot and avoids double fetch when on-demand probe emits champ-selected during FINALIZATION", async () => {
    let resolveBuild!: (value: ChampionBuild) => void;
    buildFetchHandler = () =>
      new Promise<ChampionBuild>((resolve) => {
        resolveBuild = resolve;
      });

    snapshotHandler = async () => {
      // Listeners MUST already be registered before snapshot IPC runs
      expect(listeners["champ-selected"]).toBeDefined();
      expect(listeners["champ-select-reconciled"]).toBeDefined();

      // Simulate on-demand backend poll emitting champ-selected while snapshot is resolving
      listeners["gameflow-phase-changed"]?.({ payload: "ChampSelect" });
      listeners["champ-selected"]?.({
        payload: {
          champion_id: 18,
          queue_id: 450,
          queue_mode: "Aram",
          locked_in: true,
          assigned_position: null,
        },
      });

      return {
        gameflow_phase: "ChampSelect",
        champ_selected: {
          champion_id: 18,
          queue_id: 450,
          queue_mode: "Aram",
          locked_in: true,
          assigned_position: null,
        },
        build: null,
      };
    };

    const onChampionPulled = vi.fn();
    const { result } = renderHook(() => useLcuEvents("D", { onChampionPulled }));

    await waitFor(() => {
      expect(result.current.lcuInitialChecked).toBe(true);
      expect(result.current.champSelect?.champion_id).toBe(18);
    });

    // Exactly ONE fetch started by onChampSelected; snapshot hydration must not start a second fetch
    const fetchCalls = vi
      .mocked(invoke)
      .mock.calls.filter(([cmd]) => cmd === "get_recommended_build_for_champion");
    expect(fetchCalls).toHaveLength(1);

    await act(async () => {
      resolveBuild({ ...baseBuild, supports_runes: true });
    });

    expect(result.current.build?.champion_id).toBe(18);
    expect(onChampionPulled).toHaveBeenCalledTimes(1);
  });
});

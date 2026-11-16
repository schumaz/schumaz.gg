import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useLcuEvents } from "../useLcuEvents";
import { ChampionBuild } from "../../../types";
import { baseBuild } from "./lcuEventsFixtures";

let listeners: Record<string, (event: { payload: unknown }) => void> = {};
let resolveTristana!: (value: ChampionBuild) => void;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args?: { championId?: number | null }) => {
    if (cmd === "get_summoner_status" || cmd === "sync_displayed_champion") {
      return Promise.resolve(null);
    }
    if (cmd === "get_recommended_build_for_champion" && args?.championId === 18) {
      return new Promise<ChampionBuild>((resolve) => {
        resolveTristana = resolve;
      });
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

describe("useLcuEvents lifecycle phases and auto-navigation", () => {
  beforeEach(() => {
    listeners = {};
  });

  it("calls onChampionPulled once per match, keeps build during InProgress, and calls onMatchEnded when match finishes", async () => {
    const onChampionPulled = vi.fn();
    const onMatchEnded = vi.fn();
    const { result } = renderHook(() =>
      useLcuEvents("D", { onChampionPulled, onMatchEnded })
    );

    await waitFor(() => {
      expect(listeners["champ-selected"]).toBeDefined();
      expect(listeners["gameflow-phase-changed"]).toBeDefined();
      expect(listeners["champ-select-ended"]).toBeDefined();
      expect(listeners["match-ended"]).toBeDefined();
    });

    await act(async () => {
      listeners["gameflow-phase-changed"]({ payload: "ChampSelect" });
      listeners["champ-selected"]({
        payload: {
          champion_id: 18,
          queue_id: 450,
          queue_mode: "Aram",
          locked_in: false,
        },
      });
      resolveTristana({ ...baseBuild, supports_runes: true });
    });

    expect(onChampionPulled).toHaveBeenCalledTimes(1);
    expect(result.current.build?.champion_id).toBe(18);

    await act(async () => {
      listeners["recommended-build"]({ payload: { ...baseBuild, supports_runes: true } });
    });
    expect(onChampionPulled).toHaveBeenCalledTimes(1);

    await act(async () => {
      listeners["gameflow-phase-changed"]({ payload: "InProgress" });
      listeners["champ-select-ended"]({ payload: null });
    });

    expect(result.current.gameflowPhase).toBe("InProgress");
    expect(result.current.build?.champion_id).toBe(18);
    expect(onMatchEnded).not.toHaveBeenCalled();

    await act(async () => {
      listeners["gameflow-phase-changed"]({ payload: "EndOfGame" });
      listeners["match-ended"]({ payload: null });
    });

    expect(result.current.build).toBeNull();
    expect(result.current.champSelect).toBeNull();
    expect(onMatchEnded).toHaveBeenCalledTimes(1);

    await act(async () => {
      listeners["gameflow-phase-changed"]({ payload: "ChampSelect" });
      listeners["recommended-build"]({
        payload: { ...baseBuild, champion_id: 103, champion_name: "Ahri" },
      });
    });

    expect(onChampionPulled).toHaveBeenCalledTimes(2);
  });

  it("clears build on champ-select-ended when champ select ends via dodge (not in-game)", async () => {
    const onChampionPulled = vi.fn();
    const onMatchEnded = vi.fn();
    const { result } = renderHook(() =>
      useLcuEvents("D", { onChampionPulled, onMatchEnded })
    );

    await waitFor(() => {
      expect(listeners["champ-selected"]).toBeDefined();
      expect(listeners["gameflow-phase-changed"]).toBeDefined();
      expect(listeners["champ-select-ended"]).toBeDefined();
    });

    await act(async () => {
      listeners["gameflow-phase-changed"]({ payload: "ChampSelect" });
      listeners["champ-selected"]({
        payload: {
          champion_id: 18,
          queue_id: 450,
          queue_mode: "Aram",
          locked_in: false,
        },
      });
      resolveTristana({ ...baseBuild, supports_runes: true });
    });

    expect(onChampionPulled).toHaveBeenCalledTimes(1);
    expect(result.current.build?.champion_id).toBe(18);

    await act(async () => {
      listeners["gameflow-phase-changed"]({ payload: "Lobby" });
      listeners["champ-select-ended"]({ payload: null });
    });

    expect(result.current.build).toBeNull();
    expect(result.current.champSelect).toBeNull();
    expect(onMatchEnded).not.toHaveBeenCalled();

    await act(async () => {
      listeners["recommended-build"]({
        payload: { ...baseBuild, champion_id: 103, champion_name: "Ahri" },
      });
    });
    expect(onChampionPulled).toHaveBeenCalledTimes(2);
  });
});

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useLcuEvents } from "../features/champ-select/useLcuEvents";
import { ChampionBuild } from "../types";
import { baseBuild } from "../features/champ-select/__tests__/lcuEventsFixtures";

let listeners: Record<string, (event: { payload: unknown }) => void> = {};
let resolveTristana!: (value: ChampionBuild) => void;
let resolveAhri!: (value: ChampionBuild) => void;
let syncedDisplayedChampions: Array<number | null> = [];

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args?: { championId?: number | null }) => {
    if (cmd === "get_summoner_status") {
      return Promise.resolve(null);
    }
    if (cmd === "sync_displayed_champion") {
      syncedDisplayedChampions.push(args?.championId ?? null);
      return Promise.resolve(null);
    }
    if (cmd === "get_recommended_build_for_champion") {
      if (args?.championId === 18) {
        return new Promise<ChampionBuild>((resolve) => {
          resolveTristana = resolve;
        });
      }
      if (args?.championId === 103) {
        return new Promise<ChampionBuild>((resolve) => {
          resolveAhri = resolve;
        });
      }
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

describe("useLcuEvents rapid champion switch, finalization reconciliation and ARAM spells", () => {
  beforeEach(() => {
    listeners = {};
    syncedDisplayedChampions = [];
  });

  it("ignores stale build responses and late recommended-build events after rapid champion switching", async () => {
    const { result } = renderHook(() => useLcuEvents("D"));

    await waitFor(() => {
      expect(listeners["champ-selected"]).toBeDefined();
      expect(listeners["recommended-build"]).toBeDefined();
    });

    act(() => {
      listeners["champ-selected"]({
        payload: {
          champion_id: 18,
          queue_id: 2400,
          queue_mode: "AramMayhem",
          locked_in: true,
        },
      });
      listeners["champ-selected"]({
        payload: {
          champion_id: 103,
          queue_id: 450,
          queue_mode: "Aram",
          locked_in: true,
        },
      });
    });

    const ahriBuild: ChampionBuild = {
      ...baseBuild,
      champion_id: 103,
      champion_name: "Ahri",
      spell1_id: 4,
      spell2_id: 32,
      spells: [
        { id: 4, name: "Flash", icon_url: "flash.png" },
        { id: 32, name: "Marca", icon_url: "snowball.png" },
      ],
      spell_options: [
        {
          spell1_id: 4,
          spell2_id: 32,
          play: 500,
          win: 260,
          pick_rate: 70.0,
          spells: [
            { id: 4, name: "Flash", icon_url: "flash.png" },
            { id: 32, name: "Marca", icon_url: "snowball.png" },
          ],
        },
        {
          spell1_id: 4,
          spell2_id: 13,
          play: 150,
          win: 75,
          pick_rate: 20.0,
          spells: [
            { id: 4, name: "Flash", icon_url: "flash.png" },
            { id: 13, name: "Clareza", icon_url: "clarity.png" },
          ],
        },
      ],
      supports_runes: true,
    };

    await act(async () => {
      resolveAhri(ahriBuild);
    });
    expect(result.current.build?.champion_id).toBe(103);
    expect(result.current.orderedSpells.map((s) => s.name)).toEqual(["Flash", "Marca"]);

    await act(async () => {
      resolveTristana(baseBuild);
      listeners["recommended-build"]({ payload: baseBuild });
    });

    expect(result.current.build?.champion_id).toBe(103);
    expect(result.current.build?.champion_name).toBe("Ahri");
    expect(syncedDisplayedChampions).toContain(103);
  });

  it("synchronizes UI and clears old selections when final reconciliation detects champion divergence", async () => {
    const { result } = renderHook(() => useLcuEvents("D"));

    await waitFor(() => {
      expect(listeners["champ-selected"]).toBeDefined();
      expect(listeners["champ-select-reconciled"]).toBeDefined();
    });

    await act(async () => {
      listeners["champ-selected"]({
        payload: {
          champion_id: 18,
          queue_id: 450,
          queue_mode: "Aram",
          locked_in: true,
        },
      });
      resolveTristana({ ...baseBuild, supports_runes: true });
    });

    await act(async () => {
      await result.current.handleSelectSpellPair(1);
      await result.current.handleSelectRunePage(1);
      await result.current.handleSelectBuild(1);
      result.current.setSpellsInverted(true);
    });

    expect(result.current.selectedSpellPairIndex).toBe(1);
    expect(result.current.selectedRunePageIndex).toBe(1);
    expect(result.current.selectedBuildIndex).toBe(1);
    expect(result.current.spellsInverted).toBe(true);

    const ahriBuild: ChampionBuild = {
      ...baseBuild,
      champion_id: 103,
      champion_name: "Ahri",
      spell1_id: 4,
      spell2_id: 32,
      spells: [
        { id: 4, name: "Flash", icon_url: "flash.png" },
        { id: 32, name: "Marca", icon_url: "snowball.png" },
      ],
      supports_runes: true,
    };

    await act(async () => {
      listeners["champ-select-reconciled"]({
        payload: {
          champ_selected: {
            champion_id: 103,
            queue_id: 450,
            queue_mode: "Aram",
            locked_in: true,
          },
          build: ahriBuild,
          champion_diverged: true,
        },
      });
    });

    expect(result.current.champSelect?.champion_id).toBe(103);
    expect(result.current.build?.champion_id).toBe(103);
    expect(result.current.selectedSpellPairIndex).toBe(0);
    expect(result.current.selectedRunePageIndex).toBe(0);
    expect(result.current.selectedBuildIndex).toBe(0);
    expect(result.current.spellsInverted).toBe(false);
    expect(syncedDisplayedChampions).toContain(103);
  });

  it("preserves manual selections on final reconciliation when champion matches in ARAM Mayhem", async () => {
    const { result } = renderHook(() => useLcuEvents("D"));

    await waitFor(() => {
      expect(listeners["champ-selected"]).toBeDefined();
      expect(listeners["champ-select-reconciled"]).toBeDefined();
    });

    await act(async () => {
      listeners["champ-selected"]({
        payload: {
          champion_id: 18,
          queue_id: 2400,
          queue_mode: "AramMayhem",
          locked_in: true,
        },
      });
      resolveTristana({ ...baseBuild, supports_runes: false });
    });

    await act(async () => {
      await result.current.handleSelectSpellPair(1);
      await result.current.handleSelectBuild(1);
      result.current.setSpellsInverted(true);
    });

    await act(async () => {
      listeners["champ-select-reconciled"]({
        payload: {
          champ_selected: {
            champion_id: 18,
            queue_id: 2400,
            queue_mode: "AramMayhem",
            locked_in: true,
          },
          build: { ...baseBuild, supports_runes: false },
          champion_diverged: false,
        },
      });
    });

    expect(result.current.champSelect?.champion_id).toBe(18);
    expect(result.current.build?.supports_runes).toBe(false);
    expect(result.current.selectedSpellPairIndex).toBe(1);
    expect(result.current.selectedBuildIndex).toBe(1);
    expect(result.current.spellsInverted).toBe(true);
  });
});

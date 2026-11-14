import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useLcuEvents } from "../hooks/useLcuEvents";
import { ChampionBuild } from "../types";

const baseBuild: ChampionBuild = {
  champion_id: 18,
  champion_name: "Tristana",
  win_rate: 53.0,
  pick_rate: 8.5,
  primary_style_id: 8000,
  sub_style_id: 8100,
  selected_perk_ids: [],
  spell1_id: 4,
  spell2_id: 7,
  champion_icon_url: "tristana.png",
  primary_style_name: "Precisão",
  primary_style_icon_url: "precision.png",
  sub_style_name: "Dominação",
  sub_style_icon_url: "domination.png",
  perks: [],
  rune_pages: [],
  spell_choices: [
    { id: 7, name: "Curar", icon_url: "heal.png" },
    { id: 21, name: "Barreira", icon_url: "barrier.png" },
  ],
  spells: [
    { id: 4, name: "Flash", icon_url: "flash.png" },
    { id: 7, name: "Curar", icon_url: "heal.png" },
  ],
  build_options: [],
  core_build_options: [],
  final_build_options: [],
  item_blocks: [],
  active_position: "",
  active_tier: "emerald_plus",
  active_region: "global",
  active_patch: "16.22",
  available_patches: ["16.22"],
  patch_fallback_applied: false,
  supports_runes: false,
};

let listeners: Record<string, (event: { payload: unknown }) => void> = {};
let resolveTristana!: (value: ChampionBuild) => void;
let resolveAhri!: (value: ChampionBuild) => void;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args?: { championId?: number }) => {
    if (cmd === "get_summoner_status") {
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

describe("useLcuEvents rapid champion switch and ARAM spells", () => {
  beforeEach(() => {
    listeners = {};
  });

  it("ignores stale build responses and late recommended-build events after rapid champion switching", async () => {
    const { result } = renderHook(() => useLcuEvents("D"));

    await waitFor(() => {
      expect(listeners["champ-selected"]).toBeDefined();
      expect(listeners["recommended-build"]).toBeDefined();
    });

    // 1. Rapidly select Tristana (18) then switch to Ahri (103)
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
      spell_choices: [
        { id: 32, name: "Marca", icon_url: "snowball.png" },
        { id: 13, name: "Clareza", icon_url: "clarity.png" },
      ],
      supports_runes: true,
    };

    // 2. Ahri resolves first
    await act(async () => {
      resolveAhri(ahriBuild);
    });
    expect(result.current.build?.champion_id).toBe(103);
    expect(result.current.orderedSpells.map((s) => s.name)).toEqual(["Flash", "Marca"]);

    // 3. Stale Tristana command response AND late backend recommended-build event arrive afterwards
    await act(async () => {
      resolveTristana(baseBuild);
      listeners["recommended-build"]({ payload: baseBuild });
    });

    // Must remain on Ahri (103), never reverted to Tristana (18)
    expect(result.current.build?.champion_id).toBe(103);
    expect(result.current.build?.champion_name).toBe("Ahri");
  });
});

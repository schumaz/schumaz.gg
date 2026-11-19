import type { ArenaChampionEntry, ArenaTierListResponse } from "../../../types";
import { mockArenaMasterYiBuild } from "../../champ-select/__tests__/arenaBuildFixture";

export { mockArenaMasterYiBuild };

export const mockArenaChampions: ArenaChampionEntry[] = [
  {
    id: 11,
    key: "masteryi",
    name: "Master Yi",
    icon_url: "masteryi.png",
    tier: 0,
    rank: 1,
    win_rate: 56.4,
    first_place_rate: 16.8,
    avg_placement: 3.12,
    pick_rate: 7.2,
    ban_rate: 14.3,
    play: 21000,
  },
  {
    id: 106,
    key: "volibear",
    name: "Volibear",
    icon_url: "volibear.png",
    tier: 0,
    rank: 2,
    win_rate: 55.1,
    first_place_rate: 15.4,
    avg_placement: 3.24,
    pick_rate: 5.9,
    ban_rate: 9.1,
    play: 17500,
  },
  {
    id: 117,
    key: "lulu",
    name: "Lulu",
    icon_url: "lulu.png",
    tier: 1,
    rank: 3,
    win_rate: 53.2,
    first_place_rate: 13.7,
    avg_placement: 3.45,
    pick_rate: 4.8,
    ban_rate: 6.2,
    play: 14200,
  },
  {
    id: 350,
    key: "yuumi",
    name: "Yuumi",
    icon_url: "yuumi.png",
    tier: 2,
    rank: 12,
    win_rate: 49.8,
    first_place_rate: 11.2,
    avg_placement: 3.92,
    pick_rate: 4.1,
    ban_rate: 18.5,
    play: 12000,
  },
];

export const mockArenaTierResponse: ArenaTierListResponse = {
  active_patch: "16.22",
  available_patches: ["16.22", "16.21"],
  patch_fallback_applied: false,
  patch_protection_remaining_hours: null,
  champions: mockArenaChampions,
};

export const mockProtectedArenaTierResponse: ArenaTierListResponse = {
  active_patch: "16.21",
  available_patches: ["16.22", "16.21"],
  patch_fallback_applied: true,
  patch_protection_remaining_hours: 34,
  champions: mockArenaChampions,
};

export const mockArenaLuluBuild = {
  ...mockArenaMasterYiBuild,
  champion_id: 117,
  champion_name: "Lulu",
  champion_icon_url: "lulu.png",
  win_rate: 53.2,
  first_place_rate: 13.7,
  pick_rate: 4.8,
};

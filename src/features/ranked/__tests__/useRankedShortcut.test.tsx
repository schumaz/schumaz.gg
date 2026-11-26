import { describe, it, expect, vi, beforeEach } from "vitest";
import { act, fireEvent, render, renderHook, screen, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import App from "../../../App";
import { defaultAppInvokeMock, mockAhriBuild, resetMockAppSettings } from "../../../__tests__/appTestFixtures";
import { rankedIpc } from "../../../services/ipc";
import { useRanked } from "../useRanked";
import { mockTierResponse } from "./rankedFixtures";

let eventListeners: Record<string, (event: { payload: unknown }) => void> = {};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args?: Record<string, unknown>) =>
    defaultAppInvokeMock(cmd, args)
  ),
  convertFileSrc: vi.fn((p: string, proto = "asset") => `${proto}://localhost/${p}`),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockImplementation((event: string, cb: (payload: unknown) => void) => {
    eventListeners[event] = cb;
    return Promise.resolve(() => {
      delete eventListeners[event];
    });
  }),
}));

describe("useRanked.openChampionShortcut & Profile -> Ranked navigation", () => {
  beforeEach(() => {
    eventListeners = {};
    localStorage.clear();
    resetMockAppSettings();
    vi.restoreAllMocks();
    vi.mocked(invoke).mockImplementation((cmd: string, args?: unknown) =>
      defaultAppInvokeMock(cmd, args as Record<string, unknown> | undefined)
    );
  });

  it("opens synthetic entry with position=null when tierData is unloaded and syncs resolved active_position", async () => {
    const buildSpy = vi.spyOn(rankedIpc, "getRankedChampionBuild").mockResolvedValue({
      ...mockAhriBuild,
      champion_id: 64,
      champion_name: "Lee Sin",
      active_position: "jungle",
    });

    const { result } = renderHook(() => useRanked(false, "D", true, "pt_BR"));
    expect(result.current.tierData).toBeNull();

    act(() => {
      result.current.openChampionShortcut({
        championId: 64,
        championName: "Lee Sin",
        championIconUrl: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/LeeSin.png",
      });
    });

    await waitFor(() => {
      expect(buildSpy).toHaveBeenCalledWith(64, null, "emerald_plus", "global", null);
    });
    await waitFor(() => {
      expect(result.current.championBuild?.champion_id).toBe(64);
      expect(result.current.selectedChampion?.lane).toBe("jungle");
    });
  });

  it("picks primary lane from tierData when already loaded", async () => {
    vi.spyOn(rankedIpc, "getRankedTierList").mockResolvedValue(mockTierResponse);
    const buildSpy = vi.spyOn(rankedIpc, "getRankedChampionBuild").mockResolvedValue({
      ...mockAhriBuild,
      champion_id: 245,
      champion_name: "Ekko",
      active_position: "mid",
    });

    const { result } = renderHook(() => useRanked(true, "D", true, "pt_BR"));
    await waitFor(() => expect(result.current.tierData).toBeTruthy());

    act(() => {
      result.current.openChampionShortcut({
        championId: 245,
        championName: "Ekko",
        championIconUrl: "https://example.com/ekko.png",
      });
    });

    await waitFor(() => {
      expect(buildSpy).toHaveBeenCalledWith(245, "mid", "emerald_plus", "global", null);
    });
  });

  it("switches from Profile mastery modal straight into Ranked champion detail without prior tierData", async () => {
    const leeBuild = {
      ...mockAhriBuild,
      champion_id: 64,
      champion_name: "Lee Sin",
      active_position: "jungle",
    };

    vi.mocked(invoke).mockImplementation((cmd: string, args?: unknown) => {
      if (cmd === "get_summoner_status") {
        return Promise.resolve({
          summonerId: 987654321,
          gameName: "Schuma",
          tagLine: "BR1",
          displayName: "Schuma",
          summonerLevel: 350,
          profileIconId: 548,
        });
      }
      if (cmd === "get_cached_player_profile" || cmd === "get_player_profile") {
        const profile = {
          summoner: {
            summonerId: 987654321,
            gameName: "Schuma",
            tagLine: "BR1",
            displayName: "Schuma",
            summonerLevel: 350,
            profileIconId: 548,
          },
          profile_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/profileicon/548.png",
          background_skin_id: 0,
          background_skin_name: "",
          background_splash_url: "",
          background_splash_fallback_url: "",
          ranked_stats: [],
          top_masteries: [
            {
              champion_id: 64,
              champion_name: "Lee Sin",
              champion_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/LeeSin.png",
              champion_level: 15,
              champion_points: 180000,
            },
          ],
          all_masteries: [
            {
              champion_id: 64,
              champion_name: "Lee Sin",
              champion_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/LeeSin.png",
              champion_level: 15,
              champion_points: 180000,
              last_play_time_ms: 0,
              champion_points_since_last_level: 0,
              champion_points_until_next_level: 0,
              tokens_earned: 0,
              mark_required_for_next_level: 0,
            },
          ],
          recent_matches: [],
        };
        return Promise.resolve(
          cmd === "get_cached_player_profile"
            ? { profile, createdAtUnix: 1700000000, isStale: false }
            : profile
        );
      }
      if (cmd === "get_ranked_champion_build") return Promise.resolve(leeBuild);
      if (cmd === "get_ranked_tier_list") return Promise.resolve(mockTierResponse);
      return defaultAppInvokeMock(cmd, args as Record<string, unknown>);
    });

    render(<App />);

    await waitFor(() => expect(screen.getByTestId("view-all-masteries-btn")).toBeInTheDocument());
    fireEvent.click(screen.getByTestId("view-all-masteries-btn"));
    expect(screen.getByTestId("profile-masteries-modal")).toBeInTheDocument();

    await act(async () => {
      fireEvent.click(screen.getByTestId("mastery-catalog-item-64"));
    });

    await waitFor(() => {
      expect(screen.queryByTestId("profile-masteries-modal")).not.toBeInTheDocument();
      expect(screen.getByTestId("view-ranked")).toHaveClass("active-tab");
      expect(screen.getByTestId("ranked-champion-detail")).toBeInTheDocument();
    });
    expect(invoke).toHaveBeenCalledWith(
      "get_ranked_champion_build",
      expect.objectContaining({ championId: 64, position: null })
    );
  });
});

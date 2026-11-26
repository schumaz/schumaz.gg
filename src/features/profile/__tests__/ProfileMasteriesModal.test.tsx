import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { I18nProvider } from "../../../i18n";
import { ChampionMasteryInfo, MatchScoreboardDetail, PlayerProfile } from "../../../types";
import { ProfileTab } from "../ProfileTab";
import { mockProfile, mockSummoner } from "./profileFixtures";
import { make5v5ScoreboardDetail, makeScoreboardParticipant } from "./scoreboardFixtures";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((p: string, proto = "asset") => `${proto}://localhost/${p}`),
}));

const catalogMasteries: ChampionMasteryInfo[] = [
  {
    champion_id: 103,
    champion_name: "Ahri",
    champion_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Ahri.png",
    champion_level: 25,
    champion_points: 452310,
    last_play_time_ms: Date.now() - 120 * 60_000,
    champion_points_since_last_level: 15000,
    champion_points_until_next_level: 5000,
    tokens_earned: 2,
    mark_required_for_next_level: 3,
  },
  {
    champion_id: 238,
    champion_name: "Zed",
    champion_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Zed.png",
    champion_level: 18,
    champion_points: 310400,
    last_play_time_ms: Date.now() - 48 * 3600_000,
    champion_points_since_last_level: 0,
    champion_points_until_next_level: 0,
    tokens_earned: 0,
    mark_required_for_next_level: 0,
  },
  {
    champion_id: 157,
    champion_name: "Yasuo",
    champion_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Yasuo.png",
    champion_level: 12,
    champion_points: 185000,
    last_play_time_ms: Date.now() - 72 * 3600_000,
    champion_points_since_last_level: 4000,
    champion_points_until_next_level: 4000,
    tokens_earned: 1,
    mark_required_for_next_level: 2,
  },
  {
    champion_id: 64,
    champion_name: "Lee Sin",
    champion_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/LeeSin.png",
    champion_level: 30,
    champion_points: 95000,
    last_play_time_ms: Date.now() - 15 * 60_000,
    champion_points_since_last_level: 1000,
    champion_points_until_next_level: 9000,
    tokens_earned: 0,
    mark_required_for_next_level: 0,
  },
];

const profileWithCatalog: PlayerProfile = {
  ...mockProfile,
  top_masteries: catalogMasteries.slice(0, 3),
  all_masteries: catalogMasteries,
};

describe("P14.1 Task 4 — Mastery Catalog Modal & Ranked Navigation Shortcuts", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("opens full mastery catalog via document.body portal, filters, sorts, dismisses, and navigates to Ranked", () => {
    const onOpenChampionBuild = vi.fn();
    render(
      <I18nProvider locale="pt_BR">
        <ProfileTab
          summoner={mockSummoner}
          profile={profileWithCatalog}
          loading={false}
          errorMsg={null}
          onRefresh={vi.fn()}
          onOpenChampionBuild={onOpenChampionBuild}
        />
      </I18nProvider>
    );

    const viewAllBtn = screen.getByTestId("view-all-masteries-btn");
    expect(viewAllBtn).toHaveTextContent("Ver todas (4)");
    fireEvent.click(viewAllBtn);

    const modal = screen.getByTestId("profile-masteries-modal");
    expect(modal).toHaveAttribute("role", "dialog");
    expect(modal).toHaveAttribute("aria-modal", "true");
    expect(screen.getByTestId("view-profile")).not.toContainElement(modal);
    expect(document.body).toContainElement(modal);

    expect(screen.getByTestId("masteries-total-level")).toHaveTextContent("85");
    expect(screen.getByTestId("masteries-total-points")).toHaveTextContent(/1[.,]042[.,]710/);
    expect(screen.getByTestId("masteries-total-champions")).toHaveTextContent("4");
    expect(modal).toHaveTextContent("2/3 marcas");
    expect(screen.getByTestId("mastery-progress-103")).toBeInTheDocument();
    expect(modal.textContent).not.toMatch(/LeagueClientUx|OP\.GG/i);

    const getOrder = () =>
      Array.from(screen.getByTestId("masteries-catalog-list").querySelectorAll("[data-testid^='mastery-catalog-item-']"))
        .map((el) => Number(el.getAttribute("data-testid")!.replace("mastery-catalog-item-", "")));

    expect(getOrder()).toEqual([103, 238, 157, 64]);
    fireEvent.click(screen.getByTestId("masteries-sort-level"));
    expect(getOrder()).toEqual([64, 103, 238, 157]);
    fireEvent.click(screen.getByTestId("masteries-sort-recent"));
    expect(getOrder()).toEqual([64, 103, 238, 157]);
    fireEvent.click(screen.getByTestId("masteries-sort-name"));
    expect(getOrder()).toEqual([103, 64, 157, 238]);

    const searchInput = screen.getByTestId("masteries-search-input").querySelector("input")!;
    fireEvent.change(searchInput, { target: { value: "áhrí" } });
    expect(getOrder()).toEqual([103]);
    fireEvent.change(searchInput, { target: { value: "zzzzz" } });
    expect(screen.getByTestId("masteries-empty-state")).toBeInTheDocument();
    fireEvent.change(searchInput, { target: { value: "" } });

    fireEvent.click(screen.getByTestId("mastery-catalog-item-64"));
    expect(onOpenChampionBuild).toHaveBeenCalledWith({
      championId: 64,
      championName: "Lee Sin",
      championIconUrl: catalogMasteries[3].champion_icon_url,
    });
    expect(screen.queryByTestId("profile-masteries-modal")).not.toBeInTheDocument();

    fireEvent.click(viewAllBtn);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByTestId("profile-masteries-modal")).not.toBeInTheDocument();

    fireEvent.click(viewAllBtn);
    fireEvent.click(screen.getByTestId("profile-masteries-backdrop"));
    expect(screen.queryByTestId("profile-masteries-modal")).not.toBeInTheDocument();

    fireEvent.click(viewAllBtn);
    fireEvent.click(screen.getByTestId("profile-masteries-close-btn"));
    expect(screen.queryByTestId("profile-masteries-modal")).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId("mastery-item-1"));
    expect(onOpenChampionBuild).toHaveBeenCalledWith({
      championId: 103,
      championName: "Ahri",
      championIconUrl: catalogMasteries[0].champion_icon_url,
    });
  });

  it("navigates to Ranked build when clicking champions inside 5v5 and Arena scoreboards", async () => {
    const onOpenChampionBuild = vi.fn();
    const arenaDetail: MatchScoreboardDetail = {
      ...make5v5ScoreboardDetail(2500),
      queue_id: 1700,
      game_mode_key: "arena",
      is_arena: true,
      teams: [],
      arena_teams: [
        {
          subteam_id: 1,
          placement: 1,
          win: true,
          total_kills: 10,
          total_deaths: 2,
          total_assists: 5,
          total_damage: 45000,
          participants: [
            makeScoreboardParticipant({
              participant_id: 3,
              champion_id: 238,
              champion_name: "Zed",
              champion_icon_url: catalogMasteries[1].champion_icon_url,
            }),
          ],
        },
      ],
    };

    const profileWithArena: PlayerProfile = {
      ...profileWithCatalog,
      recent_matches: [
        profileWithCatalog.recent_matches[0],
        { ...profileWithCatalog.recent_matches[0], game_id: 2500, queue_id: 1700, game_mode_key: "arena" },
      ],
    };

    const base5v5 = make5v5ScoreboardDetail(1000);
    const detail5v5: MatchScoreboardDetail = {
      ...base5v5,
      teams: (base5v5.teams ?? []).map((team) => ({
        ...team,
        participants: team.participants.map((p) =>
          p.participant_id === 6
            ? { ...p, champion_icon_url: catalogMasteries[1].champion_icon_url }
            : p
        ),
      })),
    };

    vi.mocked(invoke)
      .mockResolvedValueOnce(detail5v5)
      .mockResolvedValueOnce(arenaDetail);

    render(
      <I18nProvider locale="pt_BR">
        <ProfileTab
          summoner={mockSummoner}
          profile={profileWithArena}
          loading={false}
          errorMsg={null}
          onRefresh={vi.fn()}
          onOpenChampionBuild={onOpenChampionBuild}
        />
      </I18nProvider>
    );

    fireEvent.click(screen.getByTestId("match-item-1000").querySelector("button.match-row-btn")!);
    await waitFor(() => expect(screen.getByTestId("scoreboard-champ-btn-6")).toBeInTheDocument());
    fireEvent.click(screen.getByTestId("scoreboard-champ-btn-6"));
    expect(onOpenChampionBuild).toHaveBeenCalledWith({
      championId: 238,
      championName: "Zed",
      championIconUrl: catalogMasteries[1].champion_icon_url,
    });

    fireEvent.click(screen.getByTestId("scoreboard-player-6").querySelector("button.sb-champ-name-btn")!);
    expect(onOpenChampionBuild).toHaveBeenCalledTimes(2);

    fireEvent.click(screen.getByTestId("match-item-2500").querySelector("button.match-row-btn")!);
    await waitFor(() => expect(screen.getByTestId("scoreboard-champ-btn-3")).toBeInTheDocument());
    fireEvent.click(screen.getByTestId("scoreboard-champ-btn-3"));
    expect(onOpenChampionBuild).toHaveBeenLastCalledWith({
      championId: 238,
      championName: "Zed",
      championIconUrl: catalogMasteries[1].champion_icon_url,
    });
  });
});

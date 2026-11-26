import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { I18nProvider } from "../../../i18n";
import { MatchScoreboardDetail, PlayerProfile } from "../../../types";
import { ProfileTab } from "../ProfileTab";
import { mockProfile, mockSummoner } from "./profileFixtures";
import { make5v5ScoreboardDetail, makeScoreboardParticipant } from "./scoreboardFixtures";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((p: string, proto = "asset") => `${proto}://localhost/${p}`),
}));

describe("P14.1 Task 3 — On-Demand Match Scoreboard Drawer", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("loads 5v5 scoreboard with skeleton, deduplicates concurrent requests, caches in RAM, and resets on refresh/switch", async () => {
    let resolveDetail!: (val: MatchScoreboardDetail) => void;
    const pending = new Promise<MatchScoreboardDetail>((res) => {
      resolveDetail = res;
    });
    vi.mocked(invoke).mockImplementationOnce(() => pending);

    const onRefresh = vi.fn();
    const { rerender } = render(
      <I18nProvider locale="pt_BR">
        <ProfileTab summoner={mockSummoner} profile={mockProfile} loading={false} errorMsg={null} onRefresh={onRefresh} />
      </I18nProvider>
    );

    const rowBtn = screen.getByTestId("match-item-1000").querySelector("button.match-row-btn")!;
    fireEvent.click(rowBtn);
    expect(screen.getByTestId("scoreboard-skeleton-1000")).toBeInTheDocument();
    expect(screen.getByTestId("match-details-1000")).toHaveTextContent("29,8k");

    // Collapse & re-expand before resolution -> concurrent call deduplicated
    fireEvent.click(rowBtn);
    fireEvent.click(rowBtn);
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith("get_match_scoreboard_detail", { gameId: 1000 });

    resolveDetail(make5v5ScoreboardDetail(1000));
    await waitFor(() => expect(screen.getByTestId("scoreboard-team-100")).toBeInTheDocument());
    expect(screen.getByTestId("scoreboard-team-200")).toHaveTextContent("Zed Bot");
    expect(screen.getByTestId("scoreboard-player-1")).toHaveClass("is-local-player");
    expect(screen.getByTestId("scoreboard-player-1")).toHaveTextContent("Schuma#BR1");
    expect(screen.getByTestId("scoreboard-player-1")).toHaveTextContent("75% KP");
    expect(screen.getByTestId("scoreboard-dmg-bar-1")).toHaveStyle({ width: "100%" });
    expect(screen.getByTestId("scoreboard-dmg-bar-6")).toHaveStyle({ width: "50%" });

    // Rich tooltip inside scoreboard (trinket item6)
    const wardImg = screen.getByTestId("scoreboard-player-1").querySelector("img[alt='Sentinela Invisível']")!;
    fireEvent.mouseEnter(wardImg.closest(".ui-tooltip-trigger")!);
    expect(screen.getByRole("tooltip")).toHaveTextContent("Sentinela Invisível");
    fireEvent.mouseLeave(wardImg.closest(".ui-tooltip-trigger")!);

    // Collapse & re-expand -> served from RAM cache with 0 new IPC calls
    fireEvent.click(rowBtn);
    fireEvent.click(rowBtn);
    expect(screen.getByTestId("scoreboard-team-100")).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledTimes(1);

    // Refresh resets drawer and RAM cache
    vi.mocked(invoke).mockResolvedValueOnce(make5v5ScoreboardDetail(1000));
    fireEvent.click(screen.getByTestId("refresh-profile-btn"));
    expect(onRefresh).toHaveBeenCalledTimes(1);
    expect(screen.queryByTestId("match-details-1000")).not.toBeInTheDocument();

    fireEvent.click(rowBtn);
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));

    // Summoner switch closes drawer & clears cache
    const otherSummoner = { ...mockSummoner, summonerId: 777777, gameName: "Other" };
    rerender(
      <I18nProvider locale="pt_BR">
        <ProfileTab
          summoner={otherSummoner}
          profile={{ ...mockProfile, summoner: otherSummoner }}
          loading={false}
          errorMsg={null}
          onRefresh={onRefresh}
        />
      </I18nProvider>
    );
    expect(screen.queryByTestId("match-details-1000")).not.toBeInTheDocument();
  });

  it("renders Arena duo ranking (1st-8th), proportional damage bars, and Augment tooltips", async () => {
    const arenaProfile: PlayerProfile = {
      ...mockProfile,
      recent_matches: [
        {
          ...mockProfile.recent_matches[0],
          game_id: 2500,
          queue_id: 1700,
          game_mode_key: "arena",
          game_mode_label: "Arena",
          arena_placement: 1,
          augment_ids: [501],
        },
      ],
    };
    const arenaDetail: MatchScoreboardDetail = {
      ...make5v5ScoreboardDetail(2500),
      queue_id: 1700,
      map_id: 30,
      game_mode_key: "arena",
      game_mode_label: "Arena",
      is_arena: true,
      teams: [],
      arena_teams: [
        {
          subteam_id: 1,
          placement: 1,
          win: true,
          total_kills: 12,
          total_deaths: 3,
          total_assists: 8,
          total_damage: 55000,
          participants: [
            makeScoreboardParticipant({
              participant_id: 3,
              is_local_player: true,
              arena_placement: 1,
              augment_ids: [501],
              total_damage_dealt_to_champions: 40000,
            }),
          ],
        },
        {
          subteam_id: 4,
          placement: 4,
          win: true,
          total_kills: 6,
          total_deaths: 7,
          total_assists: 4,
          total_damage: 25000,
          participants: [
            makeScoreboardParticipant({
              participant_id: 4,
              riot_id_name: "Duo4",
              arena_placement: 4,
              augment_ids: [],
              total_damage_dealt_to_champions: 20000,
            }),
          ],
        },
      ],
      augment_metadata: {
        501: {
          id: 501,
          name: "Eureka",
          desc: "Aceleração extra.",
          icon_url:
            "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/cherry/augments/icons/eureka_small.png",
          rarity: 8,
          ability_slot: null,
        },
      },
    };
    vi.mocked(invoke).mockResolvedValueOnce(arenaDetail);

    render(
      <I18nProvider locale="pt_BR">
        <ProfileTab summoner={mockSummoner} profile={arenaProfile} loading={false} errorMsg={null} onRefresh={vi.fn()} />
      </I18nProvider>
    );

    fireEvent.click(screen.getByTestId("match-item-2500").querySelector("button.match-row-btn")!);
    await waitFor(() => expect(screen.getByTestId("scoreboard-arena-team-1")).toBeInTheDocument());
    expect(screen.getByTestId("scoreboard-arena-team-1")).toHaveTextContent("1º");
    expect(screen.getByTestId("scoreboard-arena-team-4")).toHaveTextContent("4º");

    const augImg = screen.getByTestId("scoreboard-arena-team-1").querySelector("img[alt='Eureka']")!;
    fireEvent.mouseEnter(augImg.closest(".ui-tooltip-trigger")!);
    expect(screen.getByRole("tooltip")).toHaveTextContent("Aceleração extra.");
  });

  it("keeps saved summary visible with offline notice and supports error retry without leaking client paths", async () => {
    const { rerender } = render(
      <I18nProvider locale="pt_BR">
        <ProfileTab summoner={null} profile={mockProfile} loading={false} errorMsg={null} onRefresh={vi.fn()} />
      </I18nProvider>
    );

    fireEvent.click(screen.getByTestId("match-item-1000").querySelector("button.match-row-btn")!);
    expect(invoke).not.toHaveBeenCalled();
    expect(screen.getByTestId("match-details-1000")).toHaveTextContent("29,8k");
    const offlineNotice = screen.getByTestId("scoreboard-fallback-notice-1000");
    expect(offlineNotice).toHaveTextContent("Conecte o cliente do LoL");
    expect(offlineNotice.textContent).not.toMatch(/LeagueClientUx|OP\.GG/i);

    vi.mocked(invoke)
      .mockRejectedValueOnce("LeagueClientUx.exe refused")
      .mockResolvedValueOnce(make5v5ScoreboardDetail(1000));

    rerender(
      <I18nProvider locale="pt_BR">
        <ProfileTab summoner={mockSummoner} profile={mockProfile} loading={false} errorMsg={null} onRefresh={vi.fn()} />
      </I18nProvider>
    );

    const errorNotice = await screen.findByTestId("scoreboard-fallback-notice-1000");
    expect(errorNotice).toHaveTextContent("Não foi possível carregar o placar completo");
    expect(errorNotice.textContent).not.toContain("LeagueClientUx");

    fireEvent.click(screen.getByTestId("scoreboard-retry-1000"));
    await waitFor(() => expect(screen.getByTestId("scoreboard-team-100")).toBeInTheDocument());
  });
});

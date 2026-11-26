import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { I18nProvider } from "../../../i18n";
import { MatchHistoryPage, PlayerProfile, RecentMatchInfo } from "../../../types";
import { ProfileTab } from "../ProfileTab";
import { mockProfile, mockSummoner } from "./profileFixtures";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((p: string, proto = "asset") => `${proto}://localhost/${p}`),
}));

function makeMatch(id: number, overrides: Partial<RecentMatchInfo> = {}): RecentMatchInfo {
  return {
    ...mockProfile.recent_matches[0],
    game_id: id,
    game_creation_ms: 1_700_000_000_000 - id * 60_000,
    ...overrides,
  };
}

function makeProfile(matches: RecentMatchInfo[], nextBegIndex: number, hasMore = true): PlayerProfile {
  return {
    ...mockProfile,
    recent_matches: matches,
    history_next_beg_index: nextBegIndex,
    history_has_more: hasMore,
    item_metadata: {},
    rune_metadata: {},
    spell_metadata: {},
    augment_metadata: {},
  };
}

describe("P14.1 Task 2 — Incremental Match History Pagination", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders localized Match History titles and preserves raw Practice Tool cursor + deduplication + tooltips + 10-pill streak cap", async () => {
    const initial8Matches = Array.from({ length: 8 }, (_, i) =>
      makeMatch(500 + i, {
        champion_id: 103,
        champion_name: "Ahri",
        win: i < 4,
        game_mode_key: "ranked_solo",
        game_mode_label: "Solo/Duo",
      })
    );
    const page2: MatchHistoryPage = {
      matches: [
        makeMatch(507, { champion_id: 103, champion_name: "Ahri", win: false }), // duplicate from page 1
        makeMatch(600, { champion_id: 238, champion_name: "Zed", win: true, item_ids: [3157, 0, 0, 0, 0, 0], item_icon_urls: ["https://ddragon.leagueoflegends.com/cdn/16.22.1/img/item/3157.png", "", "", "", "", ""] }),
        makeMatch(601, { champion_id: 238, champion_name: "Zed", win: true }),
        makeMatch(602, { champion_id: 238, champion_name: "Zed", win: true }),
        makeMatch(603, { champion_id: 238, champion_name: "Zed", win: true }),
        makeMatch(604, { champion_id: 238, champion_name: "Zed", win: true }),
      ],
      next_beg_index: 20,
      has_more: true,
      item_metadata: {
        3157: { id: 3157, name: "Ampulheta de Zhonya", gold_total: 3250, stats: ["120 AP"], plaintext: "Estase" },
      },
      rune_metadata: {},
      spell_metadata: {},
      augment_metadata: {},
    };

    vi.mocked(invoke).mockResolvedValueOnce(page2);

    const { rerender } = render(
      <I18nProvider locale="en_US">
        <ProfileTab summoner={mockSummoner} profile={makeProfile(initial8Matches, 10, true)} loading={false} errorMsg={null} onRefresh={vi.fn()} />
      </I18nProvider>
    );
    expect(screen.getByRole("heading", { name: "Match History" })).toBeInTheDocument();

    rerender(
      <I18nProvider locale="pt_BR">
        <ProfileTab summoner={mockSummoner} profile={makeProfile(initial8Matches, 10, true)} loading={false} errorMsg={null} onRefresh={vi.fn()} />
      </I18nProvider>
    );
    expect(screen.getByRole("heading", { name: "Histórico de Partidas" })).toBeInTheDocument();
    expect(screen.getAllByTestId(/^match-item-/)).toHaveLength(8);
    expect(screen.getByTestId("winloss-donut-svg")).toHaveTextContent("50%");

    fireEvent.click(screen.getByTestId("load-more-matches-btn"));

    await waitFor(() => {
      expect(screen.getAllByTestId(/^match-item-/)).toHaveLength(13);
    });
    expect(invoke).toHaveBeenCalledWith("get_player_match_history_page", { begIndex: 10, count: 10 });
    expect(screen.getAllByTestId("match-item-507")).toHaveLength(1);
    expect(screen.getByTestId("win-count")).toHaveTextContent("9");
    expect(screen.getByTestId("loss-count")).toHaveTextContent("4");
    expect(screen.getByTestId("winloss-donut-svg")).toHaveTextContent("69%");
    expect(screen.getByTestId("highlighted-champion")).toHaveTextContent("Ahri");
    expect(screen.getByTestId("recent-streak").querySelectorAll(".streak-pill")).toHaveLength(10);

    const zhonyaImg = screen.getByTestId("match-item-600").querySelector("img[alt='Ampulheta de Zhonya']")!;
    fireEvent.mouseEnter(zhonyaImg.closest(".ui-tooltip-trigger")!);
    expect(screen.getByRole("tooltip")).toHaveTextContent("Ampulheta de Zhonya");
  });

  it("preserves active mode filter on load more and hides button when has_more is false", async () => {
    const initial = [
      makeMatch(700, { game_mode_key: "ranked_solo", game_mode_label: "Solo/Duo", win: true }),
      makeMatch(701, { game_mode_key: "aram", game_mode_label: "ARAM", win: false }),
    ];
    vi.mocked(invoke).mockResolvedValueOnce({
      matches: [
        makeMatch(702, { game_mode_key: "aram", game_mode_label: "ARAM", win: true }),
        makeMatch(703, { game_mode_key: "ranked_solo", game_mode_label: "Solo/Duo", win: true }),
      ],
      next_beg_index: 14,
      has_more: false,
      item_metadata: {},
      rune_metadata: {},
      spell_metadata: {},
      augment_metadata: {},
    } satisfies MatchHistoryPage);

    render(
      <I18nProvider locale="pt_BR">
        <ProfileTab summoner={mockSummoner} profile={makeProfile(initial, 10, true)} loading={false} errorMsg={null} onRefresh={vi.fn()} />
      </I18nProvider>
    );

    fireEvent.click(screen.getByTestId("mode-filter-aram"));
    expect(screen.getAllByTestId(/^match-item-/)).toHaveLength(1);

    fireEvent.click(screen.getByTestId("load-more-matches-btn"));

    await waitFor(() => {
      expect(screen.getAllByTestId(/^match-item-/)).toHaveLength(2);
    });
    expect(screen.getByTestId("mode-filter-aram")).toHaveAttribute("aria-pressed", "true");
    expect(screen.queryByTestId("load-more-matches-btn")).not.toBeInTheDocument();
  });

  it("resets pagination, closes open drawer, and discards late responses on refresh or summoner switch", async () => {
    let resolvePending!: (value: MatchHistoryPage) => void;
    const pendingPromise = new Promise<MatchHistoryPage>((res) => {
      resolvePending = res;
    });
    vi.mocked(invoke)
      .mockResolvedValueOnce({
        matches: [makeMatch(801)],
        next_beg_index: 20,
        has_more: true,
        item_metadata: {},
        rune_metadata: {},
        spell_metadata: {},
        augment_metadata: {},
      })
      .mockImplementationOnce(() => pendingPromise);

    const onRefresh = vi.fn();
    const profile1 = makeProfile([makeMatch(800)], 10, true);
    const { rerender } = render(
      <I18nProvider locale="pt_BR">
        <ProfileTab summoner={mockSummoner} profile={profile1} loading={false} errorMsg={null} onRefresh={onRefresh} />
      </I18nProvider>
    );

    fireEvent.click(screen.getByTestId("load-more-matches-btn"));
    await waitFor(() => expect(screen.getAllByTestId(/^match-item-/)).toHaveLength(2));

    fireEvent.click(screen.getByTestId("match-item-800").querySelector("button.match-row-btn")!);
    expect(screen.getByTestId("match-details-800")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("refresh-profile-btn"));
    expect(onRefresh).toHaveBeenCalledTimes(1);
    expect(screen.queryByTestId("match-details-800")).not.toBeInTheDocument();
    expect(screen.getAllByTestId(/^match-item-/)).toHaveLength(1);

    // Start in-flight page request, then switch summoner before it resolves
    fireEvent.click(screen.getByTestId("load-more-matches-btn"));
    expect(screen.getByTestId("load-more-matches-dots")).toBeInTheDocument();

    const otherSummoner = { ...mockSummoner, summonerId: 111222, gameName: "Other" };
    const profile2 = { ...makeProfile([makeMatch(900)], 10, true), summoner: otherSummoner };
    rerender(
      <I18nProvider locale="pt_BR">
        <ProfileTab summoner={otherSummoner} profile={profile2} loading={false} errorMsg={null} onRefresh={onRefresh} />
      </I18nProvider>
    );

    resolvePending({
      matches: [makeMatch(802)],
      next_beg_index: 20,
      has_more: true,
      item_metadata: {},
      rune_metadata: {},
      spell_metadata: {},
      augment_metadata: {},
    });

    await waitFor(() => {
      expect(screen.getByTestId("match-item-900")).toBeInTheDocument();
    });
    expect(screen.queryByTestId("match-item-802")).not.toBeInTheDocument();
  });

  it("recovers from fetch errors with localized copy and disables pagination when offline", async () => {
    vi.mocked(invoke)
      .mockRejectedValueOnce("LeagueClientUx.exe connection refused")
      .mockResolvedValueOnce({
        matches: [makeMatch(951)],
        next_beg_index: 20,
        has_more: false,
        item_metadata: {},
        rune_metadata: {},
        spell_metadata: {},
        augment_metadata: {},
      });

    const { rerender } = render(
      <I18nProvider locale="pt_BR">
        <ProfileTab summoner={mockSummoner} profile={makeProfile([makeMatch(950)], 10, true)} loading={false} errorMsg={null} onRefresh={vi.fn()} />
      </I18nProvider>
    );

    fireEvent.click(screen.getByTestId("load-more-matches-btn"));
    const errorEl = await screen.findByTestId("load-more-matches-error");
    expect(errorEl).toHaveTextContent("Não foi possível carregar mais partidas.");
    expect(errorEl.textContent).not.toContain("LeagueClientUx");

    fireEvent.click(screen.getByTestId("load-more-matches-btn"));
    await waitFor(() => expect(screen.getAllByTestId(/^match-item-/)).toHaveLength(2));
    expect(screen.queryByTestId("load-more-matches-error")).not.toBeInTheDocument();

    rerender(
      <I18nProvider locale="pt_BR">
        <ProfileTab summoner={null} profile={makeProfile([makeMatch(950)], 10, true)} loading={false} errorMsg={null} onRefresh={vi.fn()} />
      </I18nProvider>
    );
    expect(screen.getByTestId("load-more-matches-btn")).toBeDisabled();
  });
});

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ProfileTab } from "../components/ProfileTab";
import { PlayerProfile, SummonerInfo } from "../types";

const mockSummoner: SummonerInfo = {
  summonerId: 987654321,
  gameName: "Schuma",
  tagLine: "BR1",
  displayName: "Schuma",
  summonerLevel: 350,
  profileIconId: 548,
};

const mockProfile: PlayerProfile = {
  summoner: mockSummoner,
  profile_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/profileicon/548.png",
  background_skin_id: 103015,
  background_skin_name: "Ahri Florescer Espiritual",
  background_splash_url: "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_15.jpg",
  background_splash_fallback_url: "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_0.jpg",
  ranked_stats: [
    {
      queue_type: "RANKED_SOLO_5x5",
      queue_label: "Solo/Duo",
      tier: "EMERALD",
      tier_label: "Esmeralda",
      division: "II",
      league_points: 67,
      wins: 52,
      losses: 44,
    },
    {
      queue_type: "RANKED_FLEX_SR",
      queue_label: "Flex",
      tier: "UNRANKED",
      tier_label: "Sem Ranque",
      division: "",
      league_points: 0,
      wins: 0,
      losses: 0,
    },
  ],
  top_masteries: [
    {
      champion_id: 103,
      champion_name: "Ahri",
      champion_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Ahri.png",
      champion_level: 25,
      champion_points: 452310,
    },
    {
      champion_id: 238,
      champion_name: "Zed",
      champion_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Zed.png",
      champion_level: 18,
      champion_points: 310400,
    },
    {
      champion_id: 157,
      champion_name: "Yasuo",
      champion_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Yasuo.png",
      champion_level: 12,
      champion_points: 185000,
    },
  ],
  recent_matches: [
    ...Array.from({ length: 10 }, (_, idx) => {
      const id = 1000 + idx;
      const isSolo = idx < 5;
      const isAram = idx >= 5 && idx < 8;
      const win = isSolo ? idx !== 2 : isAram ? idx === 5 : idx === 8;
      return {
        game_id: id,
        queue_id: isSolo ? 420 : isAram ? 450 : 440,
        game_mode_key: isSolo ? "ranked_solo" : isAram ? "aram" : "ranked_flex",
        game_mode_label: isSolo ? "Solo/Duo" : isAram ? "ARAM" : "Flex",
        champion_id: isSolo ? 103 : 238,
        champion_name: isSolo ? "Ahri" : "Zed",
        champion_icon_url: isSolo
          ? "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Ahri.png"
          : "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Zed.png",
        win,
        kills: 9,
        deaths: 3,
        assists: 7,
        cs: 195,
        game_duration_secs: 1800,
        game_creation_ms: Date.now() - (idx + 1) * 3600000,
        spell1_id: 4,
        spell1_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/spell/SummonerFlash.png",
        spell2_id: 14,
        spell2_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/spell/SummonerDot.png",
        item_ids: [6655, 3020, 3089, 3157, 0, 0],
        item_icon_urls: [
          "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/item/6655.png",
          "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/item/3020.png",
          "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/item/3089.png",
          "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/item/3157.png",
          "",
          "",
        ],
        gold_earned: 14500,
        total_damage_dealt_to_champions: 29800,
        vision_score: 22,
        wards_placed: 9,
        champ_level: 16,
        double_kills: idx === 0 ? 1 : 0,
        triple_kills: 0,
        quadra_kills: 0,
        penta_kills: 0,
      };
    }),
  ],
};

describe("ProfileTab Component", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("renders waiting state when League client is disconnected", () => {
    render(
      <ProfileTab
        summoner={null}
        profile={null}
        loading={false}
        errorMsg={null}
        onRefresh={vi.fn()}
      />
    );

    expect(screen.getByTestId("profile-waiting-client")).toBeInTheDocument();
    expect(screen.getByText(/Aguardando Cliente do LoL/i)).toBeInTheDocument();
  });

  it("renders summoner icon, level, ranked stats with crest icons on the right, top 3 masteries, 10 matches, and omits wallpaper skin name text", () => {
    const { container } = render(
      <ProfileTab
        summoner={mockSummoner}
        profile={mockProfile}
        loading={false}
        errorMsg={null}
        onRefresh={vi.fn()}
      />
    );

    // Identity (player name beside avatar, without ranked row nested inside identity)
    expect(screen.getByText("Schuma")).toBeInTheDocument();
    expect(screen.getByText("#BR1")).toBeInTheDocument();
    expect(screen.getByTestId("profile-level")).toHaveTextContent("Nv. 350");
    expect(screen.getByAltText("Ícone do Invocador")).toHaveAttribute(
      "src",
      mockProfile.profile_icon_url
    );

    const identityEl = container.querySelector(".profile-identity")!;
    const heroRightEl = container.querySelector(".profile-hero-right")!;
    const rankedRow = screen.getByTestId("profile-ranked-row");
    expect(identityEl).not.toContainElement(rankedRow);
    expect(heroRightEl).toContainElement(rankedRow);

    // Must NOT display background skin name text
    expect(screen.queryByText(/Ahri Florescer Espiritual/i)).not.toBeInTheDocument();

    // Ranked stats with rank crest icons
    const soloCard = screen.getByTestId("ranked-pill-RANKED_SOLO_5x5");
    expect(soloCard).toHaveTextContent("Solo/Duo");
    expect(soloCard).toHaveTextContent("Esmeralda II");
    expect(soloCard).toHaveTextContent("67 PDL · 52V 44D (54%)");
    const soloImg = soloCard.querySelector("img")!;
    expect(soloImg).toHaveAttribute(
      "src",
      "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-shared-components/global/default/images/emerald.png"
    );

    // Image error falls back to ranked-mini-crests SVG
    fireEvent.error(soloImg);
    expect(soloImg).toHaveAttribute(
      "src",
      "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/images/ranked-mini-crests/emerald.svg"
    );

    const flexCard = screen.getByTestId("ranked-pill-RANKED_FLEX_SR");
    expect(flexCard).toHaveTextContent("Flex");
    expect(flexCard).toHaveTextContent("Sem Ranque");
    const flexImg = flexCard.querySelector("img")!;
    expect(flexImg).toHaveAttribute(
      "src",
      "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-shared-components/global/default/images/unranked.png"
    );

    // Top 3 masteries
    expect(screen.getByTestId("mastery-item-1")).toHaveTextContent("Ahri");
    expect(screen.getByTestId("mastery-item-2")).toHaveTextContent("Zed");
    expect(screen.getByTestId("mastery-item-3")).toHaveTextContent("Yasuo");

    // 10 recent matches + Win/Loss SVG donut (6W 4L => 60%)
    expect(screen.getAllByTestId(/^match-item-/)).toHaveLength(10);
    expect(screen.getByTestId("winloss-donut-svg")).toHaveTextContent("60%");
    expect(screen.getByTestId("win-count")).toHaveTextContent("6");
    expect(screen.getByTestId("loss-count")).toHaveTextContent("4");
  });

  it("filters the 10 matches by game mode, updates Win/Loss chart dynamically, and never displays Practice Tool", () => {
    const profileWithPracticeLeak: PlayerProfile = {
      ...mockProfile,
      recent_matches: [
        ...mockProfile.recent_matches.slice(0, 9),
        {
          ...mockProfile.recent_matches[0],
          game_id: 9999,
          queue_id: -1,
          game_mode_key: "practicetool",
          game_mode_label: "Modo Treino",
        },
      ],
    };

    render(
      <ProfileTab
        summoner={mockSummoner}
        profile={profileWithPracticeLeak}
        loading={false}
        errorMsg={null}
        onRefresh={vi.fn()}
      />
    );

    expect(screen.queryByTestId("mode-filter-practicetool")).not.toBeInTheDocument();
    expect(screen.queryByText(/Modo Treino/i)).not.toBeInTheDocument();
    expect(screen.queryByTestId("match-item-9999")).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId("mode-filter-ranked_solo"));
    expect(screen.getAllByTestId(/^match-item-/)).toHaveLength(5);
    expect(screen.getByTestId("winloss-donut-svg")).toHaveTextContent("80%");
    expect(screen.getByTestId("win-count")).toHaveTextContent("4");
    expect(screen.getByTestId("loss-count")).toHaveTextContent("1");

    fireEvent.click(screen.getByTestId("mode-filter-aram"));
    expect(screen.getAllByTestId(/^match-item-/)).toHaveLength(3);
    expect(screen.getByTestId("winloss-donut-svg")).toHaveTextContent("33%");
    expect(screen.getByTestId("win-count")).toHaveTextContent("1");
    expect(screen.getByTestId("loss-count")).toHaveTextContent("2");
  });

  it("supports expanding match details and displaying refresh cooldown timer without wallpaper button", () => {
    const onRefresh = vi.fn();

    const { rerender } = render(
      <ProfileTab
        summoner={mockSummoner}
        profile={mockProfile}
        loading={false}
        errorMsg={null}
        onRefresh={onRefresh}
        refreshCooldownSecs={0}
      />
    );

    // Wallpaper visibility moved to SettingsTab
    expect(screen.queryByTestId("wallpaper-mode-btn")).not.toBeInTheDocument();

    // Expand match 1000 details
    expect(screen.queryByTestId("match-details-1000")).not.toBeInTheDocument();
    const firstMatchBtn = screen.getByTestId("match-item-1000").querySelector("button")!;
    fireEvent.click(firstMatchBtn);
    expect(screen.getByTestId("match-details-1000")).toBeInTheDocument();
    expect(screen.getByTestId("match-details-1000")).toHaveTextContent("29,8k");

    // Manual refresh trigger
    const refreshBtn = screen.getByTestId("refresh-profile-btn");
    expect(refreshBtn).toBeEnabled();
    fireEvent.click(refreshBtn);
    expect(onRefresh).toHaveBeenCalledTimes(1);

    // Cooldown disables button and shows countdown
    rerender(
      <ProfileTab
        summoner={mockSummoner}
        profile={mockProfile}
        loading={false}
        errorMsg={null}
        onRefresh={onRefresh}
        refreshCooldownSecs={25}
      />
    );

    expect(screen.getByTestId("refresh-profile-btn")).toBeDisabled();
    expect(screen.getByTestId("refresh-profile-btn")).toHaveTextContent("Atualizar (25s)");
  });
});

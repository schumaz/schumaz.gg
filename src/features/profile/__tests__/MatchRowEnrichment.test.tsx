import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { I18nProvider } from "../../../i18n";
import { PlayerProfile, RecentMatchInfo } from "../../../types";
import { ProfileTab } from "../ProfileTab";
import { computeWinLossStats } from "../profileHelpers";
import { mockProfile, mockSummoner } from "./profileFixtures";

function makeMatch(overrides: Partial<RecentMatchInfo>): RecentMatchInfo {
  return {
    ...mockProfile.recent_matches[0],
    ...overrides,
  };
}

describe("P14.1 Task 1 — Complete Match Row Enrichment", () => {
  it("renders Keystone + Secondary runes, spells, 2x3 items + trinket slot, rich tooltips, and valid button HTML", () => {
    const srMatch = makeMatch({
      game_id: 2001,
      queue_id: 420,
      game_mode_key: "ranked_solo",
      game_mode_label: "Solo/Duo",
      primary_rune_id: 8112,
      primary_rune_icon_url:
        "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/Domination/Electrocute/Electrocute.png",
      primary_style_id: 8100,
      primary_style_icon_url:
        "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/7200_Domination.png",
      sub_style_id: 8200,
      sub_style_icon_url:
        "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/7202_Sorcery.png",
      trinket_id: 3340,
      trinket_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/item/3340.png",
    });

    const profile: PlayerProfile = {
      ...mockProfile,
      recent_matches: [srMatch],
      item_metadata: {
        3340: {
          id: 3340,
          name: "Sentinela Invisível",
          gold_total: 0,
          stats: [],
          plaintext: "Posiciona uma sentinela invisível.",
        },
      },
      rune_metadata: {
        8112: {
          id: 8112,
          key: "Electrocute",
          name: "Eletrocutar",
          icon_url:
            "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/Domination/Electrocute/Electrocute.png",
          short_desc: "Atingir um Campeão com 3 ataques ou habilidades causa dano adaptativo.",
        },
        8200: {
          id: 8200,
          key: "Sorcery",
          name: "Feitiçaria",
          icon_url:
            "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/7202_Sorcery.png",
          short_desc: "",
        },
      },
      spell_metadata: {
        4: {
          id: 4,
          name: "Flash",
          icon_url:
            "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/spell/SummonerFlash.png",
          modes: ["CLASSIC"],
          description: "Teleporta seu Campeão por uma curta distância.",
          cooldown_secs: 300,
        },
      },
    };

    render(
      <I18nProvider locale="pt_BR">
        <ProfileTab
          summoner={mockSummoner}
          profile={profile}
          loading={false}
          errorMsg={null}
          onRefresh={vi.fn()}
        />
      </I18nProvider>
    );

    const card = screen.getByTestId("match-item-2001");
    expect(screen.getByTestId("match-runes-2001")).toBeInTheDocument();
    const trinketSlot = card.querySelector("[data-testid='match-trinket-slot']")!;
    expect(trinketSlot).toBeInTheDocument();

    // Valid HTML inside <button>: no nested buttons or focusable tabindex="0" triggers
    const rowBtn = card.querySelector("button.match-row-btn")!;
    expect(rowBtn.querySelectorAll("button, [tabindex='0']")).toHaveLength(0);

    // Rich tooltip on Keystone rune
    const keystoneImg = screen.getByAltText("Eletrocutar");
    fireEvent.mouseEnter(keystoneImg.closest(".ui-tooltip-trigger")!);
    expect(screen.getByRole("tooltip")).toHaveTextContent("Eletrocutar");
    expect(screen.getByRole("tooltip")).toHaveTextContent(/3 ataques/);
    fireEvent.mouseLeave(keystoneImg.closest(".ui-tooltip-trigger")!);

    // Rich tooltip on Summoner spell
    const flashImg = screen.getByAltText("Flash");
    fireEvent.mouseEnter(flashImg.closest(".ui-tooltip-trigger")!);
    expect(screen.getByRole("tooltip")).toHaveTextContent("300s recarga");
    fireEvent.mouseLeave(flashImg.closest(".ui-tooltip-trigger")!);

    // Rich tooltip on Trinket (item6)
    const trinketImg = trinketSlot.querySelector("img[alt='Sentinela Invisível']")!;
    fireEvent.mouseEnter(trinketImg.closest(".ui-tooltip-trigger")!);
    expect(screen.getByRole("tooltip")).toHaveTextContent("Sentinela Invisível");
    expect(screen.getByRole("tooltip")).toHaveTextContent(/Posiciona uma sentinela/);
  });

  it("renders neutral Remake badge and excludes remake matches from Win/Loss stats and streak pills", () => {
    const matches = [
      makeMatch({ game_id: 2002, win: false, remake: true, kills: 0, deaths: 0, assists: 0 }),
      makeMatch({ game_id: 2003, win: true, remake: false }),
      makeMatch({ game_id: 2004, win: false, remake: false }),
    ];

    expect(computeWinLossStats(matches)).toEqual({
      total: 2,
      wins: 1,
      losses: 1,
      winRate: 50,
    });

    const profile: PlayerProfile = {
      ...mockProfile,
      recent_matches: matches,
    };

    render(
      <I18nProvider locale="pt_BR">
        <ProfileTab
          summoner={mockSummoner}
          profile={profile}
          loading={false}
          errorMsg={null}
          onRefresh={vi.fn()}
        />
      </I18nProvider>
    );

    const remakeCard = screen.getByTestId("match-item-2002");
    expect(remakeCard).toHaveClass("remake");
    expect(screen.getByTestId("match-remake-badge-2002")).toHaveTextContent("Remake");
    expect(screen.getByTestId("winloss-donut-svg")).toHaveTextContent("50%");
    expect(screen.getByTestId("win-count")).toHaveTextContent("1");
    expect(screen.getByTestId("loss-count")).toHaveTextContent("1");
    expect(screen.getByTestId("recent-streak").querySelectorAll(".streak-pill")).toHaveLength(2);
  });

  it("renders Arena placement pill, omits CS, and shows Augment icons + rich tooltips in Arena & ARAM Mayhem", () => {
    const arenaMatch = makeMatch({
      game_id: 2005,
      queue_id: 1700,
      game_mode_key: "arena",
      game_mode_label: "Arena",
      win: true,
      remake: false,
      arena_placement: 1,
      cs: 0,
      primary_rune_id: 0,
      primary_rune_icon_url: "",
      sub_style_id: 0,
      sub_style_icon_url: "",
      augment_ids: [501, 502],
    });
    const mayhemMatch = makeMatch({
      game_id: 2006,
      queue_id: 2400,
      game_mode_key: "aram_mayhem",
      game_mode_label: "ARAM Mayhem",
      win: true,
      remake: false,
      arena_placement: null,
      augment_ids: [501],
    });

    const profile: PlayerProfile = {
      ...mockProfile,
      recent_matches: [arenaMatch, mayhemMatch],
      augment_metadata: {
        501: {
          id: 501,
          name: "Eureka",
          desc: "Concede Aceleração de Habilidade igual ao Poder de Habilidade.",
          icon_url:
            "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/cherry/augments/icons/eureka_small.png",
          rarity: 8,
        },
        502: {
          id: 502,
          name: "Matador de Tanques",
          desc: "Causa dano adicional contra alvos com mais Vida.",
          icon_url:
            "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/cherry/augments/icons/tank_small.png",
          rarity: 4,
        },
      },
    };

    render(
      <I18nProvider locale="pt_BR">
        <ProfileTab
          summoner={mockSummoner}
          profile={profile}
          loading={false}
          errorMsg={null}
          onRefresh={vi.fn()}
        />
      </I18nProvider>
    );

    const placementPill = screen.getByTestId("match-placement-2005");
    expect(placementPill).toHaveTextContent("1º");
    expect(placementPill).toHaveClass("tone-gold");

    const arenaCard = screen.getByTestId("match-item-2005");
    expect(arenaCard.querySelector(".match-kda-ratio")?.textContent).not.toMatch(/CS/i);

    const arenaAugments = screen.getByTestId("match-augments-2005");
    expect(arenaAugments).toBeInTheDocument();
    expect(screen.queryByTestId("match-runes-2005")).not.toBeInTheDocument();
    expect(screen.getByTestId("match-augments-2006")).toBeInTheDocument();

    const eurekaImg = arenaAugments.querySelector("img[alt='Eureka']")!;
    expect(eurekaImg.closest(".match-augment-frame")).toHaveClass("rarity-prismatic");
    fireEvent.mouseEnter(eurekaImg.closest(".ui-tooltip-trigger")!);
    const tooltip = screen.getByRole("tooltip");
    expect(tooltip).toHaveTextContent("Eureka");
    expect(tooltip).toHaveTextContent("Prismático");
    expect(tooltip).toHaveTextContent(/Aceleração de Habilidade/);
  });
});

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ProfileTab } from "../features/profile/ProfileTab";
import { PlayerProfile } from "../types";
import { mockProfile, mockSummoner } from "../features/profile/__tests__/profileFixtures";

describe("ProfileTab Component", () => {
  beforeEach(() => {
    localStorage.clear();
    // pure INI
  });

  it("renders waiting state with unified island branding when League client is disconnected", () => {
    const { container } = render(
      <ProfileTab
        summoner={null}
        profile={null}
        loading={false}
        errorMsg={null}
        onRefresh={vi.fn()}
      />
    );

    const waiting = screen.getByTestId("profile-waiting-client");
    expect(waiting).toBeInTheDocument();
    expect(screen.getByText(/Aguardando Cliente do LoL/i)).toBeInTheDocument();
    expect(screen.getByText("schumaz.gg · Perfil")).toBeInTheDocument();
    expect(container.querySelector(".idle-card")).not.toHaveAttribute("style");
    expect(waiting.querySelector(".idle-radar-ring")).toBeInTheDocument();
    expect(screen.queryByTestId("profile-waiting-client-dots")).not.toBeInTheDocument();
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

    expect(screen.queryByText(/Ahri Florescer Espiritual/i)).not.toBeInTheDocument();

    const soloCard = screen.getByTestId("ranked-pill-RANKED_SOLO_5x5");
    expect(soloCard).toHaveTextContent("Solo/Duo");
    expect(soloCard).toHaveTextContent("Esmeralda II");
    expect(soloCard).toHaveTextContent("67 PDL · 52V 44D (54%)");
    const soloImg = soloCard.querySelector("img")!;
    expect(soloImg).toHaveAttribute(
      "src",
      "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-shared-components/global/default/images/emerald.png"
    );

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

    expect(screen.getByTestId("mastery-item-1")).toHaveTextContent("Ahri");
    expect(screen.getByTestId("mastery-item-2")).toHaveTextContent("Zed");
    expect(screen.getByTestId("mastery-item-3")).toHaveTextContent("Yasuo");

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

  it("supports expanding match details, rich final-item tooltips, and refresh cooldown without wallpaper button", () => {
    const onRefresh = vi.fn();
    const profileWithItemMeta: PlayerProfile = {
      ...mockProfile,
      item_metadata: {
        6655: {
          id: 6655,
          name: "Companheiro de Luden",
          gold_total: 2750,
          stats: ["100 de Poder de Habilidade", "600 de Mana"],
          plaintext: "Dispara projéteis mágicos adicionais.",
        },
      },
    };

    const { rerender } = render(
      <ProfileTab
        summoner={mockSummoner}
        profile={profileWithItemMeta}
        loading={false}
        errorMsg={null}
        onRefresh={onRefresh}
        refreshCooldownSecs={0}
      />
    );

    expect(screen.queryByTestId("wallpaper-mode-btn")).not.toBeInTheDocument();
    expect(screen.queryByTestId("match-details-1000")).not.toBeInTheDocument();

    const firstMatchCard = screen.getByTestId("match-item-1000");
    const firstItemImg = firstMatchCard.querySelector("img[alt='Companheiro de Luden']")!;
    fireEvent.mouseEnter(firstItemImg.closest(".ui-tooltip-trigger")!);
    const tooltip = screen.getByRole("tooltip");
    expect(tooltip).toHaveTextContent("Companheiro de Luden");
    expect(tooltip).toHaveTextContent(/2[.,]750 ouro/);
    expect(tooltip).toHaveTextContent("100 de Poder de Habilidade");
    fireEvent.mouseLeave(firstItemImg.closest(".ui-tooltip-trigger")!);

    const firstMatchBtn = firstMatchCard.querySelector("button")!;
    fireEvent.click(firstMatchBtn);
    expect(screen.getByTestId("match-details-1000")).toBeInTheDocument();
    expect(screen.getByTestId("match-details-1000")).toHaveTextContent("29,8k");

    const refreshBtn = screen.getByTestId("refresh-profile-btn");
    expect(refreshBtn).toBeEnabled();
    fireEvent.click(refreshBtn);
    expect(onRefresh).toHaveBeenCalledTimes(1);

    rerender(
      <ProfileTab
        summoner={mockSummoner}
        profile={profileWithItemMeta}
        loading={false}
        errorMsg={null}
        onRefresh={onRefresh}
        refreshCooldownSecs={25}
      />
    );

    expect(screen.getByTestId("refresh-profile-btn")).toBeDisabled();
    expect(screen.getByTestId("refresh-profile-btn")).toHaveTextContent("Atualizar (25s)");
  });

  it("renders initial profile loading spinner ring and in-place refresh loading bar + dots", () => {
    const { rerender } = render(
      <ProfileTab
        summoner={mockSummoner}
        profile={null}
        loading={true}
        errorMsg={null}
        onRefresh={vi.fn()}
      />
    );

    const loadingCard = screen.getByTestId("profile-loading");
    expect(loadingCard.querySelector(".idle-spinner-ring")).toBeInTheDocument();
    expect(screen.getByTestId("profile-loading-dots").querySelectorAll(".loading-dot")).toHaveLength(3);

    rerender(
      <ProfileTab
        summoner={mockSummoner}
        profile={mockProfile}
        loading={true}
        errorMsg={null}
        onRefresh={vi.fn()}
      />
    );

    expect(screen.getByTestId("profile-loading-bar")).toHaveAttribute("role", "progressbar");
    const refreshBtn = screen.getByTestId("refresh-profile-btn");
    expect(refreshBtn).toBeDisabled();
    expect(refreshBtn).toHaveTextContent("Atualizando");
    expect(screen.getByTestId("profile-refresh-dots").querySelectorAll(".loading-dot")).toHaveLength(3);
  });
});

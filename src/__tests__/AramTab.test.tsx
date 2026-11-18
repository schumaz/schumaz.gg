import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import {
  AramTab,
  LEAGUE_ARAM_ICON_URL,
  LEAGUE_ARAM_MAYHEM_ICON_URL,
} from "../features/aram/AramTab";
import {
  mockAhriAugments,
  mockAramChampions,
  mockMayhemChampions,
} from "../features/aram/__tests__/aramFixtures";

describe("AramTab UI", () => {
  it("renders official League icons in the ARAM mode chooser", () => {
    render(
      <AramTab
        isActive={true}
        savedMode={null}
        showChooser={true}
        tierLists={{}}
        tierLoading={{}}
        tierErrors={{}}
        selectedChampionKey={null}
        augmentsByChampion={{}}
        augmentsLoading={false}
        augmentsError={null}
        onSelectMode={vi.fn()}
        onSelectChampion={vi.fn()}
        onRetryTierList={vi.fn()}
        onRetryAugments={vi.fn()}
      />
    );

    const aramBtn = screen.getByTestId("aram-choose-aram");
    const mayhemBtn = screen.getByTestId("aram-choose-mayhem");
    expect(aramBtn.querySelector("img")).toHaveAttribute("src", LEAGUE_ARAM_ICON_URL);
    expect(mayhemBtn.querySelector("img")).toHaveAttribute("src", LEAGUE_ARAM_MAYHEM_ICON_URL);
  });

  it("renders classic ARAM with unified branding, ARAM badge on the right, no champion count pill, and filters by search", () => {
    render(
      <AramTab
        isActive={true}
        savedMode="aram"
        showChooser={false}
        tierLists={{ aram: mockAramChampions }}
        tierLoading={{}}
        tierErrors={{}}
        selectedChampionKey={null}
        augmentsByChampion={{}}
        augmentsLoading={false}
        augmentsError={null}
        onSelectMode={vi.fn()}
        onSelectChampion={vi.fn()}
        onRetryTierList={vi.fn()}
        onRetryAugments={vi.fn()}
      />
    );

    const classicView = screen.getByTestId("aram-classic-view");
    const heroHeading = within(classicView).getByRole("heading", { name: "Ranking de Campeões" });
    expect(heroHeading).toHaveClass("page-hero-title");

    const rightContainer = classicView.querySelector(".aram-hero-right") as HTMLElement;
    expect(rightContainer).toBeTruthy();
    expect(within(rightContainer).getByText("ARAM")).toHaveClass("aram-mode-badge");
    expect(within(rightContainer).getByLabelText("Buscar campeão no ARAM")).toBeInTheDocument();

    expect(within(classicView).queryByText(/2 campeões/i)).not.toBeInTheDocument();

    const list = screen.getByTestId("aram-classic-list");
    expect(within(list).getByText("Swain")).toBeInTheDocument();
    expect(within(list).getByText("Miss Fortune")).toBeInTheDocument();
    expect(within(list).getByText("OP")).toHaveClass("tier-op");
    expect(within(list).getByText("Tier 1")).toHaveClass("tier-1");
    expect(list.querySelectorAll(".dd-icon-frame.aram-champ-frame .aram-champ-avatar")).toHaveLength(2);

    expect(screen.queryByLabelText("Limpar busca de campeão no ARAM")).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Buscar campeão no ARAM"), {
      target: { value: "miss" },
    });
    expect(within(list).queryByText("Swain")).not.toBeInTheDocument();
    expect(within(list).getByText("Miss Fortune")).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText("Limpar busca de campeão no ARAM"));
    expect(screen.getByLabelText("Buscar campeão no ARAM")).toHaveValue("");
    expect(within(list).getByText("Swain")).toBeInTheDocument();
  });

  it("orders ARAM Mayhem augments best-to-worst, shows explicit Cartinhas branding title, and filters by rarity and search", () => {
    render(
      <AramTab
        isActive={true}
        savedMode="aram_mayhem"
        showChooser={false}
        tierLists={{ aram_mayhem: mockMayhemChampions }}
        tierLoading={{}}
        tierErrors={{}}
        selectedChampionKey="ahri"
        augmentsByChampion={{ ahri: mockAhriAugments }}
        augmentsLoading={false}
        augmentsError={null}
        onSelectMode={vi.fn()}
        onSelectChampion={vi.fn()}
        onRetryTierList={vi.fn()}
        onRetryAugments={vi.fn()}
      />
    );

    const mayhemView = screen.getByTestId("aram-mayhem-view");
    const titleRow = mayhemView.querySelector(".aram-panel-title-row") as HTMLElement;
    expect(titleRow).toBeTruthy();
    const mayhemBadge = within(titleRow).getByText("ARAM Mayhem");
    expect(mayhemBadge).toHaveClass("aram-mode-badge", "is-mayhem");
    const cartinhasHeading = within(titleRow).getByRole("heading", { name: "Cartinhas" });
    expect(cartinhasHeading).toHaveClass("aram-mayhem-hero-title");
    expect(Array.from(titleRow.children)).toEqual([mayhemBadge, cartinhasHeading]);
    expect(
      within(mayhemView).queryByText("Busque o campeão para ver as cartinhas")
    ).not.toBeInTheDocument();

    const tfRow = screen.getByTestId("mayhem-champ-twistedfate");
    expect(within(tfRow).getByText("OP")).toHaveClass("tier-op");
    const ahriRow = screen.getByTestId("mayhem-champ-ahri");
    expect(within(ahriRow).getByText("Tier 1")).toHaveClass("tier-1");

    const augList = screen.getByTestId("aram-augments-list");
    const names = Array.from(augList.querySelectorAll(".aram-augment-name")).map(
      (el) => el.textContent
    );
    expect(names).toEqual(["Coice", "Conjuração Ecoante", "Raio Encolhedor"]);

    const silverBtn = screen.getByRole("button", { name: "Prata" });
    expect(silverBtn).toHaveClass("segment-btn");

    fireEvent.click(silverBtn);
    expect(within(augList).getByText("Raio Encolhedor")).toBeInTheDocument();
    expect(within(augList).queryByText("Coice")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Ouro" }));
    expect(within(augList).getByText("Coice")).toBeInTheDocument();
    expect(within(augList).queryByText("Conjuração Ecoante")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Prismática" }));
    expect(within(augList).getByText("Conjuração Ecoante")).toBeInTheDocument();
    expect(within(augList).queryByText("Coice")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Todas" }));
    fireEvent.change(screen.getByLabelText("Buscar cartinha"), {
      target: { value: "coice" },
    });
    expect(within(augList).getByText("Coice")).toBeInTheDocument();
    expect(within(augList).queryByText("Conjuração Ecoante")).not.toBeInTheDocument();
    fireEvent.click(screen.getByLabelText("Limpar busca de cartinha"));
    expect(screen.getByLabelText("Buscar cartinha")).toHaveValue("");

    fireEvent.change(screen.getByLabelText("Buscar campeão no ARAM Mayhem"), {
      target: { value: "ahri" },
    });
    expect(screen.queryByTestId("mayhem-champ-twistedfate")).not.toBeInTheDocument();
    fireEvent.click(screen.getByLabelText("Limpar busca de campeão no ARAM Mayhem"));
    expect(screen.getByTestId("mayhem-champ-twistedfate")).toBeInTheDocument();
  });

  it("renders interactive loading skeletons and dots in ARAM classic and Mayhem champions & augments", () => {
    const baseProps = {
      isActive: true,
      showChooser: false,
      tierErrors: {},
      augmentsByChampion: {},
      augmentsError: null,
      onSelectMode: vi.fn(),
      onSelectChampion: vi.fn(),
      onRetryTierList: vi.fn(),
      onRetryAugments: vi.fn(),
    };

    const { rerender } = render(
      <AramTab
        {...baseProps}
        savedMode="aram"
        tierLists={{}}
        tierLoading={{ aram: true }}
        selectedChampionKey={null}
        augmentsLoading={false}
      />
    );

    expect(screen.getByTestId("aram-classic-loading")).toHaveTextContent(/Carregando ranking do ARAM/i);
    expect(screen.getByTestId("aram-classic-loading-dots").querySelectorAll(".loading-dot")).toHaveLength(3);
    expect(screen.getByTestId("aram-classic-skeleton")).toHaveAttribute("aria-hidden", "true");

    rerender(
      <AramTab
        {...baseProps}
        savedMode="aram"
        tierLists={{ aram: mockAramChampions }}
        tierLoading={{ aram: true }}
        selectedChampionKey={null}
        augmentsLoading={false}
      />
    );

    expect(screen.getByTestId("aram-classic-loading-bar")).toHaveAttribute("role", "progressbar");

    rerender(
      <AramTab
        {...baseProps}
        savedMode="aram_mayhem"
        tierLists={{}}
        tierLoading={{ aram_mayhem: true }}
        selectedChampionKey={null}
        augmentsLoading={false}
      />
    );

    expect(screen.getByTestId("aram-mayhem-champions-loading")).toHaveTextContent(/Carregando campeões/i);
    expect(screen.getByTestId("aram-mayhem-champions-skeleton")).toHaveAttribute("aria-hidden", "true");

    rerender(
      <AramTab
        {...baseProps}
        savedMode="aram_mayhem"
        tierLists={{ aram_mayhem: mockMayhemChampions }}
        tierLoading={{ aram_mayhem: false }}
        selectedChampionKey="ahri"
        augmentsLoading={true}
      />
    );

    expect(screen.getByTestId("aram-augments-loading")).toHaveTextContent(/Carregando cartinhas/i);
    expect(screen.getByTestId("aram-augments-skeleton")).toHaveAttribute("aria-hidden", "true");
  });
});

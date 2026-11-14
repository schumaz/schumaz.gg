import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, within } from "@testing-library/react";
import { renderHook, act } from "@testing-library/react";
import {
  AramTab,
  LEAGUE_ARAM_ICON_URL,
  LEAGUE_ARAM_MAYHEM_ICON_URL,
  getChampionTierClass,
  getChampionTierLabel,
} from "../components/AramTab";
import { NavigationDock } from "../components/NavigationDock";
import { useAram } from "../hooks/useAram";
import type { AramAugmentEntry_Serialize, AramChampionRank_Serialize } from "../types/bindings";
import { ARAM_SUBMODE_STORAGE_KEY, commands } from "../types";

const mockAramChampions: AramChampionRank_Serialize[] = [
  {
    id: 50,
    key: "swain",
    name: "Swain",
    icon_url: "https://example.com/swain.png",
    tier: 0,
    rank: 1,
    win_rate: 63.3,
    pick_rate: 9.1,
  },
  {
    id: 21,
    key: "missfortune",
    name: "Miss Fortune",
    icon_url: "https://example.com/mf.png",
    tier: 1,
    rank: 2,
    win_rate: 52.8,
    pick_rate: 13.7,
  },
];

const mockMayhemChampions: AramChampionRank_Serialize[] = [
  {
    id: 4,
    key: "twistedfate",
    name: "Twisted Fate",
    icon_url: "https://example.com/tf.png",
    tier: 0,
    rank: 1,
    win_rate: null,
    pick_rate: null,
  },
  {
    id: 103,
    key: "ahri",
    name: "Ahri",
    icon_url: "https://example.com/ahri.png",
    tier: 1,
    rank: 14,
    win_rate: null,
    pick_rate: null,
  },
];

const mockAhriAugments: AramAugmentEntry_Serialize[] = [
  {
    id: 1074,
    name: "Raio Encolhedor",
    tier: 2,
    rarity: 1,
    performance: 76.8,
    popular: 0.4,
    icon_url: "https://example.com/shrink.png",
    desc: "Reduz o dano do alvo.",
  },
  {
    id: 2062,
    name: "Conjuração Ecoante",
    tier: 0,
    rarity: 8,
    performance: 114.2,
    popular: 3.2,
    icon_url: "https://example.com/echo.png",
    desc: "Conjura habilidades duas vezes.",
  },
  {
    id: 3075,
    name: "Coice",
    tier: 0,
    rarity: 4,
    performance: 99.6,
    popular: 3.8,
    icon_url: "https://example.com/recoil.png",
    desc: "Aumenta o dano e recua.",
  },
];

describe("NavigationDock Aram label and icon", () => {
  it("always displays the dock button label as Aram with an official League icon", () => {
    render(<NavigationDock activeTab="aram" onTabChange={vi.fn()} onAramClick={vi.fn()} />);
    const btn = screen.getByRole("button", { name: "Aram" });
    expect(btn).toBeInTheDocument();
    const img = btn.querySelector("img.dock-icon-img");
    expect(img).toHaveAttribute("src", LEAGUE_ARAM_ICON_URL);
  });
});

describe("useAram persistence and click behavior", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.restoreAllMocks();
  });

  it("asks only on first click, saves choice, opens saved tab on 1 click, and reopens chooser on 2nd click", async () => {
    vi.spyOn(commands, "getAramTierList").mockResolvedValue(mockAramChampions);
    vi.spyOn(commands, "getAramMayhemAugments").mockResolvedValue(mockAhriAugments);

    const { result } = renderHook(({ isActive }) => useAram(isActive, null), {
      initialProps: { isActive: false },
    });

    // No saved mode initially -> clicking Aram from another tab opens chooser
    expect(result.current.savedMode).toBeNull();
    act(() => {
      result.current.handleAramDockClick(false);
    });
    expect(result.current.showChooser).toBe(true);

    // Selecting ARAM saves it to localStorage and closes chooser
    act(() => {
      result.current.selectMode("aram");
    });
    expect(result.current.savedMode).toBe("aram");
    expect(result.current.showChooser).toBe(false);
    expect(localStorage.getItem(ARAM_SUBMODE_STORAGE_KEY)).toBe("aram");

    // Coming from another tab clicks once -> opens saved mode directly (no chooser)
    act(() => {
      result.current.handleAramDockClick(false);
    });
    expect(result.current.showChooser).toBe(false);

    // Clicking Aram a second time while already on Aram -> opens chooser to change mode
    act(() => {
      result.current.handleAramDockClick(true);
    });
    expect(result.current.showChooser).toBe(true);
  });

  it("auto-selects current champion in ARAM Mayhem when active in Champ Select", async () => {
    localStorage.setItem(ARAM_SUBMODE_STORAGE_KEY, "aram_mayhem");
    vi.spyOn(commands, "getAramTierList").mockResolvedValue(mockMayhemChampions);
    const augSpy = vi
      .spyOn(commands, "getAramMayhemAugments")
      .mockResolvedValue(mockAhriAugments);

    const { result } = renderHook(() => useAram(true, 103));

    await waitFor(() => {
      expect(augSpy).toHaveBeenCalledWith("ahri");
    });
    expect(result.current.selectedChampionKey).toBe("ahri");
    expect(result.current.augmentsByChampion.ahri).toHaveLength(3);
  });
});

describe("AramTab UI", () => {
  it("maps champion tiers 0..5 to OP and Tier 1..5 labels and CSS classes", () => {
    expect(getChampionTierLabel(0)).toBe("OP");
    expect(getChampionTierClass(0)).toBe("tier-op");
    expect(getChampionTierLabel(1)).toBe("Tier 1");
    expect(getChampionTierClass(1)).toBe("tier-1");
    expect(getChampionTierLabel(2)).toBe("Tier 2");
    expect(getChampionTierClass(2)).toBe("tier-2");
    expect(getChampionTierLabel(3)).toBe("Tier 3");
    expect(getChampionTierClass(3)).toBe("tier-3");
    expect(getChampionTierLabel(4)).toBe("Tier 4");
    expect(getChampionTierClass(4)).toBe("tier-4");
    expect(getChampionTierLabel(5)).toBe("Tier 5");
    expect(getChampionTierClass(5)).toBe("tier-5");
  });

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

    // ARAM badge moved to the right container alongside the search input
    const rightContainer = classicView.querySelector(".aram-hero-right") as HTMLElement;
    expect(rightContainer).toBeTruthy();
    expect(within(rightContainer).getByText("ARAM")).toHaveClass("aram-mode-badge");
    expect(within(rightContainer).getByLabelText("Buscar campeão no ARAM")).toBeInTheDocument();

    // Champion count pill removed
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
    const cartinhasHeading = within(mayhemView).getByRole("heading", { name: "Cartinhas" });
    expect(cartinhasHeading).toHaveClass("aram-mayhem-hero-title");
    expect(
      within(mayhemView).queryByText("Busque o campeão para ver as cartinhas")
    ).not.toBeInTheDocument();

    const tfRow = screen.getByTestId("mayhem-champ-twistedfate");
    expect(within(tfRow).getByText("OP")).toHaveClass("tier-op");
    expect(tfRow.querySelector(".dd-icon-frame.aram-champ-frame .aram-champ-avatar")).toBeTruthy();
    const ahriRow = screen.getByTestId("mayhem-champ-ahri");
    expect(within(ahriRow).getByText("Tier 1")).toHaveClass("tier-1");

    expect(mayhemView.querySelector(".dd-icon-frame.aram-hero-frame .aram-hero-avatar")).toBeTruthy();

    const augList = screen.getByTestId("aram-augments-list");
    const names = Array.from(augList.querySelectorAll(".aram-augment-name")).map(
      (el) => el.textContent
    );
    // Tier 0 (Coice pop=3.8, Conjuração Ecoante pop=3.2) before Tier 2 (Raio Encolhedor)
    expect(names).toEqual(["Coice", "Conjuração Ecoante", "Raio Encolhedor"]);

    // Rarity filter uses project's .segment-btn class
    const silverBtn = screen.getByRole("button", { name: "Prata" });
    expect(silverBtn).toHaveClass("segment-btn");

    // Filter by Prata (rarity 1)
    fireEvent.click(silverBtn);
    expect(within(augList).getByText("Raio Encolhedor")).toBeInTheDocument();
    expect(within(augList).queryByText("Coice")).not.toBeInTheDocument();

    // Filter by Ouro (rarity 4)
    fireEvent.click(screen.getByRole("button", { name: "Ouro" }));
    expect(within(augList).getByText("Coice")).toBeInTheDocument();
    expect(within(augList).queryByText("Conjuração Ecoante")).not.toBeInTheDocument();

    // Filter by Prismática (rarity 8)
    fireEvent.click(screen.getByRole("button", { name: "Prismática" }));
    expect(within(augList).getByText("Conjuração Ecoante")).toBeInTheDocument();
    expect(within(augList).queryByText("Coice")).not.toBeInTheDocument();

    // Back to Todas + search + clear X buttons in both Mayhem inputs
    fireEvent.click(screen.getByRole("button", { name: "Todas" }));
    expect(screen.queryByLabelText("Limpar busca de cartinha")).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Buscar cartinha"), {
      target: { value: "coice" },
    });
    expect(within(augList).getByText("Coice")).toBeInTheDocument();
    expect(within(augList).queryByText("Conjuração Ecoante")).not.toBeInTheDocument();
    fireEvent.click(screen.getByLabelText("Limpar busca de cartinha"));
    expect(screen.getByLabelText("Buscar cartinha")).toHaveValue("");
    expect(within(augList).getByText("Conjuração Ecoante")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Buscar campeão no ARAM Mayhem"), {
      target: { value: "ahri" },
    });
    expect(screen.queryByTestId("mayhem-champ-twistedfate")).not.toBeInTheDocument();
    fireEvent.click(screen.getByLabelText("Limpar busca de campeão no ARAM Mayhem"));
    expect(screen.getByLabelText("Buscar campeão no ARAM Mayhem")).toHaveValue("");
    expect(screen.getByTestId("mayhem-champ-twistedfate")).toBeInTheDocument();
  });
});

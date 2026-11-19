import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import { NavigationDock } from "../../../components/ui/NavigationDock";
import { ArenaTab } from "../ArenaTab";
import {
  mockArenaChampions,
  mockArenaMasterYiBuild,
  mockArenaTierResponse,
  mockProtectedArenaTierResponse,
} from "./arenaFixtures";

describe("NavigationDock Arena position", () => {
  it("renders Arena strictly between Ranqueada and Aram with active aria-current", () => {
    const onArenaClick = vi.fn();
    render(<NavigationDock activeTab="arena" onTabChange={vi.fn()} onArenaClick={onArenaClick} />);

    const buttons = screen.getAllByRole("button");
    expect(buttons.map((b) => b.textContent?.trim())).toEqual([
      "Perfil",
      "Runas",
      "Ranqueada",
      "Arena",
      "Aram",
      "Configurações",
    ]);

    const arenaBtn = screen.getByRole("button", { name: "Arena" });
    expect(arenaBtn).toHaveAttribute("aria-current", "page");
    fireEvent.click(arenaBtn);
    expect(onArenaClick).toHaveBeenCalledTimes(1);
  });
});

describe("ArenaTab UI", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  const baseProps = {
    isActive: true,
    filters: { patch: null },
    tierData: mockArenaTierResponse,
    tierLoading: false,
    tierError: null,
    selectedChampion: null,
    championBuild: null,
    buildLoading: false,
    buildError: null,
    selectedBuildIndex: 0,
    onPatchChange: vi.fn(),
    onDetailFilterChange: vi.fn(),
    onSelectChampion: vi.fn(),
    onSelectSynergyChampion: vi.fn(),
    onBackToTierList: vi.fn(),
    onRetryTierList: vi.fn(),
    onRetryBuild: vi.fn(),
    onSelectBuild: vi.fn(),
  };

  it("renders Arena tier list grouped by tier with Arena metrics and no lane/elo/region controls", () => {
    const onSelectChampion = vi.fn();
    render(<ArenaTab {...baseProps} onSelectChampion={onSelectChampion} />);

    const toolbar = screen.getByTestId("arena-filters-card");
    expect(within(toolbar).getByRole("heading", { name: "Arena" })).toHaveClass("page-hero-title");
    expect(within(toolbar).getByText("3x6")).toHaveClass("arena-mode-badge");
    expect(screen.queryByTestId("ranked-lane-tabs")).not.toBeInTheDocument();
    expect(screen.queryByTestId("ranked-filter-tier")).not.toBeInTheDocument();
    expect(screen.queryByTestId("ranked-filter-region")).not.toBeInTheDocument();

    const opGroup = screen.getByTestId("arena-tier-group-0");
    const opButtons = within(opGroup).getAllByRole("button");
    expect(opButtons.map((b) => b.getAttribute("data-testid"))).toEqual([
      "arena-champ-masteryi",
      "arena-champ-volibear",
    ]);

    const yiRow = screen.getByTestId("arena-champ-masteryi");
    expect(within(yiRow).getByText("#1")).toBeInTheDocument();
    expect(within(yiRow).getByText("Top 3")).toBeInTheDocument();
    expect(within(yiRow).getByText("56.4%")).toBeInTheDocument();
    expect(within(yiRow).getByText("1º")).toBeInTheDocument();
    expect(within(yiRow).getByText("16.8%")).toBeInTheDocument();
    expect(within(yiRow).getByText("14.3%")).toBeInTheDocument();

    fireEvent.click(yiRow);
    expect(onSelectChampion).toHaveBeenCalledWith(mockArenaChampions[0]);
  });

  it("filters champions by Tier dropdown and search input", () => {
    render(<ArenaTab {...baseProps} />);

    const tierTrigger = screen.getByTestId("arena-filter-champ-tier");
    fireEvent.click(tierTrigger);
    const tierListbox = screen.getByRole("listbox", { name: "Tier:" });
    fireEvent.click(within(tierListbox).getByRole("option", { name: "Tier 1" }));

    expect(screen.queryByTestId("arena-tier-group-0")).not.toBeInTheDocument();
    expect(screen.getByTestId("arena-tier-group-1")).toBeInTheDocument();
    expect(screen.getByTestId("arena-champ-lulu")).toBeInTheDocument();

    fireEvent.click(tierTrigger);
    fireEvent.click(within(screen.getByRole("listbox", { name: "Tier:" })).getByRole("option", { name: "Todos" }));
    fireEvent.change(screen.getByLabelText("Buscar campeão na Arena"), { target: { value: "voli" } });

    expect(screen.getByTestId("arena-champ-volibear")).toBeInTheDocument();
    expect(screen.queryByTestId("arena-champ-masteryi")).not.toBeInTheDocument();

    fireEvent.click(screen.getByLabelText("Limpar busca na Arena"));
    expect(screen.getByTestId("arena-champ-masteryi")).toBeInTheDocument();
  });

  it("renders Patch dropdown with 48h patch protection badge, tooltip, and manual patch selection", () => {
    const onPatchChange = vi.fn();
    render(
      <ArenaTab
        {...baseProps}
        tierData={mockProtectedArenaTierResponse}
        onPatchChange={onPatchChange}
      />
    );

    expect(screen.getByTestId("arena-patch-fallback-badge")).toHaveTextContent(
      /Volume Protegido \(16\.21\).*Faltam 34h/i
    );
    fireEvent.mouseEnter(screen.getByTestId("arena-patch-protection-indicator"));
    expect(screen.getByTestId("arena-patch-protection-tooltip")).toHaveTextContent(/34 horas/i);

    const patchTrigger = screen.getByTestId("arena-filter-patch");
    fireEvent.click(patchTrigger);
    const listbox = screen.getByRole("listbox", { name: "Patch:" });
    fireEvent.click(within(listbox).getByRole("option", { name: "Patch 16.22 (Mais atual)" }));
    expect(onPatchChange).toHaveBeenCalledWith("16.22");
  });

  it("renders ArenaChampionDetail with Augments, Best Synergies, read-only Arena ItemBlocks, and Back button", () => {
    const onBackToTierList = vi.fn();
    const onSelectSynergyChampion = vi.fn();
    const onSelectBuild = vi.fn();

    render(
      <ArenaTab
        {...baseProps}
        selectedChampion={mockArenaChampions[0]}
        championBuild={mockArenaMasterYiBuild}
        onBackToTierList={onBackToTierList}
        onSelectSynergyChampion={onSelectSynergyChampion}
        onSelectBuild={onSelectBuild}
      />
    );

    const view = screen.getByTestId("view-arena");
    expect(within(view).getByTestId("arena-champion-detail")).toBeInTheDocument();
    expect(within(view).getByTestId("champ-hero")).toHaveTextContent("OP · #1 Arena");
    expect(within(view).getByTestId("champ-hero")).toHaveTextContent("Arena (3x6)");
    expect(within(view).queryByTestId("arena-mode-notice")).not.toBeInTheDocument();
    expect(within(view).getByTestId("arena-augments-card")).toBeInTheDocument();
    const synergiesCard = within(view).getByTestId("arena-synergies-card");
    expect(within(synergiesCard).getByText("Melhores Sinergias")).toBeInTheDocument();
    expect(within(synergiesCard).getByText("2 aliados")).toBeInTheDocument();
    expect(within(view).getByTestId("item-blocks")).toBeInTheDocument();
    expect(within(view).getByText("Build Recomendada")).toBeInTheDocument();
    expect(within(view).getByText(/1\. Itens Prismáticos/i)).toBeInTheDocument();

    fireEvent.click(within(view).getByTestId("arena-rarity-btn-8"));
    expect(within(view).getByText("Duelo Místico")).toBeInTheDocument();
    expect(within(view).queryByText("Mãos Ágeis")).not.toBeInTheDocument();

    fireEvent.click(within(view).getByTestId("arena-synergy-117"));
    expect(onSelectSynergyChampion).toHaveBeenCalledWith(117);

    fireEvent.click(within(view).getByTestId("build-opt-btn-1"));
    expect(onSelectBuild).toHaveBeenCalledWith(1);

    fireEvent.click(within(view).getByTestId("ranked-back-btn"));
    expect(onBackToTierList).toHaveBeenCalledTimes(1);
  });

  it("renders loading and error states cleanly for tier list and champion build", () => {
    const onRetryTierList = vi.fn();
    const onRetryBuild = vi.fn();

    const { rerender } = render(
      <ArenaTab
        {...baseProps}
        tierData={null}
        tierLoading={true}
        onRetryTierList={onRetryTierList}
        onRetryBuild={onRetryBuild}
      />
    );
    expect(screen.getByTestId("arena-tierlist-loading")).toBeInTheDocument();

    rerender(
      <ArenaTab
        {...baseProps}
        tierData={null}
        tierLoading={false}
        tierError="Falha na Arena"
        onRetryTierList={onRetryTierList}
        onRetryBuild={onRetryBuild}
      />
    );
    fireEvent.click(screen.getByRole("button", { name: "Tentar novamente" }));
    expect(onRetryTierList).toHaveBeenCalledTimes(1);

    rerender(
      <ArenaTab
        {...baseProps}
        selectedChampion={mockArenaChampions[0]}
        championBuild={null}
        buildLoading={true}
        onRetryTierList={onRetryTierList}
        onRetryBuild={onRetryBuild}
      />
    );
    expect(screen.getByTestId("arena-build-loading")).toHaveTextContent(
      /Carregando build e aprimoramentos de Master Yi/i
    );
  });
});

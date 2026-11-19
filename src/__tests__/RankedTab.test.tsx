import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import { NavigationDock } from "../components/ui/NavigationDock";
import { RankedTab } from "../features/ranked/RankedTab";
import {
  mockProtectedTierResponse,
  mockTierResponse,
} from "../features/ranked/__tests__/rankedFixtures";

describe("NavigationDock Ranqueada position", () => {
  it("places Ranqueada and Arena strictly between Runas and Aram", () => {
    render(<NavigationDock activeTab="ranked" onTabChange={vi.fn()} />);
    const buttons = screen.getAllByRole("button");
    expect(buttons.map((b) => b.textContent?.trim())).toEqual([
      "Perfil",
      "Runas",
      "Ranqueada",
      "Arena",
      "Aram",
      "Configurações",
    ]);
    expect(screen.getByRole("button", { name: "Ranqueada" })).toHaveAttribute(
      "aria-current",
      "page"
    );
  });
});

describe("RankedTab UI", () => {
  it("combines all lanes in 'Todas as Rotas', differentiates global rank and lane per champ, and removes duplicate Rota dropdown", () => {
    const onTierListFilterChange = vi.fn();
    render(
      <RankedTab
        isActive={true}
        filters={{ region: "global", tier: "emerald_plus", lane: "all", patch: null }}
        tierData={mockTierResponse}
        tierLoading={false}
        tierError={null}
        selectedChampion={null}
        championBuild={null}
        buildLoading={false}
        buildError={null}
        orderedSpells={[]}
        selectedRunePageIndex={0}
        selectedBuildIndex={0}
        onTierListFilterChange={onTierListFilterChange}
        onDetailFilterChange={vi.fn()}
        onSelectChampion={vi.fn()}
        onBackToTierList={vi.fn()}
        onRetryTierList={vi.fn()}
        onRetryBuild={vi.fn()}
        onSelectRunePage={vi.fn()}
        onSelectBuild={vi.fn()}
      />
    );

    const filtersCard = screen.getByTestId("ranked-filters-card");
    const heroTitle = within(filtersCard).getByRole("heading", { name: "Tier List" });
    expect(heroTitle).toHaveClass("page-hero-title");
    expect(within(filtersCard).getByText("Ranqueada")).toHaveClass("ranked-mode-badge");
    expect(screen.queryByText(/entradas/i)).not.toBeInTheDocument();

    expect(within(filtersCard).getByTestId("ranked-toolbar-search-wrap")).toBeInTheDocument();
    expect(within(filtersCard).getByLabelText("Buscar campeão na Ranqueada")).toBeInTheDocument();

    expect(screen.queryByTestId("ranked-filter-position")).not.toBeInTheDocument();
    expect(screen.getByTestId("ranked-filter-tier")).toBeInTheDocument();
    expect(screen.getByTestId("ranked-filter-region")).toBeInTheDocument();
    expect(screen.getByTestId("ranked-filter-patch")).toBeInTheDocument();

    expect(screen.getByTestId("ranked-lane-tabs")).toBeInTheDocument();
    expect(screen.getByTestId("ranked-filter-champ-tier")).toBeInTheDocument();

    expect(screen.queryByTestId("ranked-lane-section-top")).not.toBeInTheDocument();
    expect(screen.getByTestId("ranked-all-lanes-list")).toBeInTheDocument();

    const opGroup = screen.getByTestId("ranked-tier-group-all-0");
    expect(within(opGroup).getAllByText("OP")[0]).toHaveClass("tier-op");
    const opButtons = within(opGroup).getAllByRole("button");
    expect(opButtons.map((btn) => btn.getAttribute("data-testid"))).toEqual([
      "ranked-champ-mid-ekko",
      "ranked-champ-top-malphite",
      "ranked-champ-mid-ahri",
    ]);

    expect(within(screen.getByTestId("ranked-champ-mid-ekko")).getByText("#1")).toBeInTheDocument();
    expect(within(screen.getByTestId("ranked-champ-top-malphite")).getByText("#2")).toBeInTheDocument();
    expect(within(screen.getByTestId("ranked-champ-mid-ahri")).getByText("#3")).toBeInTheDocument();
    expect(screen.getByTestId("ranked-champ-lane-mid-ekko")).toHaveTextContent("Meio");
    expect(screen.getByTestId("ranked-champ-lane-top-malphite")).toHaveTextContent("Topo");
    expect(screen.getByTestId("ranked-champ-lane-jungle-ekko")).toHaveTextContent("Selva");

    const tier1Group = screen.getByTestId("ranked-tier-group-all-1");
    expect(within(tier1Group).getByText("Garen")).toBeInTheDocument();
    const tier2Group = screen.getByTestId("ranked-tier-group-all-2");
    expect(within(tier2Group).getByText("Ekko")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("ranked-lane-tab-mid"));
    expect(onTierListFilterChange).toHaveBeenCalledWith("position", "mid");
  });

  it("filters champions by Tier menu (OP, Tier 1, Tier 2, Todos) alongside search", () => {
    render(
      <RankedTab
        isActive={true}
        filters={{ region: "global", tier: "emerald_plus", lane: "all", patch: null }}
        tierData={mockTierResponse}
        tierLoading={false}
        tierError={null}
        selectedChampion={null}
        championBuild={null}
        buildLoading={false}
        buildError={null}
        orderedSpells={[]}
        selectedRunePageIndex={0}
        selectedBuildIndex={0}
        onTierListFilterChange={vi.fn()}
        onDetailFilterChange={vi.fn()}
        onSelectChampion={vi.fn()}
        onBackToTierList={vi.fn()}
        onRetryTierList={vi.fn()}
        onRetryBuild={vi.fn()}
        onSelectRunePage={vi.fn()}
        onSelectBuild={vi.fn()}
      />
    );

    const tierTrigger = screen.getByTestId("ranked-filter-champ-tier");
    expect(tierTrigger).toHaveAttribute("data-value", "all");

    fireEvent.click(tierTrigger);
    const tierListbox = screen.getByRole("listbox", { name: "Tier:" });
    fireEvent.click(within(tierListbox).getByRole("option", { name: "Tier 1" }));

    expect(screen.queryByTestId("ranked-tier-group-all-0")).not.toBeInTheDocument();
    expect(screen.getByTestId("ranked-tier-group-all-1")).toBeInTheDocument();
    expect(screen.queryByTestId("ranked-tier-group-all-2")).not.toBeInTheDocument();
    expect(screen.getByText("Garen")).toBeInTheDocument();

    fireEvent.click(tierTrigger);
    fireEvent.click(within(screen.getByRole("listbox", { name: "Tier:" })).getByRole("option", { name: "OP" }));
    expect(screen.getByTestId("ranked-tier-group-all-0")).toBeInTheDocument();
    expect(screen.queryByTestId("ranked-tier-group-all-1")).not.toBeInTheDocument();

    expect(screen.queryByLabelText("Limpar busca na Ranqueada")).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Buscar campeão na Ranqueada"), {
      target: { value: "malph" },
    });
    expect(screen.getByTestId("ranked-champ-top-malphite")).toBeInTheDocument();
    expect(screen.queryByTestId("ranked-champ-mid-ahri")).not.toBeInTheDocument();

    fireEvent.click(screen.getByLabelText("Limpar busca na Ranqueada"));
    expect(screen.getByLabelText("Buscar campeão na Ranqueada")).toHaveValue("");
    expect(screen.getByTestId("ranked-champ-mid-ahri")).toBeInTheDocument();
    fireEvent.click(tierTrigger);
    fireEvent.click(within(screen.getByRole("listbox", { name: "Tier:" })).getByRole("option", { name: "Todos" }));
    expect(screen.getByTestId("ranked-tier-group-all-0")).toBeInTheDocument();
    expect(screen.getByTestId("ranked-tier-group-all-1")).toBeInTheDocument();
    expect(screen.getByTestId("ranked-tier-group-all-2")).toBeInTheDocument();
  });

  it("filters to a single lane section sorted by lane rank when a specific lane pill is selected", () => {
    render(
      <RankedTab
        isActive={true}
        filters={{ region: "global", tier: "emerald_plus", lane: "mid", patch: null }}
        tierData={mockTierResponse}
        tierLoading={false}
        tierError={null}
        selectedChampion={null}
        championBuild={null}
        buildLoading={false}
        buildError={null}
        orderedSpells={[]}
        selectedRunePageIndex={0}
        selectedBuildIndex={0}
        onTierListFilterChange={vi.fn()}
        onDetailFilterChange={vi.fn()}
        onSelectChampion={vi.fn()}
        onBackToTierList={vi.fn()}
        onRetryTierList={vi.fn()}
        onRetryBuild={vi.fn()}
        onSelectRunePage={vi.fn()}
        onSelectBuild={vi.fn()}
      />
    );

    const midSection = screen.getByTestId("ranked-lane-section-mid");
    expect(within(midSection).getByText("Meio")).toBeInTheDocument();
    expect(within(midSection).queryByText("Malphite")).not.toBeInTheDocument();

    const opMidGroup = screen.getByTestId("ranked-tier-group-mid-0");
    const midOpButtons = within(opMidGroup).getAllByRole("button");
    expect(midOpButtons.map((btn) => btn.getAttribute("data-testid"))).toEqual([
      "ranked-champ-mid-ahri",
      "ranked-champ-mid-ekko",
    ]);
  });

  it("displays Patch dropdown with 48h protection indicator, tooltip, and manual override", () => {
    const onTierListFilterChange = vi.fn();
    render(
      <RankedTab
        isActive={true}
        filters={{ region: "global", tier: "emerald_plus", lane: "all", patch: null }}
        tierData={mockProtectedTierResponse}
        tierLoading={false}
        tierError={null}
        selectedChampion={null}
        championBuild={null}
        buildLoading={false}
        buildError={null}
        orderedSpells={[]}
        selectedRunePageIndex={0}
        selectedBuildIndex={0}
        onTierListFilterChange={onTierListFilterChange}
        onDetailFilterChange={vi.fn()}
        onSelectChampion={vi.fn()}
        onBackToTierList={vi.fn()}
        onRetryTierList={vi.fn()}
        onRetryBuild={vi.fn()}
        onSelectRunePage={vi.fn()}
        onSelectBuild={vi.fn()}
      />
    );

    expect(screen.getByTestId("ranked-patch-fallback-badge")).toHaveTextContent(
      /Volume Protegido \(16\.21\).*Faltam 38h/i
    );
    const indicator = screen.getByTestId("ranked-patch-protection-indicator");
    fireEvent.mouseEnter(indicator);
    const tooltip = screen.getByTestId("ranked-patch-protection-tooltip");
    expect(tooltip).toHaveTextContent(/48h/i);
    expect(tooltip).toHaveTextContent(/Faltam 38 horas/i);

    const patchTrigger = screen.getByTestId("ranked-filter-patch");
    expect(patchTrigger).toHaveAttribute("data-value", "16.21");
    fireEvent.click(patchTrigger);
    const listbox = screen.getByRole("listbox", { name: "Patch:" });
    fireEvent.click(within(listbox).getByRole("option", { name: "Patch 16.22 (Mais atual)" }));
    expect(onTierListFilterChange).toHaveBeenCalledWith("patch", "16.22");
  });
});

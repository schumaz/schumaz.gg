import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import { RankedTab } from "../RankedTab";
import {
  mockAhriBuild,
  mockRankedChampions,
  mockTierResponse,
} from "./rankedFixtures";

describe("RankedTab Detail & Loading States", () => {
  it("renders read-only consultation build screen inside Ranked tab when a champion is selected", () => {
    const onBackToTierList = vi.fn();
    const onDetailFilterChange = vi.fn();

    render(
      <RankedTab
        isActive={true}
        filters={{ region: "global", tier: "emerald_plus", lane: "mid", patch: null }}
        tierData={mockTierResponse}
        tierLoading={false}
        tierError={null}
        selectedChampion={mockRankedChampions[2]}
        championBuild={mockAhriBuild}
        buildLoading={false}
        buildError={null}
        orderedSpells={mockAhriBuild.spells}
        selectedRunePageIndex={0}
        selectedBuildIndex={0}
        onTierListFilterChange={vi.fn()}
        onDetailFilterChange={onDetailFilterChange}
        onSelectChampion={vi.fn()}
        onBackToTierList={onBackToTierList}
        onRetryTierList={vi.fn()}
        onRetryBuild={vi.fn()}
        onSelectRunePage={vi.fn()}
        onSelectBuild={vi.fn()}
      />
    );

    const view = screen.getByTestId("view-ranked");
    expect(within(view).getByTestId("ranked-champion-detail")).toBeInTheDocument();
    expect(within(view).getByTestId("champ-hero")).toBeInTheDocument();
    expect(within(view).getByTestId("runes-tree")).toBeInTheDocument();
    expect(within(view).getByTestId("spells-card")).toBeInTheDocument();
    expect(within(view).getByTestId("skill-order-card")).toBeInTheDocument();
    expect(within(view).getByTestId("skill-priority-chain")).toHaveTextContent(/Q.*>.*W.*>.*E/);
    expect(within(view).getByTestId("item-blocks")).toBeInTheDocument();

    expect(within(view).getByText("Feitiços que Pode Usar")).toBeInTheDocument();
    expect(within(view).queryByText("Escolha de Segundo Feitiço")).not.toBeInTheDocument();
    expect(within(view).queryByTestId("invert-spells-btn")).not.toBeInTheDocument();
    expect(within(view).getByTestId("sec-spell-info-14")).toBeInTheDocument();
    expect(within(view).queryByTestId("sec-spell-btn-14")).not.toBeInTheDocument();
    expect(within(view).queryByText("D")).not.toBeInTheDocument();
    expect(within(view).queryByText("F")).not.toBeInTheDocument();
    expect(within(view).getByText("Build Recomendada")).toBeInTheDocument();
    expect(within(view).queryByText("Build na Loja")).not.toBeInTheDocument();

    fireEvent.click(within(view).getByTestId("ranked-back-btn"));
    expect(onBackToTierList).toHaveBeenCalledTimes(1);
  });

  it("renders interactive loading indicators for initial tier list, in-place reload, and champion build", () => {
    const baseProps = {
      isActive: true,
      filters: { lane: "all" as const, tier: "emerald_plus", region: "global", patch: "16.22" },
      tierError: null,
      buildError: null,
      orderedSpells: [],
      selectedRunePageIndex: 0,
      selectedBuildIndex: 0,
      onTierListFilterChange: vi.fn(),
      onDetailFilterChange: vi.fn(),
      onSelectChampion: vi.fn(),
      onBackToTierList: vi.fn(),
      onRetryTierList: vi.fn(),
      onRetryBuild: vi.fn(),
      onSelectRunePage: vi.fn(),
      onSelectBuild: vi.fn(),
    };

    const { rerender } = render(
      <RankedTab
        {...baseProps}
        tierData={null}
        tierLoading={true}
        selectedChampion={null}
        championBuild={null}
        buildLoading={false}
      />
    );

    expect(screen.getByTestId("ranked-tierlist-loading")).toHaveTextContent(
      /Carregando tier list ranqueada/i
    );
    expect(screen.getByTestId("ranked-tierlist-loading-dots").querySelectorAll(".loading-dot")).toHaveLength(3);
    expect(screen.getByTestId("ranked-tierlist-skeleton")).toHaveAttribute("aria-hidden", "true");
    expect(screen.queryByTestId("ranked-loading-bar")).not.toBeInTheDocument();

    rerender(
      <RankedTab
        {...baseProps}
        tierData={mockTierResponse}
        tierLoading={true}
        selectedChampion={null}
        championBuild={null}
        buildLoading={false}
      />
    );

    expect(screen.getByTestId("ranked-loading-bar")).toHaveAttribute("role", "progressbar");
    expect(screen.getByTestId("ranked-tierlist-scroll")).toBeInTheDocument();

    rerender(
      <RankedTab
        {...baseProps}
        tierData={mockTierResponse}
        tierLoading={false}
        selectedChampion={mockRankedChampions[2]}
        championBuild={null}
        buildLoading={true}
      />
    );

    expect(screen.getByTestId("ranked-build-loading")).toHaveTextContent(
      /Carregando runas e build de Ahri/i
    );
    expect(screen.getByTestId("ranked-build-loading-dots").querySelectorAll(".loading-dot")).toHaveLength(3);
  });
});

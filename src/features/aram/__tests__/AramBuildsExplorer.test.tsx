import { describe, it, expect, vi, beforeEach } from "vitest";
import { fireEvent, render, renderHook, screen, waitFor, within, act } from "@testing-library/react";
import { AramTab } from "../AramTab";
import { useAram } from "../useAram";
import { aramIpc, buildIpc, injectorIpc } from "../../../services/ipc";
import { mockAhriBuild } from "../../../__tests__/mockAhriBuildFixture";
import { mockAhriAugments, mockAramChampions, mockMayhemChampions } from "./aramFixtures";
import type { ChampionBuild } from "../../../types";

const aramClassicAhriBuild: ChampionBuild = {
  ...mockAhriBuild,
  active_position: "",
  active_tier: "all",
  supports_runes: true,
};

const aramMayhemAhriBuild: ChampionBuild = {
  ...mockAhriBuild,
  active_position: "",
  active_tier: "all",
  supports_runes: false,
  rune_pages: [],
};

const baseAramTabProps = {
  isActive: true,
  showChooser: false,
  tierErrors: {},
  tierLoading: {},
  augmentsByChampion: { ahri: mockAhriAugments },
  augmentsLoading: false,
  augmentsError: null,
  onSelectMode: vi.fn(),
  onSelectChampion: vi.fn(),
  onRetryTierList: vi.fn(),
  onRetryAugments: vi.fn(),
};

describe("P13.2 — Classic ARAM Build Explorer & ARAM Mayhem Build/Augments Toggle", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("opens Classic ARAM read-only build detail on champion click with SkillOrder, RuneTree, SpellsBar, ItemBlocks, and loading/error states", () => {
    const onSelectClassicChampion = vi.fn();
    const onBackToClassicList = vi.fn();
    const onRetryClassicBuild = vi.fn();
    const classicProps = { ...baseAramTabProps, savedMode: "aram" as const, tierLists: { aram: mockAramChampions }, selectedChampionKey: null };

    const { rerender } = render(<AramTab {...classicProps} onSelectClassicChampion={onSelectClassicChampion} />);
    fireEvent.click(screen.getByTestId("aram-classic-champ-swain"));
    expect(onSelectClassicChampion).toHaveBeenCalledWith(mockAramChampions[0]);

    rerender(<AramTab {...classicProps} selectedClassicChampion={mockAramChampions[0]} classicBuild={null} classicBuildLoading={true} onBackToClassicList={onBackToClassicList} />);
    expect(screen.getByTestId("aram-classic-build-loading")).toHaveTextContent(/Carregando runas e build de Swain/i);
    fireEvent.click(screen.getByTestId("aram-back-btn"));
    expect(onBackToClassicList).toHaveBeenCalledTimes(1);

    rerender(<AramTab {...classicProps} selectedClassicChampion={mockAramChampions[0]} classicBuild={null} classicBuildError="Falha ARAM" onRetryClassicBuild={onRetryClassicBuild} />);
    fireEvent.click(screen.getByRole("button", { name: "Tentar novamente" }));
    expect(onRetryClassicBuild).toHaveBeenCalledTimes(1);

    rerender(<AramTab {...classicProps} selectedClassicChampion={mockAramChampions[0]} classicBuild={aramClassicAhriBuild} onBackToClassicList={onBackToClassicList} />);
    const detail = screen.getByTestId("aram-classic-champion-detail");
    expect(within(detail).getByTestId("champ-hero")).toHaveTextContent("ARAM (Howling Abyss)");
    expect(within(detail).getByTestId("champ-hero")).toHaveTextContent("OP · #1 ARAM");
    expect(within(detail).getByTestId("filter-tier")).toBeInTheDocument();
    expect(within(detail).getByTestId("filter-region")).toBeInTheDocument();
    expect(within(detail).getByTestId("filter-patch")).toBeInTheDocument();
    expect(within(detail).queryByTestId("filter-position")).not.toBeInTheDocument();

    fireEvent.click(within(detail).getByTestId("skill-order-trigger"));
    expect(within(detail).getByTestId("skill-order-card")).toBeInTheDocument();

    expect(within(detail).getByText("Runas Recomendadas")).toBeInTheDocument();
    expect(within(detail).getByText("Eletrocutar")).toBeInTheDocument();
    fireEvent.click(within(detail).getByTestId("rune-page-btn-1"));
    expect(within(detail).getByText("Aprimoramento Glacial")).toBeInTheDocument();
    expect(within(detail).queryByText("Eletrocutar")).not.toBeInTheDocument();

    expect(within(detail).getByTestId("spell-pair-info-0")).toBeInTheDocument();
    expect(within(detail).getByTestId("invert-spells-readonly")).toBeInTheDocument();
    expect(within(detail).queryByTestId("spell-pair-btn-0")).not.toBeInTheDocument();
    expect(within(detail).queryByTestId("invert-spells-btn")).not.toBeInTheDocument();

    expect(within(detail).getByText("Build Recomendada")).toBeInTheDocument();
    fireEvent.click(within(detail).getByTestId("build-opt-btn-1"));
    expect(within(detail).getByTestId("active-build-meta")).toHaveTextContent(/31\.2% uso/i);

    fireEvent.click(within(detail).getByTestId("ranked-back-btn"));
    expect(onBackToClassicList).toHaveBeenCalledTimes(2);
  });

  it("toggles ARAM Mayhem right panel between Augments and Build & Feitiços with loading/error states", () => {
    const onRetryMayhemBuild = vi.fn();
    const mayhemProps = { ...baseAramTabProps, savedMode: "aram_mayhem" as const, tierLists: { aram_mayhem: mockMayhemChampions }, selectedChampionKey: "ahri" };

    const { rerender } = render(<AramTab {...mayhemProps} mayhemBuildsByChampion={{ ahri: aramMayhemAhriBuild }} />);
    const augTab = screen.getByTestId("mayhem-tab-augments");
    const buildTab = screen.getByTestId("mayhem-tab-build");
    expect(augTab).toHaveAttribute("aria-selected", "true");
    expect(screen.getByTestId("aram-augments-list")).toBeInTheDocument();

    fireEvent.click(buildTab);
    expect(buildTab).toHaveAttribute("aria-selected", "true");
    const buildPanel = screen.getByTestId("aram-mayhem-build-panel");
    expect(within(buildPanel).getByTestId("spell-pair-info-0")).toBeInTheDocument();
    expect(within(buildPanel).getByTestId("invert-spells-readonly")).toBeInTheDocument();
    expect(within(buildPanel).queryByTestId("runes-tree")).not.toBeInTheDocument();
    expect(within(buildPanel).getByText("Build Recomendada")).toBeInTheDocument();
    expect(within(buildPanel).getByText(/1\. Itens Iniciais/i)).toBeInTheDocument();
    expect(within(buildPanel).getByText(/2\. Primeiros 3 Itens/i)).toBeInTheDocument();
    expect(within(buildPanel).getByText(/3\. 3 Botas Mais Feitas/i)).toBeInTheDocument();
    expect(within(buildPanel).getByText(/4\. Build Final/i)).toBeInTheDocument();
    expect(within(buildPanel).getByText(/5\. Itens Situacionais/i)).toBeInTheDocument();

    fireEvent.click(within(buildPanel).getByTestId("build-opt-btn-1"));
    expect(within(buildPanel).getByTestId("active-build-meta")).toHaveTextContent(/31\.2% uso/i);

    fireEvent.click(screen.getByTestId("skill-order-trigger"));
    expect(screen.getByTestId("skill-order-card")).toBeInTheDocument();

    fireEvent.click(augTab);
    expect(screen.getByTestId("aram-augments-list")).toBeInTheDocument();

    rerender(<AramTab {...mayhemProps} mayhemPanel="build" mayhemBuildsByChampion={{}} mayhemBuildLoading={true} />);
    expect(screen.getByTestId("aram-mayhem-build-loading")).toHaveTextContent(/Carregando build e feitiços de Ahri/i);

    rerender(<AramTab {...mayhemProps} mayhemPanel="build" mayhemBuildsByChampion={{}} mayhemBuildError="Falha Mayhem" onRetryMayhemBuild={onRetryMayhemBuild} />);
    fireEvent.click(screen.getByRole("button", { name: "Tentar novamente" }));
    expect(onRetryMayhemBuild).toHaveBeenCalledTimes(1);
  });

  it("fetches Classic ARAM builds with filters and lazy-loads + caches ARAM Mayhem builds only on Build tab without LCU injection", async () => {
    vi.spyOn(aramIpc, "getAramTierList").mockImplementation((mode) =>
      Promise.resolve(mode === "aram" ? mockAramChampions : mockMayhemChampions)
    );
    const augSpy = vi.spyOn(aramIpc, "getAramMayhemAugments").mockResolvedValue(mockAhriAugments);
    const buildSpy = vi.spyOn(aramIpc, "getAramChampionBuild").mockImplementation((_id, mode) =>
      Promise.resolve(mode === "aram" ? aramClassicAhriBuild : aramMayhemAhriBuild)
    );
    const injectRunesSpy = vi.spyOn(injectorIpc, "injectRunePage");
    const injectBuildSpy = vi.spyOn(injectorIpc, "injectItemBuild");
    const injectSpellsSpy = vi.spyOn(injectorIpc, "injectSummonerSpells");
    const updateFiltersSpy = vi.spyOn(buildIpc, "updateBuildFilters");

    const classicHook = renderHook(() => useAram(true, null, "pt_BR", { savedAramMode: "aram" }, "D"));
    await waitFor(() => expect(classicHook.result.current.tierLists.aram).toHaveLength(2));

    act(() => { classicHook.result.current.selectClassicChampion(mockAramChampions[0]); });
    await waitFor(() => {
      expect(buildSpy).toHaveBeenCalledWith(50, "aram", null, "global", null);
      expect(classicHook.result.current.classicBuild?.champion_name).toBe("Ahri");
    });

    act(() => { classicHook.result.current.handleClassicDetailFilterChange("tier", "diamond_plus"); });
    await waitFor(() => expect(buildSpy).toHaveBeenCalledWith(50, "aram", "diamond_plus", "global", null));

    act(() => { classicHook.result.current.clearSelectedClassicChampion(); });
    expect(classicHook.result.current.selectedClassicChampion).toBeNull();
    expect(classicHook.result.current.classicBuild).toBeNull();
    classicHook.unmount();

    buildSpy.mockClear();
    const mayhemHook = renderHook(() => useAram(true, null, "pt_BR", { savedAramMode: "aram_mayhem" }, "D"));
    await waitFor(() => expect(mayhemHook.result.current.tierLists.aram_mayhem).toHaveLength(2));

    await act(async () => { await mayhemHook.result.current.selectChampion("ahri"); });
    expect(augSpy).toHaveBeenCalledWith("ahri");
    expect(buildSpy).not.toHaveBeenCalled();

    await act(async () => { mayhemHook.result.current.setMayhemPanel("build"); });
    await waitFor(() => {
      expect(buildSpy).toHaveBeenCalledWith(103, "aram_mayhem", null, "global", null);
      expect(mayhemHook.result.current.mayhemBuildsByChampion.ahri).toBeDefined();
    });

    act(() => { mayhemHook.result.current.setMayhemPanel("augments"); });
    act(() => { mayhemHook.result.current.setMayhemPanel("build"); });
    expect(buildSpy).toHaveBeenCalledTimes(1);
    mayhemHook.unmount();

    expect(injectRunesSpy).not.toHaveBeenCalled();
    expect(injectBuildSpy).not.toHaveBeenCalled();
    expect(injectSpellsSpy).not.toHaveBeenCalled();
    expect(updateFiltersSpy).not.toHaveBeenCalled();
  });

  it("retries failed ARAM Mayhem augment icons with backoff and falls back to initials", () => {
    vi.useFakeTimers();
    try {
      const augmentsWithMissing = [
        mockAhriAugments[2],
        { ...mockAhriAugments[0], id: 9991, name: "Empty Icon", icon_url: "" },
      ];
      render(
        <AramTab
          {...baseAramTabProps}
          savedMode="aram_mayhem"
          tierLists={{ aram_mayhem: mockMayhemChampions }}
          selectedChampionKey="ahri"
          augmentsByChampion={{ ahri: augmentsWithMissing }}
        />
      );

      expect(screen.getByTestId("aram-augment-icon-fallback")).toHaveTextContent("EM");
      const getImg = () => screen.getByRole("img", { name: "Coice" });
      expect(getImg()).toHaveAttribute("src", "https://example.com/recoil.png");

      fireEvent.error(getImg());
      expect(screen.queryByRole("img", { name: "Coice" })).not.toBeInTheDocument();

      act(() => {
        vi.advanceTimersByTime(400);
      });
      expect(getImg()).toHaveAttribute("src", "https://example.com/recoil.png?retry=1");

      fireEvent.error(getImg());
      act(() => {
        vi.advanceTimersByTime(1000);
      });
      expect(getImg()).toHaveAttribute("src", "https://example.com/recoil.png?retry=2");

      fireEvent.error(getImg());
      const fallbacks = screen.getAllByTestId("aram-augment-icon-fallback");
      expect(fallbacks.map((el) => el.textContent)).toEqual(["CO", "EM"]);
    } finally {
      vi.useRealTimers();
    }
  });
});

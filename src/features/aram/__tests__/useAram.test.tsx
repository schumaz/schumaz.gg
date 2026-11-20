import { describe, it, expect, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { renderHook, act } from "@testing-library/react";
import { AramTab, LEAGUE_ARAM_ICON_URL } from "../AramTab";
import { NavigationDock } from "../../../components/ui/NavigationDock";
import { useAram } from "../useAram";
import { aramIpc } from "../../../services/ipc";
import {
  mockAhriAugments,
  mockAramChampions,
  mockMayhemChampions,
} from "./aramFixtures";

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
    vi.restoreAllMocks();
  });

  it("asks only on first click, saves choice, opens saved tab on 1 click, and reopens chooser on 2nd click", async () => {
    vi.spyOn(aramIpc, "getAramTierList").mockResolvedValue(mockAramChampions);
    vi.spyOn(aramIpc, "getAramMayhemAugments").mockResolvedValue(mockAhriAugments);
    const onNavPatch = vi.fn();

    const { result } = renderHook(
      ({ isActive }) => useAram(isActive, null, "pt_BR", { onNavigationPatch: onNavPatch }),
      {
        initialProps: { isActive: false },
      }
    );

    expect(result.current.savedMode).toBeNull();
    act(() => {
      result.current.handleAramDockClick(false);
    });
    expect(result.current.showChooser).toBe(true);

    act(() => {
      result.current.selectMode("aram");
    });
    expect(result.current.savedMode).toBe("aram");
    expect(result.current.showChooser).toBe(false);
    expect(onNavPatch).toHaveBeenCalledWith({ savedAramMode: "aram" });

    act(() => {
      result.current.handleAramDockClick(false);
    });
    expect(result.current.showChooser).toBe(false);

    act(() => {
      result.current.handleAramDockClick(true);
    });
    expect(result.current.showChooser).toBe(true);
  });

  it("auto-selects current champion in ARAM Mayhem when active in Champ Select", async () => {
    vi.spyOn(aramIpc, "getAramTierList").mockResolvedValue(mockMayhemChampions);
    const augSpy = vi
      .spyOn(aramIpc, "getAramMayhemAugments")
      .mockResolvedValue(mockAhriAugments);

    const { result } = renderHook(() =>
      useAram(true, 103, "pt_BR", { savedAramMode: "aram_mayhem" })
    );

    await waitFor(() => {
      expect(augSpy).toHaveBeenCalledWith("ahri");
    });
    expect(result.current.selectedChampionKey).toBe("ahri");
    expect(result.current.augmentsByChampion.ahri).toHaveLength(3);
  });
});

describe("AramTab ephemeral ARAM Mayhem rarity filter", () => {
  it("always initializes rarity filter to All, filters locally, and resets to All on remount", () => {
    const props = {
      isActive: true,
      savedMode: "aram_mayhem" as const,
      showChooser: false,
      tierLists: { aram_mayhem: mockMayhemChampions },
      tierLoading: {},
      tierErrors: {},
      selectedChampionKey: "ahri",
      augmentsByChampion: { ahri: mockAhriAugments },
      augmentsLoading: false,
      augmentsError: null,
      onSelectMode: vi.fn(),
      onSelectChampion: vi.fn(),
      onRetryTierList: vi.fn(),
      onRetryAugments: vi.fn(),
    };

    const { unmount } = render(<AramTab {...props} />);

    // Initially shows all 3 rarities with "Todas" (All) active
    expect(screen.getByRole("button", { name: "Todas" })).toHaveClass("active");
    expect(screen.getByText("Raio Encolhedor")).toBeInTheDocument();
    expect(screen.getByText("Coice")).toBeInTheDocument();
    expect(screen.getByText("Conjuração Ecoante")).toBeInTheDocument();

    // Selecting "Ouro" filters locally without persistence
    fireEvent.click(screen.getByRole("button", { name: "Ouro" }));
    expect(screen.getByRole("button", { name: "Ouro" })).toHaveClass("active");
    expect(screen.getByText("Coice")).toBeInTheDocument();
    expect(screen.queryByText("Raio Encolhedor")).not.toBeInTheDocument();
    expect(screen.queryByText("Conjuração Ecoante")).not.toBeInTheDocument();

    // Simulating app restart (unmount + remount) always resets back to "Todas" (All)
    unmount();
    render(<AramTab {...props} />);
    expect(screen.getByRole("button", { name: "Todas" })).toHaveClass("active");
    expect(screen.getByText("Raio Encolhedor")).toBeInTheDocument();
    expect(screen.getByText("Coice")).toBeInTheDocument();
    expect(screen.getByText("Conjuração Ecoante")).toBeInTheDocument();
  });
});

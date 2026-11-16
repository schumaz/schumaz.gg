import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { renderHook, act } from "@testing-library/react";
import { LEAGUE_ARAM_ICON_URL } from "../AramTab";
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

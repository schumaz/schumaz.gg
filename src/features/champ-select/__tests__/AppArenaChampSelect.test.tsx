import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, renderHook, screen, fireEvent, waitFor, act, within } from "@testing-library/react";
import App from "../../../App";
import { invoke } from "@tauri-apps/api/core";
import { defaultAppInvokeMock } from "../../../__tests__/appTestFixtures";
import { useLcuEvents } from "../useLcuEvents";
import { mockArenaMasterYiBuild } from "./arenaBuildFixture";

let eventListeners: Record<string, (event: { payload: unknown }) => void> = {};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args?: Record<string, unknown>) => {
    if (cmd === "get_recommended_build_for_champion" && args?.championId === 11) {
      return Promise.resolve(mockArenaMasterYiBuild);
    }
    if (cmd === "update_build_filters" && args?.championId === 11) {
      return Promise.resolve({
        ...mockArenaMasterYiBuild,
        active_patch: (args.patch as string) || "16.22",
      });
    }
    return defaultAppInvokeMock(cmd, args);
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockImplementation((event: string, cb: (payload: unknown) => void) => {
    eventListeners[event] = cb;
    return Promise.resolve(() => {
      delete eventListeners[event];
    });
  }),
}));

describe("App — Arena (CHERRY / 3x6) Champ Select Integration", () => {
  beforeEach(() => {
    eventListeners = {};
    localStorage.clear();
    vi.clearAllMocks();
  });

  it("renders Arena header, augments, synergies, prismatic item blocks, filters by rarity, and injects Arena item sets", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["champ-selected"]).toBeDefined();
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    await act(async () => {
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 11,
          queue_id: 1700,
          queue_mode: "Arena",
          locked_in: true,
          assigned_position: "jungle",
        },
      });
      eventListeners["recommended-build"]({ payload: mockArenaMasterYiBuild });
    });

    await waitFor(() => {
      expect(screen.getByTestId("arena-augments-card")).toBeInTheDocument();
    });

    expect(invoke).toHaveBeenCalledWith("get_recommended_build_for_champion", {
      championId: 11,
      queueId: 1700,
    });

    expect(screen.getByText("Arena (3x6)")).toBeInTheDocument();
    expect(screen.getByText("56.4% Top 3")).toBeInTheDocument();
    expect(screen.getByText("16.8% 1º Lugar")).toBeInTheDocument();
    expect(screen.getByText("Melhores Sinergias")).toBeInTheDocument();
    expect(screen.getByText("2 aliados")).toBeInTheDocument();
    expect(screen.queryByTestId("filter-tier")).not.toBeInTheDocument();
    expect(screen.queryByTestId("filter-position")).not.toBeInTheDocument();
    expect(screen.queryByTestId("filter-region")).not.toBeInTheDocument();
    expect(screen.getByTestId("filter-patch")).toBeInTheDocument();

    expect(screen.getByTestId("arena-mode-notice")).toBeInTheDocument();
    expect(screen.queryByTestId("runes-tree")).not.toBeInTheDocument();
    expect(screen.queryByTestId("spells-bar")).not.toBeInTheDocument();
    expect(screen.getByTestId("arena-augment-101")).toBeInTheDocument();
    expect(screen.getByTestId("arena-augment-102")).toBeInTheDocument();
    expect(screen.getByTestId("arena-augment-103")).toBeInTheDocument();
    expect(screen.getByTestId("arena-synergy-117")).toHaveTextContent("Lulu");
    expect(screen.getByTestId("arena-synergy-350")).toHaveTextContent("Yuumi");

    fireEvent.click(screen.getByTestId("arena-rarity-btn-8"));
    expect(screen.getByTestId("arena-augment-101")).toBeInTheDocument();
    expect(screen.queryByTestId("arena-augment-102")).not.toBeInTheDocument();
    expect(screen.queryByTestId("arena-augment-103")).not.toBeInTheDocument();

    expect(screen.getByText("1. Itens Prismáticos")).toBeInTheDocument();
    expect(screen.getByText("2. Primeiros 3 Itens")).toBeInTheDocument();
    expect(screen.getByText("3. Botas")).toBeInTheDocument();
    expect(screen.getByText("4. Build Final")).toBeInTheDocument();
    expect(screen.getByText("5. Itens Situacionais / Úteis")).toBeInTheDocument();

    await act(async () => {
      fireEvent.click(screen.getByTestId("build-opt-btn-1"));
    });
    expect(invoke).toHaveBeenCalledWith("inject_item_build", {
      championId: 11,
      queueId: 1700,
      buildIndex: 1,
    });
    expect(screen.getByAltText("Item 223072")).toBeInTheDocument();

    const patchSelect = screen.getByTestId("filter-patch");
    await act(async () => {
      fireEvent.click(patchSelect);
    });
    const listbox = screen.getByRole("listbox", { name: "Patch:" });
    await act(async () => {
      fireEvent.click(within(listbox).getByRole("option", { name: "Patch 16.21" }));
    });
    expect(invoke).toHaveBeenCalledWith("update_build_filters", {
      championId: 11,
      queueId: 1700,
      position: null,
      tier: null,
      region: null,
      patch: "16.21",
    });

    expect(invoke).not.toHaveBeenCalledWith("inject_rune_page", expect.anything());
    expect(invoke).not.toHaveBeenCalledWith("inject_summoner_spells", expect.anything());
  });

  it("keeps orderedSpells empty and skips rune/spell injections while preserving queueId 1750 for Arena 3x6 item builds", async () => {
    const { result } = renderHook(() => useLcuEvents("D"));

    await waitFor(() => {
      expect(eventListeners["champ-selected"]).toBeDefined();
    });

    await act(async () => {
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 11,
          queue_id: 1750,
          queue_mode: "Arena",
          locked_in: true,
        },
      });
    });

    await waitFor(() => {
      expect(result.current.build?.champion_id).toBe(11);
    });

    expect(result.current.orderedSpells).toEqual([]);

    await act(async () => {
      await result.current.handleSelectRunePage(0);
      await result.current.handleSelectSpellPair(0);
      await result.current.handleFilterChange("tier", "master_plus");
      await result.current.handleSelectBuild(1);
    });

    expect(invoke).not.toHaveBeenCalledWith("inject_rune_page", expect.anything());
    expect(invoke).not.toHaveBeenCalledWith("inject_summoner_spells", expect.anything());
    expect(invoke).not.toHaveBeenCalledWith("update_build_filters", expect.anything());
    expect(invoke).toHaveBeenCalledWith("inject_item_build", {
      championId: 11,
      queueId: 1750,
      buildIndex: 1,
    });
  });
});

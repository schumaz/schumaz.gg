import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, within } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import App from "../../../App";
import { defaultAppInvokeMock, resetMockAppSettings } from "../../../__tests__/appTestFixtures";
import {
  mockArenaLuluBuild,
  mockArenaMasterYiBuild,
  mockArenaTierResponse,
} from "./arenaFixtures";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

describe("ArenaTab App integration & repeated dock click", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
    resetMockAppSettings();
    (listen as any).mockImplementation(async () => vi.fn());
    (invoke as any).mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "get_arena_tier_list") {
        return mockArenaTierResponse;
      }
      if (cmd === "get_arena_champion_build") {
        if (args?.championId === 117) return mockArenaLuluBuild;
        return mockArenaMasterYiBuild;
      }
      return defaultAppInvokeMock(cmd, args);
    });
  });

  it("loads Arena tier list on dock click, opens detail, jumps via duo synergy, and returns on repeated Arena dock click", async () => {
    render(<App />);

    const arenaDockBtn = await screen.findByRole("button", { name: "Arena" });
    fireEvent.click(arenaDockBtn);

    const arenaView = screen.getByTestId("view-arena");
    expect(arenaView).toHaveClass("active-tab");

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_arena_tier_list", { patch: null });
      expect(within(arenaView).getByTestId("arena-champ-masteryi")).toBeInTheDocument();
    });

    // Open Master Yi detail
    fireEvent.click(within(arenaView).getByTestId("arena-champ-masteryi"));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_arena_champion_build", {
        championId: 11,
        patch: null,
      });
      expect(within(arenaView).getByTestId("arena-champion-detail")).toBeInTheDocument();
      expect(within(arenaView).getByText("Duelo Místico")).toBeInTheDocument();
    });

    // Ensure no LCU injection is triggered from read-only Arena tab consultation
    const injectedCalls = (invoke as any).mock.calls.filter(
      ([cmd]: [string]) =>
        cmd === "inject_item_set" ||
        cmd === "inject_rune_page" ||
        cmd === "inject_summoner_spells"
    );
    expect(injectedCalls).toHaveLength(0);

    // Clicking duo synergy partner Lulu jumps to Lulu's Arena detail
    fireEvent.click(within(arenaView).getByTestId("arena-synergy-117"));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_arena_champion_build", {
        championId: 117,
        patch: null,
      });
      expect(within(arenaView).getByRole("heading", { name: "Lulu" })).toBeInTheDocument();
    });

    // Repeated click on Arena in NavigationDock returns to the tier list
    fireEvent.click(arenaDockBtn);
    await waitFor(() => {
      expect(within(arenaView).getByTestId("arena-tierlist-view")).toBeInTheDocument();
      expect(within(arenaView).queryByTestId("arena-champion-detail")).not.toBeInTheDocument();
    });
  });
});

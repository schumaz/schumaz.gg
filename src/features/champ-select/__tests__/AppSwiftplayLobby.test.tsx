import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import App from "../../../App";
import { invoke } from "@tauri-apps/api/core";
import type { ChampionBuild, ChampSelectedPayload } from "../../../types";
import { defaultAppInvokeMock, mockAhriBuild, resetMockAppSettings } from "../../../__tests__/appTestFixtures";

let eventListeners: Record<string, (event: { payload: unknown }) => void> = {};

const swiftplayPayloadPrimary: ChampSelectedPayload = {
  champion_id: 103,
  queue_id: 480,
  queue_mode: "SummonersRift",
  locked_in: true,
  assigned_position: "mid",
  is_lobby_preselect: true,
  active_slot_index: 0,
  lobby_slots: [
    {
      slot_index: 0,
      champion_id: 103,
      champion_name: "Ahri",
      champion_icon_url: "",
      assigned_position: "mid",
    },
    {
      slot_index: 1,
      champion_id: 222,
      champion_name: "Jinx",
      champion_icon_url: "",
      assigned_position: "adc",
    },
  ],
};

const jinxSwiftplayBuild: ChampionBuild = {
  ...mockAhriBuild,
  champion_id: 222,
  champion_name: "Jinx",
  active_position: "adc",
  spell1_id: 4,
  spell2_id: 21,
  spells: [
    { id: 4, name: "Flash", icon_url: "flash.png" },
    { id: 21, name: "Barreira", icon_url: "barrier.png" },
  ],
};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args?: Record<string, unknown>) => {
    if (cmd === "select_lobby_preselect_slot" && args?.slotIndex === 1) {
      return Promise.resolve({
        gameflow_phase: "Lobby",
        champ_selected: {
          ...swiftplayPayloadPrimary,
          champion_id: 222,
          assigned_position: "adc",
          active_slot_index: 1,
        },
        build: jinxSwiftplayBuild,
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

describe("App — Swiftplay Lobby Pre-Selection (480)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    eventListeners = {};
    resetMockAppSettings();
  });

  it("displays Primary/Secondary lobby slots without duplicate fetch, switches slots, locks spells during Matchmaking, and clears on lobby exit", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["champ-selected"]).toBeDefined();
      expect(eventListeners["recommended-build"]).toBeDefined();
      expect(eventListeners["gameflow-phase-changed"]).toBeDefined();
    });

    await act(async () => {
      eventListeners["gameflow-phase-changed"]({ payload: "Lobby" });
      eventListeners["champ-selected"]({ payload: swiftplayPayloadPrimary });
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    await waitFor(() => {
      expect(screen.getByTestId("lobby-slots-bar")).toBeInTheDocument();
    });

    expect(screen.getByText("Jogo Rápido (Swiftplay)")).toBeInTheDocument();
    expect(screen.getByText("Pré-seleção no Lobby")).toBeInTheDocument();

    // Lobby preselect must NOT trigger redundant build fetch from frontend onChampSelected
    const redundantCalls = vi
      .mocked(invoke)
      .mock.calls.filter(
        ([cmd]) => cmd === "get_recommended_build_for_champion" || cmd === "update_build_filters"
      );
    expect(redundantCalls).toHaveLength(0);

    const slot0 = screen.getByTestId("lobby-slot-btn-0");
    const slot1 = screen.getByTestId("lobby-slot-btn-1");
    expect(slot0).toHaveAttribute("aria-pressed", "true");
    expect(slot1).toHaveAttribute("aria-pressed", "false");

    await act(async () => {
      fireEvent.click(slot1);
    });

    expect(invoke).toHaveBeenCalledWith("select_lobby_preselect_slot", { slotIndex: 1 });
    await waitFor(() => {
      expect(screen.getByRole("heading", { name: "Jinx" })).toBeInTheDocument();
    });
    expect(screen.getByTestId("lobby-slot-btn-1")).toHaveAttribute("aria-pressed", "true");

    // Entering Matchmaking locks spells to read-only while keeping the dashboard visible
    await act(async () => {
      eventListeners["gameflow-phase-changed"]({ payload: "Matchmaking" });
    });
    expect(screen.getByTestId("spell-pair-info-0")).toBeInTheDocument();
    expect(screen.queryByTestId("spell-pair-btn-0")).not.toBeInTheDocument();

    // Leaving Swiftplay lobby clears dashboard back to idle waiting state
    await act(async () => {
      eventListeners["gameflow-phase-changed"]({ payload: "Lobby" });
      eventListeners["champ-select-ended"]({ payload: null });
    });
    expect(screen.queryByTestId("champ-hero")).not.toBeInTheDocument();
  });
});

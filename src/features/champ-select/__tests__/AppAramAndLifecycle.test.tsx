import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import App from "../../../App";
import { invoke } from "@tauri-apps/api/core";
import { defaultAppInvokeMock, mockAhriBuild } from "../../../__tests__/appTestFixtures";

let eventListeners: Record<string, (event: { payload: unknown }) => void> = {};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args?: Record<string, unknown>) =>
    defaultAppInvokeMock(cmd, args)
  ),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockImplementation((event: string, cb: (payload: unknown) => void) => {
    eventListeners[event] = cb;
    return Promise.resolve(() => {
      delete eventListeners[event];
    });
  }),
}));

describe("App — ARAM Modes & Match Lifecycle Navigation", () => {
  beforeEach(() => {
    eventListeners = {};
    localStorage.clear();
  });

  it("hides Rota selector, omits position in filter updates, and forwards active queueId to inject_item_build in ARAM (450)", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["champ-selected"]).toBeDefined();
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    await act(async () => {
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 103,
          queue_id: 450,
          queue_mode: "Aram",
          locked_in: true,
        },
      });
      eventListeners["recommended-build"]({
        payload: {
          ...mockAhriBuild,
          active_position: "",
          supports_runes: true,
        },
      });
    });

    await waitFor(() => {
      expect(screen.getByTestId("champ-filters-bar")).toBeInTheDocument();
    });

    expect(screen.getByText("ARAM (Howling Abyss)")).toBeInTheDocument();
    expect(screen.queryByTestId("filter-position")).not.toBeInTheDocument();

    const tierSelect = screen.getByTestId("filter-tier");
    await act(async () => {
      fireEvent.click(tierSelect);
    });
    await act(async () => {
      fireEvent.click(screen.getByRole("option", { name: "Diamante +" }));
    });

    expect(invoke).toHaveBeenCalledWith("update_build_filters", {
      championId: 103,
      queueId: 450,
      position: null,
      tier: "diamond_plus",
      region: "global",
      patch: "16.22",
    });

    const buildOpt1 = screen.getByTestId("build-opt-btn-1");
    await act(async () => {
      fireEvent.click(buildOpt1);
    });

    expect(invoke).toHaveBeenCalledWith("inject_item_build", {
      championId: 103,
      queueId: 450,
      buildIndex: 1,
    });
  });

  it("hides Rota selector and runes, omits position in filter updates, and forwards queueId 2400 in ARAM Mayhem", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["champ-selected"]).toBeDefined();
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    await act(async () => {
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 103,
          queue_id: 2400,
          queue_mode: "AramMayhem",
          locked_in: true,
        },
      });
      eventListeners["recommended-build"]({
        payload: {
          ...mockAhriBuild,
          active_position: "",
          supports_runes: false,
          rune_pages: [],
        },
      });
    });

    await waitFor(() => {
      expect(screen.getByTestId("champ-filters-bar")).toBeInTheDocument();
    });

    expect(screen.getByText("ARAM Mayhem (Howling Abyss)")).toBeInTheDocument();
    expect(screen.queryByTestId("filter-position")).not.toBeInTheDocument();
    expect(screen.queryByTestId("rune-pages-selector")).not.toBeInTheDocument();

    const tierSelect = screen.getByTestId("filter-tier");
    await act(async () => {
      fireEvent.click(tierSelect);
    });
    await act(async () => {
      fireEvent.click(screen.getByRole("option", { name: "Mestre +" }));
    });

    expect(invoke).toHaveBeenCalledWith("update_build_filters", {
      championId: 103,
      queueId: 2400,
      position: null,
      tier: "master_plus",
      region: "global",
      patch: "16.22",
    });

    const buildOpt1 = screen.getByTestId("build-opt-btn-1");
    await act(async () => {
      fireEvent.click(buildOpt1);
    });

    expect(invoke).toHaveBeenCalledWith("inject_item_build", {
      championId: 103,
      queueId: 2400,
      buildIndex: 1,
    });
  });

  it("automatically navigates to Runas when entering a match and pulling a champion, and returns to Perfil when the match ends", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["champ-selected"]).toBeDefined();
      expect(eventListeners["recommended-build"]).toBeDefined();
      expect(eventListeners["gameflow-phase-changed"]).toBeDefined();
      expect(eventListeners["champ-select-ended"]).toBeDefined();
      expect(eventListeners["match-ended"]).toBeDefined();
    });

    const profileBtn = await screen.findByRole("button", { name: /^perfil$/i });
    const runesBtn = screen.getByRole("button", { name: /^runas$/i });
    const settingsBtn = screen.getByRole("button", { name: /^configurações$/i });

    expect(profileBtn).toHaveAttribute("aria-current", "page");

    await act(async () => {
      eventListeners["gameflow-phase-changed"]({ payload: "ChampSelect" });
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 103,
          queue_id: 420,
          queue_mode: "RankedSolo",
          locked_in: false,
        },
      });
    });

    await waitFor(() => {
      expect(runesBtn).toHaveAttribute("aria-current", "page");
    });

    fireEvent.click(settingsBtn);
    expect(settingsBtn).toHaveAttribute("aria-current", "page");

    await act(async () => {
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 103,
          queue_id: 420,
          queue_mode: "RankedSolo",
          locked_in: true,
        },
      });
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });
    expect(settingsBtn).toHaveAttribute("aria-current", "page");

    fireEvent.click(runesBtn);
    await act(async () => {
      eventListeners["gameflow-phase-changed"]({ payload: "InProgress" });
      eventListeners["champ-select-ended"]({ payload: null });
    });
    expect(screen.getByTestId("champ-hero")).toBeInTheDocument();
    expect(screen.getByTestId("spell-pair-info-0")).toBeInTheDocument();
    expect(screen.getByTestId("invert-spells-readonly")).toBeInTheDocument();
    expect(screen.queryByTestId("spell-pair-btn-0")).not.toBeInTheDocument();
    expect(screen.queryByTestId("invert-spells-btn")).not.toBeInTheDocument();

    await act(async () => {
      eventListeners["gameflow-phase-changed"]({ payload: "EndOfGame" });
      eventListeners["match-ended"]({ payload: null });
    });
    expect(profileBtn).toHaveAttribute("aria-current", "page");
    expect(screen.queryByTestId("champ-hero")).not.toBeInTheDocument();
  });
});

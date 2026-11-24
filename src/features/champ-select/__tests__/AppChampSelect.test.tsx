import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, act, within } from "@testing-library/react";
import App from "../../../App";
import { invoke } from "@tauri-apps/api/core";
import { defaultAppInvokeMock, mockAhriBuild, resetMockAppSettings } from "../../../__tests__/appTestFixtures";

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

describe("App — Champ Select Dashboard, Filters & Spells", () => {
  beforeEach(() => {
    eventListeners = {};
    resetMockAppSettings();
  });

  it("swaps summoner spell slots and supports inverting spells with immediate injection in champion select", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    act(() => {
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    await waitFor(() => {
      expect(screen.getByTestId("spells-card")).toBeInTheDocument();
    });

    expect(screen.queryByTestId("injection-report")).not.toBeInTheDocument();

    const spellItemsBefore = within(screen.getByTestId("spell-pair-btn-0")).getAllByText(/Flash|Incendiar/);
    expect(spellItemsBefore[0]).toHaveTextContent("Flash");
    expect(spellItemsBefore[1]).toHaveTextContent("Incendiar");

    const invertBtn = screen.getByTestId("invert-spells-btn");
    expect(invertBtn).toHaveAccessibleName(/Inverter Feitiços/i);

    await act(async () => {
      fireEvent.click(invertBtn);
    });

    const spellItemsAfter = within(screen.getByTestId("spell-pair-btn-0")).getAllByText(/Flash|Incendiar/);
    expect(spellItemsAfter[0]).toHaveTextContent("Incendiar");
    expect(spellItemsAfter[1]).toHaveTextContent("Flash");

    expect(invoke).toHaveBeenCalledWith("inject_summoner_spells", {
      spell1Id: 14,
      spell2Id: 4,
      championId: null,
    });

    expect(invoke).toHaveBeenCalledWith("sync_user_preferences", {
      preferences: expect.objectContaining({ flashKey: "F" }),
    });

    fireEvent.click(screen.getByRole("button", { name: /^configurações$/i }));
    const flashDBtn = screen.getByRole("button", { name: "D" });
    await act(async () => {
      fireEvent.click(flashDBtn);
    });

    expect(invoke).toHaveBeenCalledWith("inject_summoner_spells", {
      spell1Id: 4,
      spell2Id: 14,
      championId: null,
    });
  });

  it("renders full match dashboard when recommended build is received", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    act(() => {
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    await waitFor(() => {
      expect(screen.getByTestId("champ-hero")).toBeInTheDocument();
    });

    expect(screen.getByText("Ahri")).toBeInTheDocument();
    const tierBadge = screen.getByTestId("champ-tier-badge");
    expect(tierBadge).toHaveTextContent("OP");
    expect(tierBadge).toHaveClass("aram-tier-pill", "tier-op");
    expect(screen.queryByText(/Win Rate/i)).not.toBeInTheDocument();
    expect(screen.getByText(/8.4% Pick Rate/i)).toBeInTheDocument();
    expect(screen.queryByTestId("reapply-build-btn")).not.toBeInTheDocument();

    expect(screen.getByText("Dominação")).toBeInTheDocument();
    expect(screen.getByText("Eletrocutar")).toBeInTheDocument();
    expect(screen.getByText("Feitiçaria")).toBeInTheDocument();
    expect(screen.getByText("Fragmentos de Atributo")).toBeInTheDocument();

    expect(within(screen.getByTestId("spell-pair-btn-0")).getByText("Flash")).toBeInTheDocument();
    expect(within(screen.getByTestId("spell-pair-btn-0")).getByText("Incendiar")).toBeInTheDocument();
    expect(screen.getByTestId("skill-order-trigger")).toBeInTheDocument();
    expect(screen.getByTestId("skill-priority-chain")).toHaveTextContent(/Q.*>.*W.*>.*E/);
    expect(screen.queryByTestId("skill-order-popover")).not.toBeInTheDocument();
    fireEvent.click(screen.getByTestId("skill-order-trigger"));
    expect(screen.getByTestId("skill-order-card")).toBeInTheDocument();
  });

  it("renders the 4 filter selectors (Elo, Rota, Servidor, Patch) and handles filter updates", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    act(() => {
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    await waitFor(() => {
      expect(screen.getByTestId("champ-filters-bar")).toBeInTheDocument();
    });

    const tierSelect = screen.getByTestId("filter-tier");
    const posSelect = screen.getByTestId("filter-position");
    const regionSelect = screen.getByTestId("filter-region");
    const patchSelect = screen.getByTestId("filter-patch");

    expect(tierSelect).toHaveAttribute("data-value", "emerald_plus");
    expect(posSelect).toHaveAttribute("data-value", "mid");
    expect(regionSelect).toHaveAttribute("data-value", "global");
    expect(patchSelect).toHaveAttribute("data-value", "16.22");

    await act(async () => {
      fireEvent.click(posSelect);
    });
    await act(async () => {
      fireEvent.click(screen.getByRole("option", { name: "Atirador (Bot)" }));
    });

    expect(invoke).toHaveBeenCalledWith("update_build_filters", {
      championId: 103,
      queueId: 420,
      position: "adc",
      tier: "emerald_plus",
      region: "global",
      patch: "16.22",
    });

    await act(async () => {
      fireEvent.click(tierSelect);
    });
    await act(async () => {
      fireEvent.click(screen.getByRole("option", { name: "Diamante +" }));
    });

    expect(invoke).toHaveBeenCalledWith("update_build_filters", expect.objectContaining({
      championId: 103,
      tier: "diamond_plus",
    }));
  });

  it("displays the patch fallback badge when latest patch is less than 48 hours old", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    act(() => {
      eventListeners["recommended-build"]({
        payload: {
          ...mockAhriBuild,
          tier: null,
          active_patch: "16.21",
          patch_fallback_applied: true,
        },
      });
    });

    await waitFor(() => {
      expect(screen.getByTestId("patch-fallback-badge")).toBeInTheDocument();
    });

    expect(screen.getByText(/Volume Protegido \(16\.21\)/i)).toBeInTheDocument();
    expect(screen.queryByTestId("champ-tier-badge")).not.toBeInTheDocument();
  });

  it("displays Practice Tool badge and allows toggling unified spell pair", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["champ-selected"]).toBeDefined();
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    await act(async () => {
      eventListeners["champ-selected"]({
        payload: { champion_id: 103, queue_id: -1, queue_mode: "PracticeTool", locked_in: true },
      });
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    await waitFor(() => {
      expect(screen.getByTestId("spell-pair-selector")).toBeInTheDocument();
    });

    expect(screen.getByText("Modo Treino (Build Ranqueada)")).toBeInTheDocument();

    const pair0Btn = screen.getByTestId("spell-pair-btn-0");
    const pair1Btn = screen.getByTestId("spell-pair-btn-1");
    expect(pair0Btn).toHaveClass("active");

    await act(async () => {
      fireEvent.click(pair1Btn);
    });

    expect(pair1Btn).toHaveClass("active");
    expect(screen.getByText("Teleporte")).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("inject_summoner_spells", {
      spell1Id: 4,
      spell2Id: 12,
      championId: 103,
    });
  });
});

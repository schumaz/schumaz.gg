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

describe("App — Rune Pages & Unified Build Options", () => {
  beforeEach(() => {
    eventListeners = {};
    localStorage.clear();
  });

  it("renders rune page selector with 3 most played options, displays keystone/sub-style and game count, and allows switching rune pages with LCU injection", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    act(() => {
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    await waitFor(() => {
      expect(screen.getByTestId("rune-pages-selector")).toBeInTheDocument();
    });

    const opt0 = screen.getByTestId("rune-page-btn-0");
    const opt1 = screen.getByTestId("rune-page-btn-1");
    const opt2 = screen.getByTestId("rune-page-btn-2");

    expect(opt0).toHaveClass("active");
    expect(opt1).not.toHaveClass("active");
    expect(opt2).not.toHaveClass("active");

    expect(screen.getByText("Opção 1 (Mais Usada)")).toBeInTheDocument();
    expect(opt0).toHaveTextContent(/45[.,]0k/i);
    expect(opt0).toHaveAttribute("title", expect.stringMatching(/45[.,]000 partidas/i));
    expect(screen.getByText("Eletrocutar")).toBeInTheDocument();

    act(() => {
      fireEvent.click(opt1);
    });

    expect(opt1).toHaveClass("active");
    expect(screen.getByText("Aprimoramento Glacial")).toBeInTheDocument();

    expect(invoke).toHaveBeenCalledWith("inject_rune_page", {
      championId: 103,
      championName: "Ahri",
      pageIndex: 1,
      primaryStyleId: 8300,
      subStyleId: 8100,
      selectedPerkIds: [8351, 8304, 8345, 8352, 8126, 8135, 5008, 5008, 5001],
    });
  });

  it("persists user manual rune page and secondary spell selections across draft lock-in for the same champion", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["champ-selected"]).toBeDefined();
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    act(() => {
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 103,
          queue_id: 420,
          queue_mode: "RankedSolo",
          locked_in: false,
        },
      });
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    await waitFor(() => {
      expect(screen.getByTestId("sec-spell-btn-12")).toBeInTheDocument();
      expect(screen.getByTestId("rune-page-btn-1")).toBeInTheDocument();
    });

    await act(async () => {
      fireEvent.click(screen.getByTestId("sec-spell-btn-12"));
      fireEvent.click(screen.getByTestId("rune-page-btn-1"));
    });

    expect(screen.getByTestId("sec-spell-btn-12")).toHaveClass("active");
    expect(screen.getByTestId("rune-page-btn-1")).toHaveClass("active");

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

    expect(screen.getByTestId("sec-spell-btn-12")).toHaveClass("active");
    expect(screen.getByTestId("rune-page-btn-1")).toHaveClass("active");
    expect(screen.getByText("Aprimoramento Glacial")).toBeInTheDocument();

    await act(async () => {
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 804,
          queue_id: 420,
          queue_mode: "RankedSolo",
          locked_in: false,
        },
      });
      eventListeners["recommended-build"]({
        payload: {
          ...mockAhriBuild,
          champion_id: 804,
          champion_name: "Yunara",
        },
      });
    });

    expect(screen.getByTestId("rune-page-btn-0")).toHaveClass("active");
    expect(screen.getByTestId("rune-page-btn-1")).not.toHaveClass("active");
  });

  it("renders unified build selector, injects item build, updates situational items, and persists choice across lock-in", async () => {
    render(<App />);

    await waitFor(() => {
      expect(eventListeners["champ-selected"]).toBeDefined();
      expect(eventListeners["recommended-build"]).toBeDefined();
    });

    await act(async () => {
      eventListeners["champ-selected"]({
        payload: {
          champion_id: 103,
          queue_id: 420,
          queue_mode: "RankedSolo",
          locked_in: false,
        },
      });
      eventListeners["recommended-build"]({ payload: mockAhriBuild });
    });

    await waitFor(() => {
      expect(screen.getByTestId("unified-build-selector")).toBeInTheDocument();
    });

    const buildOpt0 = screen.getByTestId("build-opt-btn-0");
    const buildOpt1 = screen.getByTestId("build-opt-btn-1");
    expect(buildOpt0).toHaveClass("active");
    expect(buildOpt0.querySelectorAll(".build-pill-mini-icon")).toHaveLength(3);
    expect(screen.queryByAltText("Item 3116")).not.toBeInTheDocument();

    await act(async () => {
      fireEvent.click(buildOpt1);
    });

    expect(buildOpt1).toHaveClass("active");
    expect(screen.getByAltText("Item 3116")).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("inject_item_build", {
      championId: 103,
      queueId: 420,
      buildIndex: 1,
    });

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

    expect(buildOpt1).toHaveClass("active");
  });
});

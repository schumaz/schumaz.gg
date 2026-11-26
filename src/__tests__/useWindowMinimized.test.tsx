import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, renderHook, act, waitFor } from "@testing-library/react";
import { NavigationDock } from "../components/ui/NavigationDock";
import {
  resolveWindowGraphicsMode,
  useWindowGraphicsMode,
  useWindowMinimized,
} from "../hooks/useWindowMinimized";

const listeners: Record<string, (event: { payload: boolean }) => void> = {};

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockImplementation((event: string, cb: (payload: { payload: boolean }) => void) => {
    listeners[event] = cb;
    return Promise.resolve(() => {
      delete listeners[event];
    });
  }),
}));

describe("useWindowGraphicsMode, useWindowMinimized, and NavigationDock animation suspension", () => {
  const originalVisibilityDescriptor = Object.getOwnPropertyDescriptor(
    document,
    "visibilityState"
  );
  const originalHasFocus = document.hasFocus;

  const setDocumentVisibility = (state: DocumentVisibilityState) => {
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      get: () => state,
    });
    document.dispatchEvent(new Event("visibilitychange"));
  };

  beforeEach(() => {
    Object.keys(listeners).forEach((k) => delete listeners[k]);
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      get: () => "visible",
    });
    document.hasFocus = vi.fn(() => true);
  });

  afterEach(() => {
    if (originalVisibilityDescriptor) {
      Object.defineProperty(document, "visibilityState", originalVisibilityDescriptor);
    } else {
      delete (document as unknown as Record<string, unknown>).visibilityState;
    }
    document.hasFocus = originalHasFocus;
  });

  it("separates focused, unfocused side-by-side, and zero-gpu states across events and gameflow phases", async () => {
    const { result, rerender, unmount } = renderHook(
      ({ phase }: { phase?: string }) => useWindowGraphicsMode(phase),
      { initialProps: { phase: "Lobby" } }
    );
    expect(result.current.mode).toBe("focused");
    expect(result.current.isZeroGpu).toBe(false);
    expect(result.current.animationsPaused).toBe(false);
    expect(result.current.shellClassName).toBe("");

    await waitFor(() => {
      expect(listeners["window-minimized-changed"]).toBeDefined();
    });

    // Side-by-side focus loss enters "unfocused" (pauses decorative dock beam, keeps glass/spinners)
    act(() => {
      window.dispatchEvent(new Event("blur"));
    });
    expect(result.current.mode).toBe("unfocused");
    expect(result.current.isZeroGpu).toBe(false);
    expect(result.current.animationsPaused).toBe(true);
    expect(result.current.shellClassName).toBe("app-unfocused");

    // Focused back
    act(() => {
      window.dispatchEvent(new Event("focus"));
    });
    expect(result.current.mode).toBe("focused");
    expect(result.current.shellClassName).toBe("");

    // Minimized via Tauri window event -> zero-gpu
    act(() => {
      listeners["window-minimized-changed"]({ payload: true });
    });
    expect(result.current.mode).toBe("zero-gpu");
    expect(result.current.isZeroGpu).toBe(true);
    expect(result.current.shellClassName).toBe("app-minimized");

    // Restored via Tauri window event -> focused
    act(() => {
      listeners["window-minimized-changed"]({ payload: false });
    });
    expect(result.current.mode).toBe("focused");

    // Hidden via document visibilitychange -> zero-gpu
    act(() => {
      setDocumentVisibility("hidden");
    });
    expect(result.current.mode).toBe("zero-gpu");

    act(() => {
      setDocumentVisibility("visible");
    });
    expect(result.current.mode).toBe("focused");

    // Active game phases -> zero-gpu even while visible and focused
    for (const phase of ["GameStart", "InProgress", "Reconnect"]) {
      rerender({ phase });
      expect(result.current.mode).toBe("zero-gpu");
      expect(result.current.isInGame).toBe(true);
      expect(result.current.isZeroGpu).toBe(true);
    }

    rerender({ phase: "WaitingForStats" });
    expect(result.current.mode).toBe("focused");
    expect(result.current.isInGame).toBe(false);

    unmount();
    expect(listeners["window-minimized-changed"]).toBeUndefined();
  });

  it("keeps useWindowMinimized false when merely unfocused side-by-side", () => {
    document.hasFocus = vi.fn(() => false);
    const { result, rerender } = renderHook(
      ({ phase }: { phase?: string }) => useWindowMinimized(phase),
      { initialProps: { phase: "ChampSelect" } }
    );
    expect(result.current).toBe(false);

    rerender({ phase: "InProgress" });
    expect(result.current).toBe(true);

    expect(
      resolveWindowGraphicsMode({
        isDocumentHidden: false,
        isWindowMinimized: false,
        isWindowUnfocused: true,
        gameflowPhase: "ChampSelect",
      })
    ).toBe("unfocused");
  });

  it("does not render dock beam animation nodes when animationsPaused is true", () => {
    const { container, rerender } = render(
      <NavigationDock activeTab="profile" onTabChange={vi.fn()} animationsPaused={false} />
    );
    expect(container.querySelector(".dock-beam-wrap")).toBeInTheDocument();
    expect(container.querySelector(".dock-beam-glow")).toBeInTheDocument();

    rerender(
      <NavigationDock activeTab="profile" onTabChange={vi.fn()} animationsPaused={true} />
    );
    expect(container.querySelector(".dock-beam-wrap")).not.toBeInTheDocument();
    expect(container.querySelector(".dock-beam-glow")).not.toBeInTheDocument();
  });
});

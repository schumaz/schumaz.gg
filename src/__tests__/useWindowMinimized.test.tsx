import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, renderHook, act, waitFor } from "@testing-library/react";
import { NavigationDock } from "../components/ui/NavigationDock";
import {
  resolveWindowGraphicsMode,
  useWindowGraphicsMode,
  useWindowMinimized,
} from "../hooks/useWindowMinimized";

const listeners: Record<string, (event: { payload: boolean }) => void> = {};
const mockIsFocused = vi.fn(async () => true);
const mockIsMinimized = vi.fn(async () => false);
const mockFocusUnlisten = vi.fn();
let focusChangedHandler: ((event: { payload: boolean }) => void) | undefined;

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockImplementation((event: string, cb: (payload: { payload: boolean }) => void) => {
    listeners[event] = cb;
    return Promise.resolve(() => {
      delete listeners[event];
    });
  }),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    isFocused: () => mockIsFocused(),
    isMinimized: () => mockIsMinimized(),
    onFocusChanged: vi.fn().mockImplementation((cb: (event: { payload: boolean }) => void) => {
      focusChangedHandler = cb;
      return Promise.resolve(() => {
        focusChangedHandler = undefined;
        mockFocusUnlisten();
      });
    }),
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
    focusChangedHandler = undefined;
    mockIsFocused.mockReset().mockResolvedValue(true);
    mockIsMinimized.mockReset().mockResolvedValue(false);
    mockFocusUnlisten.mockReset();
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      get: () => "visible",
    });
    document.hasFocus = vi.fn(() => true);
  });

  afterEach(() => {
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
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

  it("renders NavigationDock with static styling and zero dock beam animation nodes", () => {
    const { container } = render(
      <NavigationDock activeTab="profile" onTabChange={vi.fn()} />
    );
    expect(container.querySelector(".floating-dock")).toBeInTheDocument();
    expect(container.querySelector(".dock-beam-wrap")).not.toBeInTheDocument();
    expect(container.querySelector(".dock-beam-glow")).not.toBeInTheDocument();
  });

  it("syncs unfocused mode from native Tauri onFocusChanged and initial isFocused state", async () => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    mockIsFocused.mockResolvedValueOnce(false);
    mockIsMinimized.mockResolvedValueOnce(false);

    const { result, unmount } = renderHook(() => useWindowGraphicsMode("Lobby"));

    await waitFor(() => {
      expect(result.current.mode).toBe("unfocused");
    });
    expect(result.current.animationsPaused).toBe(true);
    expect(result.current.shellClassName).toBe("app-unfocused");
    expect(focusChangedHandler).toBeDefined();

    act(() => {
      focusChangedHandler?.({ payload: true });
    });
    expect(result.current.mode).toBe("focused");
    expect(result.current.animationsPaused).toBe(false);
    expect(result.current.shellClassName).toBe("");

    act(() => {
      focusChangedHandler?.({ payload: false });
    });
    expect(result.current.mode).toBe("unfocused");
    expect(result.current.animationsPaused).toBe(true);
    expect(result.current.shellClassName).toBe("app-unfocused");

    unmount();
    expect(mockFocusUnlisten).toHaveBeenCalledTimes(1);
  });
});

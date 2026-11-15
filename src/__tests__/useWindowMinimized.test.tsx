import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, renderHook, act, waitFor } from "@testing-library/react";
import { NavigationDock } from "../components/NavigationDock";
import { useWindowMinimized } from "../hooks/useWindowMinimized";

const listeners: Record<string, (event: { payload: boolean }) => void> = {};

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockImplementation((event: string, cb: (payload: { payload: boolean }) => void) => {
    listeners[event] = cb;
    return Promise.resolve(() => {
      delete listeners[event];
    });
  }),
}));

describe("useWindowMinimized and NavigationDock animation suspension", () => {
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

  it("tracks Tauri window-minimized-changed events, document visibilityState, and window focus/blur", async () => {
    const { result, unmount } = renderHook(() => useWindowMinimized());
    expect(result.current).toBe(false);

    await waitFor(() => {
      expect(listeners["window-minimized-changed"]).toBeDefined();
    });

    // Minimized or unfocused via Tauri window event
    act(() => {
      listeners["window-minimized-changed"]({ payload: true });
    });
    expect(result.current).toBe(true);

    // Restored and focused via Tauri window event
    act(() => {
      listeners["window-minimized-changed"]({ payload: false });
    });
    expect(result.current).toBe(false);

    // Hidden via document visibilitychange
    act(() => {
      setDocumentVisibility("hidden");
    });
    expect(result.current).toBe(true);

    // Visible again
    act(() => {
      setDocumentVisibility("visible");
    });
    expect(result.current).toBe(false);

    // Unfocused via clicking another app (even while visible on another monitor)
    act(() => {
      window.dispatchEvent(new Event("blur"));
    });
    expect(result.current).toBe(true);

    // Focused back
    act(() => {
      window.dispatchEvent(new Event("focus"));
    });
    expect(result.current).toBe(false);

    unmount();
    expect(listeners["window-minimized-changed"]).toBeUndefined();
  });

  it("initializes as paused when document starts without focus", () => {
    document.hasFocus = vi.fn(() => false);
    const { result } = renderHook(() => useWindowMinimized());
    expect(result.current).toBe(true);

    act(() => {
      window.dispatchEvent(new Event("focus"));
    });
    expect(result.current).toBe(false);
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

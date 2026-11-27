import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, renderHook, screen } from "@testing-library/react";
import App from "../App";
import { defaultAppInvokeMock, resetMockAppSettings } from "./appTestFixtures";
import {
  TAB_TRANSITION_CLEANUP_MS,
  useTabTransition,
} from "../hooks/useTabTransition";
import {
  SUBVIEW_MOTION_CLEANUP_MS,
  useSubviewTransition,
} from "../hooks/useSubviewTransition";
import type { ActiveTab } from "../types";
import pageTransitionsCss from "../styles/page-transitions.css?raw";
import tokensShellCss from "../styles/tokens-shell.css?raw";
import profileTabCss from "../features/profile/ProfileTab.css?raw";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args?: Record<string, unknown>) =>
    defaultAppInvokeMock(cmd, args)
  ),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => Promise.resolve()),
}));

describe("Page & Subview Transitions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetMockAppSettings();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("tracks synchronous tab enter/exit direction, cancels stale timers on rapid switch, and respects disabled", () => {
    vi.useFakeTimers();
    const { result, rerender } = renderHook(
      ({ tab, disabled = false }: { tab: ActiveTab; disabled?: boolean }) =>
        useTabTransition(tab, disabled),
      { initialProps: { tab: "profile" as ActiveTab, disabled: false } }
    );

    expect(result.current).toEqual({
      enteringTab: null,
      exitingTab: null,
      direction: "forward",
    });

    rerender({ tab: "ranked", disabled: false });
    expect(result.current).toEqual({
      enteringTab: "ranked",
      exitingTab: "profile",
      direction: "forward",
    });

    act(() => {
      vi.advanceTimersByTime(100);
    });
    rerender({ tab: "runes", disabled: false });
    expect(result.current).toEqual({
      enteringTab: "runes",
      exitingTab: "ranked",
      direction: "backward",
    });

    act(() => {
      vi.advanceTimersByTime(TAB_TRANSITION_CLEANUP_MS - 1);
    });
    expect(result.current.enteringTab).toBe("runes");

    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(result.current).toEqual({
      enteringTab: null,
      exitingTab: null,
      direction: "backward",
    });

    rerender({ tab: "settings", disabled: false });
    expect(result.current.enteringTab).toBe("settings");
    rerender({ tab: "settings", disabled: true });
    expect(result.current).toEqual({
      enteringTab: null,
      exitingTab: null,
      direction: "forward",
    });
  });

  it("animates intra-tab subviews only when the tab remains active and clears after completion", () => {
    vi.useFakeTimers();
    const { result, rerender } = renderHook(
      ({
        viewKey,
        rank,
        active,
        axis = "depth",
      }: {
        viewKey: string;
        rank: number;
        active: boolean;
        axis?: "depth" | "horizontal";
      }) => useSubviewTransition(viewKey, rank, active, axis),
      { initialProps: { viewKey: "list", rank: 0, active: false, axis: "depth" } }
    );

    expect(result.current).toBe("");

    // Cross-tab jump that activates tab and sets detail simultaneously must not double-animate
    rerender({ viewKey: "detail:103", rank: 1, active: true, axis: "depth" });
    expect(result.current).toBe("");

    rerender({ viewKey: "list", rank: 0, active: true, axis: "depth" });
    expect(result.current).toBe("subview-enter-back");

    act(() => {
      vi.advanceTimersByTime(SUBVIEW_MOTION_CLEANUP_MS);
    });
    expect(result.current).toBe("");

    rerender({ viewKey: "detail:103", rank: 1, active: true, axis: "depth" });
    expect(result.current).toBe("subview-enter-forward");

    rerender({ viewKey: "augments", rank: 0, active: true, axis: "horizontal" });
    expect(result.current).toBe("subview-enter-swap-back");

    rerender({ viewKey: "build", rank: 1, active: true, axis: "horizontal" });
    expect(result.current).toBe("subview-enter-swap-forward");
  });

  it("synchronously sets and cleans up .tabs-viewport transition attributes in App", async () => {
    vi.useFakeTimers();
    render(<App />);

    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

    const viewport = document.querySelector(".tabs-viewport") as HTMLElement;
    expect(viewport).not.toBeNull();
    expect(viewport).not.toHaveAttribute("data-entering-tab");
    expect(viewport).not.toHaveAttribute("data-exiting-tab");

    fireEvent.click(screen.getByRole("button", { name: /^ranqueada$/i }));
    expect(viewport).toHaveAttribute("data-entering-tab", "ranked");
    expect(viewport).toHaveAttribute("data-exiting-tab", "profile");
    expect(viewport).toHaveAttribute("data-transition-dir", "forward");

    fireEvent.click(screen.getByRole("button", { name: /^runas$/i }));
    expect(viewport).toHaveAttribute("data-entering-tab", "runes");
    expect(viewport).toHaveAttribute("data-exiting-tab", "ranked");
    expect(viewport).toHaveAttribute("data-transition-dir", "backward");

    act(() => {
      vi.advanceTimersByTime(TAB_TRANSITION_CLEANUP_MS);
    });
    expect(viewport).not.toHaveAttribute("data-entering-tab");
    expect(viewport).not.toHaveAttribute("data-exiting-tab");
    expect(viewport).not.toHaveAttribute("data-transition-dir");
  });

  it("enforces compositor-only keyframes, parked content-visibility, software fallback, and reduced motion", () => {
    const keyframeBlocks = pageTransitionsCss.match(/@keyframes\s+\w+\s*\{[\s\S]*?\n\}/g) ?? [];
    expect(keyframeBlocks.length).toBeGreaterThanOrEqual(8);
    for (const block of keyframeBlocks) {
      expect(block).not.toMatch(/\b(left|top|width|height|filter|backdrop-filter)\s*:/);
    }

    expect(tokensShellCss).toContain('data-exiting-tab="profile"');
    expect(tokensShellCss).not.toContain("data-fading-tab");
    expect(tokensShellCss).toMatch(/content-visibility:\s*hidden/);
    expect(tokensShellCss).toMatch(/contain-intrinsic-block-size:\s*600px/);

    expect(pageTransitionsCss).toMatch(
      /\.gpu-effects-disabled\s+\.tabs-viewport\[data-transition-dir\]\[data-entering-tab\][^}]*animation:\s*tabFadeEnter/
    );
    expect(pageTransitionsCss).toMatch(
      /\.gpu-effects-disabled\s+\.tabs-viewport\[data-transition-dir\]\[data-exiting-tab\][^}]*animation:\s*tabFadeExit/
    );
    expect(pageTransitionsCss).toMatch(
      /@media\s*\(prefers-reduced-motion:\s*reduce\)\s*\{[\s\S]*animation:\s*none\s*!important;\s*transition:\s*none\s*!important;/
    );

    expect(profileTabCss).toMatch(
      /\.profile-wallpaper-backdrop\s*\{[\s\S]*transition:\s*opacity\s+var\(--motion-wallpaper-enter-duration/
    );
  });
});

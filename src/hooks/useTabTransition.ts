import { useEffect, useMemo, useState } from "react";
import type { ActiveTab } from "../types";

export type TabTransitionDirection = "forward" | "backward";

export const TAB_ORDER: Record<ActiveTab, number> = {
  profile: 0,
  runes: 1,
  ranked: 2,
  arena: 3,
  aram: 4,
  settings: 5,
};

export const TAB_TRANSITION_CLEANUP_MS = 260;

export interface TabTransitionState {
  enteringTab: ActiveTab | null;
  exitingTab: ActiveTab | null;
  direction: TabTransitionDirection;
}

interface InternalTabTransitionState extends TabTransitionState {
  currentTab: ActiveTab;
  generation: number;
}

/**
 * Resolves the directional axis between two top-level navigation dock tabs.
 */
export function getTabTransitionDirection(
  from: ActiveTab,
  to: ActiveTab
): TabTransitionDirection {
  return TAB_ORDER[to] >= TAB_ORDER[from] ? "forward" : "backward";
}

/**
 * Tracks synchronous tab transitions so `.tabs-viewport` commits entering,
 * exiting, and directional attributes in the exact same render as `activeTab`.
 */
export function useTabTransition(
  activeTab: ActiveTab,
  disabled = false
): TabTransitionState {
  const [state, setState] = useState<InternalTabTransitionState>(() => ({
    currentTab: activeTab,
    enteringTab: null,
    exitingTab: null,
    direction: "forward",
    generation: 0,
  }));

  if (state.currentTab !== activeTab) {
    setState((prev) => ({
      currentTab: activeTab,
      enteringTab: disabled ? null : activeTab,
      exitingTab: disabled ? null : prev.currentTab,
      direction: getTabTransitionDirection(prev.currentTab, activeTab),
      generation: prev.generation + 1,
    }));
  } else if (disabled && (state.enteringTab !== null || state.exitingTab !== null)) {
    setState((prev) => ({
      ...prev,
      enteringTab: null,
      exitingTab: null,
    }));
  }

  useEffect(() => {
    if (disabled || (state.enteringTab === null && state.exitingTab === null)) {
      return;
    }

    const activeGeneration = state.generation;
    const timer = window.setTimeout(() => {
      setState((prev) =>
        prev.generation === activeGeneration
          ? { ...prev, enteringTab: null, exitingTab: null }
          : prev
      );
    }, TAB_TRANSITION_CLEANUP_MS);

    return () => window.clearTimeout(timer);
  }, [disabled, state.generation, state.enteringTab, state.exitingTab]);

  return useMemo(
    () => ({
      enteringTab: disabled ? null : state.enteringTab,
      exitingTab: disabled ? null : state.exitingTab,
      direction: state.direction,
    }),
    [disabled, state.enteringTab, state.exitingTab, state.direction]
  );
}

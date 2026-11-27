import { useEffect, useState } from "react";

export type SubviewMotionKind =
  | "none"
  | "drill-in"
  | "drill-out"
  | "swap-forward"
  | "swap-back";

export const SUBVIEW_MOTION_CLEANUP_MS = 230;

interface InternalSubviewState {
  key: string;
  rank: number;
  wasTabActive: boolean;
  kind: SubviewMotionKind;
  generation: number;
}

/**
 * Maps a subview transition kind to its corresponding CSS animation class.
 */
export function resolveSubviewMotionClass(kind: SubviewMotionKind): string {
  switch (kind) {
    case "drill-in":
      return "subview-enter-forward";
    case "drill-out":
      return "subview-enter-back";
    case "swap-forward":
      return "subview-enter-swap-forward";
    case "swap-back":
      return "subview-enter-swap-back";
    case "none":
      return "";
  }
}

/**
 * Synchronously tracks intra-tab navigation changes and returns an entry
 * motion class only while the owning tab remains continuously active.
 */
export function useSubviewTransition(
  viewKey: string,
  viewRank: number,
  isTabActive: boolean,
  axis: "depth" | "horizontal" = "depth"
): string {
  const [state, setState] = useState<InternalSubviewState>(() => ({
    key: viewKey,
    rank: viewRank,
    wasTabActive: isTabActive,
    kind: "none",
    generation: 0,
  }));

  if (state.key !== viewKey || state.rank !== viewRank || state.wasTabActive !== isTabActive) {
    setState((prev) => {
      const keyChanged = prev.key !== viewKey;
      const canAnimate = keyChanged && prev.wasTabActive && isTabActive;
      let nextKind: SubviewMotionKind = !isTabActive ? "none" : prev.kind;

      if (canAnimate) {
        if (axis === "horizontal") {
          nextKind = viewRank >= prev.rank ? "swap-forward" : "swap-back";
        } else {
          nextKind = viewRank >= prev.rank ? "drill-in" : "drill-out";
        }
      }

      return {
        key: viewKey,
        rank: viewRank,
        wasTabActive: isTabActive,
        kind: nextKind,
        generation: canAnimate ? prev.generation + 1 : prev.generation,
      };
    });
  }

  useEffect(() => {
    if (state.kind === "none") {
      return;
    }

    const activeGeneration = state.generation;
    const timer = window.setTimeout(() => {
      setState((prev) =>
        prev.generation === activeGeneration ? { ...prev, kind: "none" } : prev
      );
    }, SUBVIEW_MOTION_CLEANUP_MS);

    return () => window.clearTimeout(timer);
  }, [state.kind, state.generation]);

  return isTabActive ? resolveSubviewMotionClass(state.kind) : "";
}

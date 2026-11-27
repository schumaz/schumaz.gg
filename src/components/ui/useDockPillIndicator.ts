import { RefObject, useLayoutEffect, useRef, useState } from "react";
import type { ActiveTab } from "../../types";

export interface DockPillRect {
  left: number;
  top: number;
  width: number;
  height: number;
  visible: boolean;
  animated: boolean;
}

const HIDDEN_PILL_RECT: DockPillRect = {
  left: 0,
  top: 0,
  width: 0,
  height: 0,
  visible: false,
  animated: false,
};

function readActiveButtonRect(nav: HTMLElement | null): {
  left: number;
  top: number;
  width: number;
  height: number;
} | null {
  if (!nav) return null;
  const activeBtn = nav.querySelector<HTMLButtonElement>('button[aria-current="page"]');
  if (!activeBtn) return null;

  const width = activeBtn.offsetWidth;
  const height = activeBtn.offsetHeight;
  if (width <= 0 || height <= 0) {
    return null;
  }

  return {
    left: activeBtn.offsetLeft,
    top: activeBtn.offsetTop,
    width,
    height,
  };
}

/**
 * Measures the active dock button in a single layout-read pass so the sliding
 * pill indicator tracks tab selection without startup fly-in or resize wobble.
 */
export function useDockPillIndicator(
  navRef: RefObject<HTMLElement | null>,
  activeTab: ActiveTab,
  locale: string
): DockPillRect {
  const [rect, setRect] = useState<DockPillRect>(HIDDEN_PILL_RECT);
  const prevTabRef = useRef<ActiveTab>(activeTab);
  const hasMeasuredRef = useRef(false);

  useLayoutEffect(() => {
    const tabChanged = prevTabRef.current !== activeTab;
    const measured = readActiveButtonRect(navRef.current);

    if (!measured) {
      if (tabChanged) {
        prevTabRef.current = activeTab;
      }
      setRect((prev) => (prev.visible ? HIDDEN_PILL_RECT : prev));
      return;
    }

    const shouldAnimate = hasMeasuredRef.current && tabChanged;
    hasMeasuredRef.current = true;
    prevTabRef.current = activeTab;

    setRect((prev) =>
      prev.visible &&
      prev.left === measured.left &&
      prev.top === measured.top &&
      prev.width === measured.width &&
      prev.height === measured.height &&
      prev.animated === shouldAnimate
        ? prev
        : { ...measured, visible: true, animated: shouldAnimate }
    );
  }, [navRef, activeTab, locale]);

  useLayoutEffect(() => {
    const nav = navRef.current;
    if (!nav || typeof ResizeObserver === "undefined") {
      return;
    }

    const observer = new ResizeObserver(() => {
      const measured = readActiveButtonRect(navRef.current);
      if (!measured) {
        setRect((prev) => (prev.visible ? HIDDEN_PILL_RECT : prev));
        return;
      }

      hasMeasuredRef.current = true;
      setRect((prev) =>
        prev.visible &&
        prev.left === measured.left &&
        prev.top === measured.top &&
        prev.width === measured.width &&
        prev.height === measured.height &&
        !prev.animated
          ? prev
          : { ...measured, visible: true, animated: false }
      );
    });

    observer.observe(nav);
    return () => observer.disconnect();
  }, [navRef]);

  return rect;
}

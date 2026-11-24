export type TooltipPlacement = "top" | "bottom" | "left" | "right";

export interface RectLike {
  top: number;
  left: number;
  right: number;
  bottom: number;
  width: number;
  height: number;
}

export interface ViewportSize {
  width: number;
  height: number;
}

export interface TooltipCoords {
  top: number;
  left: number;
  placement: TooltipPlacement;
  arrowOffset: number;
}

const VIEWPORT_PADDING = 10;
const TRIGGER_GAP = 8;
const ARROW_MIN_INSET = 14;

function clamp(value: number, min: number, max: number): number {
  if (max < min) return min;
  return Math.min(Math.max(value, min), max);
}

function fitsPlacement(
  placement: TooltipPlacement,
  trigger: RectLike,
  tooltip: { width: number; height: number },
  viewport: ViewportSize
): boolean {
  switch (placement) {
    case "top":
      return trigger.top - TRIGGER_GAP - tooltip.height >= VIEWPORT_PADDING;
    case "bottom":
      return trigger.bottom + TRIGGER_GAP + tooltip.height <= viewport.height - VIEWPORT_PADDING;
    case "left":
      return trigger.left - TRIGGER_GAP - tooltip.width >= VIEWPORT_PADDING;
    case "right":
      return trigger.right + TRIGGER_GAP + tooltip.width <= viewport.width - VIEWPORT_PADDING;
  }
}

function flipPlacement(placement: TooltipPlacement): TooltipPlacement {
  switch (placement) {
    case "top":
      return "bottom";
    case "bottom":
      return "top";
    case "left":
      return "right";
    case "right":
      return "left";
  }
}

/**
 * Computes fixed viewport coordinates for a tooltip clamped inside `100vw × 100vh`,
 * flipping away from viewport edges when insufficient space is available.
 */
export function computeTooltipPosition(
  trigger: RectLike,
  tooltip: { width: number; height: number },
  viewport: ViewportSize,
  preferred: TooltipPlacement = "top"
): TooltipCoords {
  const opposite = flipPlacement(preferred);
  let placement = preferred;
  if (!fitsPlacement(preferred, trigger, tooltip, viewport)) {
    if (fitsPlacement(opposite, trigger, tooltip, viewport)) {
      placement = opposite;
    } else {
      const spacePreferred =
        preferred === "top"
          ? trigger.top
          : preferred === "bottom"
            ? viewport.height - trigger.bottom
            : preferred === "left"
              ? trigger.left
              : viewport.width - trigger.right;
      const spaceOpposite =
        opposite === "top"
          ? trigger.top
          : opposite === "bottom"
            ? viewport.height - trigger.bottom
            : opposite === "left"
              ? trigger.left
              : viewport.width - trigger.right;
      if (spaceOpposite > spacePreferred) {
        placement = opposite;
      }
    }
  }

  const triggerCenterX = trigger.left + trigger.width / 2;
  const triggerCenterY = trigger.top + trigger.height / 2;

  let rawTop = 0;
  let rawLeft = 0;

  if (placement === "top") {
    rawTop = trigger.top - TRIGGER_GAP - tooltip.height;
    rawLeft = triggerCenterX - tooltip.width / 2;
  } else if (placement === "bottom") {
    rawTop = trigger.bottom + TRIGGER_GAP;
    rawLeft = triggerCenterX - tooltip.width / 2;
  } else if (placement === "left") {
    rawTop = triggerCenterY - tooltip.height / 2;
    rawLeft = trigger.left - TRIGGER_GAP - tooltip.width;
  } else {
    rawTop = triggerCenterY - tooltip.height / 2;
    rawLeft = trigger.right + TRIGGER_GAP;
  }

  const maxLeft = Math.max(VIEWPORT_PADDING, viewport.width - tooltip.width - VIEWPORT_PADDING);
  const maxTop = Math.max(VIEWPORT_PADDING, viewport.height - tooltip.height - VIEWPORT_PADDING);

  const left = clamp(rawLeft, VIEWPORT_PADDING, maxLeft);
  const top = clamp(rawTop, VIEWPORT_PADDING, maxTop);

  const arrowOffset =
    placement === "top" || placement === "bottom"
      ? clamp(triggerCenterX - left, ARROW_MIN_INSET, Math.max(ARROW_MIN_INSET, tooltip.width - ARROW_MIN_INSET))
      : clamp(triggerCenterY - top, ARROW_MIN_INSET, Math.max(ARROW_MIN_INSET, tooltip.height - ARROW_MIN_INSET));

  return { top, left, placement, arrowOffset };
}

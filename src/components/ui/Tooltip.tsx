import {
  CSSProperties,
  ReactNode,
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import {
  computeTooltipPosition,
  TooltipCoords,
  TooltipPlacement,
} from "./tooltipPosition";

export interface TooltipProps {
  content: ReactNode;
  children: ReactNode;
  preferredPlacement?: TooltipPlacement;
  /** Set `false` when wrapping a `<button>` or placing inside an existing `<button>`. */
  focusable?: boolean;
  /** Optional wrapper class applied to the inline trigger element. */
  triggerClassName?: string;
  /** Optional accessible label on the trigger when `focusable` is enabled. */
  ariaLabel?: string;
  /** Optional test ID on the rendered tooltip portal. */
  tooltipTestId?: string;
}

/**
 * Pure visual portal tooltip with smart viewport-lock clamping (`100vw × 100vh`),
 * mouse hover + keyboard focus accessibility, and `prefers-reduced-motion` support.
 */
export function Tooltip({
  content,
  children,
  preferredPlacement = "top",
  focusable = true,
  triggerClassName,
  ariaLabel,
  tooltipTestId,
}: TooltipProps) {
  const tooltipId = useId();
  const triggerRef = useRef<HTMLSpanElement | null>(null);
  const tooltipRef = useRef<HTMLDivElement | null>(null);
  const [open, setOpen] = useState(false);
  const [coords, setCoords] = useState<TooltipCoords | null>(null);

  const hasContent = content !== null && content !== undefined && content !== false;

  const updatePosition = useCallback(() => {
    const triggerEl = triggerRef.current;
    const tooltipEl = tooltipRef.current;
    if (!triggerEl || !tooltipEl || typeof window === "undefined") return;

    const triggerRect = triggerEl.getBoundingClientRect();
    const tooltipRect = tooltipEl.getBoundingClientRect();
    const next = computeTooltipPosition(
      triggerRect,
      { width: tooltipRect.width || 240, height: tooltipRect.height || 64 },
      { width: window.innerWidth, height: window.innerHeight },
      preferredPlacement
    );
    setCoords(next);
  }, [preferredPlacement]);

  useLayoutEffect(() => {
    if (!open || !hasContent) return;
    updatePosition();
  }, [open, hasContent, content, updatePosition]);

  useEffect(() => {
    if (!open || typeof window === "undefined") return;

    const handleClose = () => setOpen(false);
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
      }
    };

    window.addEventListener("scroll", handleClose, true);
    window.addEventListener("resize", updatePosition);
    window.addEventListener("blur", handleClose);
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("scroll", handleClose, true);
      window.removeEventListener("resize", updatePosition);
      window.removeEventListener("blur", handleClose);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [open, updatePosition]);

  if (!hasContent) {
    return <>{children}</>;
  }

  const portalStyle: CSSProperties = coords
    ? {
        top: `${Math.round(coords.top)}px`,
        left: `${Math.round(coords.left)}px`,
        ["--tooltip-arrow-offset" as string]: `${Math.round(coords.arrowOffset)}px`,
      }
    : {
        top: "-9999px",
        left: "-9999px",
      };

  const placementClass = `is-${coords?.placement ?? preferredPlacement}`;

  return (
    <>
      <span
        ref={triggerRef}
        className={`ui-tooltip-trigger${triggerClassName ? ` ${triggerClassName}` : ""}`}
        tabIndex={focusable ? 0 : undefined}
        aria-label={focusable ? ariaLabel : undefined}
        aria-describedby={open ? tooltipId : undefined}
        onMouseEnter={() => setOpen(true)}
        onMouseLeave={() => setOpen(false)}
        onFocus={() => setOpen(true)}
        onBlur={() => setOpen(false)}
      >
        {children}
      </span>

      {open &&
        typeof document !== "undefined" &&
        createPortal(
          <div
            ref={tooltipRef}
            id={tooltipId}
            role="tooltip"
            data-testid={tooltipTestId ?? "rich-tooltip"}
            className={`ui-tooltip-surface ${placementClass} ${coords ? "is-positioned" : ""}`}
            style={portalStyle}
          >
            <div className="ui-tooltip-body">{content}</div>
          </div>,
          document.body
        )}
    </>
  );
}

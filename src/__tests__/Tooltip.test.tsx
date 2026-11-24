import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { Tooltip } from "../components/ui/Tooltip";
import { computeTooltipPosition } from "../components/ui/tooltipPosition";

describe("computeTooltipPosition", () => {
  const viewport = { width: 1000, height: 700 };
  const tooltipSize = { width: 220, height: 90 };

  it("places tooltip above trigger when top space is sufficient and clamps horizontally inside viewport", () => {
    const pos = computeTooltipPosition(
      { top: 300, left: 480, width: 40, height: 40, right: 520, bottom: 340 },
      tooltipSize,
      viewport,
      "top"
    );

    expect(pos.placement).toBe("top");
    expect(pos.top).toBe(300 - 90 - 8);
    expect(pos.left).toBe(390);
    expect(pos.arrowOffset).toBe(110);
  });

  it("flips to bottom when trigger is near the top edge and clamps left/right within viewport lock", () => {
    const topLeft = computeTooltipPosition(
      { top: 12, left: 6, width: 32, height: 32, right: 38, bottom: 44 },
      tooltipSize,
      viewport,
      "top"
    );

    expect(topLeft.placement).toBe("bottom");
    expect(topLeft.top).toBe(44 + 8);
    expect(topLeft.left).toBe(10);
    expect(topLeft.left + tooltipSize.width).toBeLessThanOrEqual(viewport.width - 10);

    const topRight = computeTooltipPosition(
      { top: 200, left: 960, width: 32, height: 32, right: 992, bottom: 232 },
      tooltipSize,
      viewport,
      "top"
    );

    expect(topRight.left + tooltipSize.width).toBeLessThanOrEqual(viewport.width - 10);
    expect(topRight.arrowOffset).toBeGreaterThanOrEqual(14);
    expect(topRight.arrowOffset).toBeLessThanOrEqual(tooltipSize.width - 14);
  });
});

describe("Tooltip component", () => {
  it("opens on mouse hover and keyboard focus, links via aria-describedby, and closes on blur or Escape", () => {
    render(
      <Tooltip content={<span>Rich details</span>} ariaLabel="Item slot">
        <span>Target</span>
      </Tooltip>
    );

    const trigger = screen.getByLabelText("Item slot");
    expect(trigger).toHaveAttribute("tabindex", "0");
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();

    fireEvent.mouseEnter(trigger);
    const tooltip = screen.getByRole("tooltip");
    expect(tooltip).toHaveTextContent("Rich details");
    expect(trigger).toHaveAttribute("aria-describedby", tooltip.id);

    fireEvent.mouseLeave(trigger);
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();

    fireEvent.focus(trigger);
    expect(screen.getByRole("tooltip")).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();

    fireEvent.focus(trigger);
    expect(screen.getByRole("tooltip")).toBeInTheDocument();
    fireEvent.blur(trigger);
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });

  it("omits tabIndex on wrapper when focusable=false so nested buttons do not double-stop", () => {
    render(
      <Tooltip content={<span>Spell info</span>} focusable={false}>
        <button type="button">Flash</button>
      </Tooltip>
    );

    const button = screen.getByRole("button", { name: "Flash" });
    expect(button.parentElement).not.toHaveAttribute("tabindex");

    fireEvent.focus(button);
    expect(screen.getByRole("tooltip")).toHaveTextContent("Spell info");
  });
});

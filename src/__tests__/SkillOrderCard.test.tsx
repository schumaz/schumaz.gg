import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { SkillOrderCard } from "../components/ui/SkillOrderCard";
import { SkillOrderPopover } from "../components/ui/SkillOrderPopover";
import type { SkillOrderRecommendation } from "../types";

const sampleSkillOrder: SkillOrderRecommendation = {
  priority: ["Q", "E", "W"],
  order: ["Q", "W", "E", "Q", "Q", "R", "Q", "Q", "E", "R", "E", "E", "E", "W", "W", "R", "W", "W"],
  play: 53930,
  win: 32799,
  pick_rate: 79.2,
  win_rate: 60.8,
  abilities: [
    {
      key: "Q",
      name: "Disparo Místico",
      icon_url: "EzrealQ.png",
      description: "Ezreal dispara um raio de energia que aplica efeitos ao contato.",
      cooldown_burn: "5.5/5.25/5/4.75/4.5",
      max_rank: 5,
    },
    {
      key: "W",
      name: "Fluxo Essencial",
      icon_url: "EzrealW.png",
      description: "Ezreal dispara uma orbe que se prende ao primeiro Campeão atingido.",
      cooldown_burn: "8",
      max_rank: 5,
    },
    {
      key: "E",
      name: "Translocação Arcana",
      icon_url: "EzrealE.png",
      description: "Ezreal se teleporta para um local próximo e dispara um raio.",
      cooldown_burn: "26/23/20/17/14",
      max_rank: 5,
    },
    {
      key: "R",
      name: "Barragem Incendiária",
      icon_url: "EzrealR.png",
      description: "Ezreal canaliza antes de disparar uma poderosa onda de energia.",
      cooldown_burn: "120/105/90",
      max_rank: 3,
    },
  ],
};

describe("SkillOrderCard", () => {
  it("renders priority badges, stats, and 1-18 matrix in Champ Select and readOnly modes", () => {
    const { rerender } = render(<SkillOrderCard skillOrder={sampleSkillOrder} />);

    expect(screen.getByTestId("skill-order-card")).toBeInTheDocument();
    expect(screen.getByText("Ordem de Habilidades")).toBeInTheDocument();
    expect(screen.getByTestId("skill-priority-chain")).toHaveTextContent(/Q.*>.*E.*>.*W/);

    const stats = screen.getByTestId("skill-order-stats");
    expect(stats).toHaveTextContent("60.8% WR");
    expect(stats).toHaveTextContent(/79\.2% escolha/i);

    for (let lvl = 1; lvl <= 18; lvl += 1) {
      const cell = screen.getByTestId(`skill-level-${lvl}`);
      expect(cell).toHaveAttribute("data-skill", sampleSkillOrder.order![lvl - 1]);
      expect(cell).toHaveTextContent(sampleSkillOrder.order![lvl - 1]);
    }

    rerender(<SkillOrderCard skillOrder={sampleSkillOrder} readOnly={true} />);
    expect(screen.getByTestId("skill-order-stats")).toHaveTextContent(/79\.2% uso/i);
  });

  it("shows rich localized ability tooltip on hover and keyboard focus", () => {
    render(<SkillOrderCard skillOrder={sampleSkillOrder} />);

    const triggers = screen.getAllByLabelText("Disparo Místico");
    fireEvent.mouseEnter(triggers[0]);
    const hoverTooltip = screen.getByRole("tooltip");
    expect(hoverTooltip).toHaveTextContent("Disparo Místico");
    expect(hoverTooltip).toHaveTextContent("5.5/5.25/5/4.75/4.5s recarga");
    expect(hoverTooltip).toHaveTextContent(/Ezreal dispara um raio de energia/);
    fireEvent.mouseLeave(triggers[0]);

    const ultTrigger = screen.getByLabelText("Barragem Incendiária");
    fireEvent.focus(ultTrigger);
    const focusTooltip = screen.getByRole("tooltip");
    expect(focusTooltip).toHaveTextContent("Barragem Incendiária");
    expect(focusTooltip).toHaveTextContent("120/105/90s recarga");
  });

  it("falls back cleanly without ability icons and returns null when skillOrder is absent or empty", () => {
    const { container, rerender } = render(<SkillOrderCard skillOrder={null} />);
    expect(container.firstChild).toBeNull();

    rerender(<SkillOrderCard skillOrder={{ priority: [], order: [] }} />);
    expect(container.firstChild).toBeNull();

    rerender(
      <SkillOrderCard
        skillOrder={{
          priority: ["Q", "W", "E"],
          order: ["Q", "W", "E"],
          play: 150,
          win: 80,
          pick_rate: 0,
          win_rate: 53.3,
          abilities: [],
        }}
      />
    );
    expect(screen.getByTestId("skill-priority-chain")).toHaveTextContent(/Q.*>.*W.*>.*E/);
    expect(screen.getByTestId("skill-order-stats")).toHaveTextContent(/150 partidas/i);
    expect(screen.getByTestId("skill-level-1")).toHaveTextContent("Q");
  });

  it("opens and closes the on-demand 1-18 popover via click, close button, Escape, outside click, and resetKey", () => {
    const { rerender } = render(
      <div>
        <button type="button" data-testid="outside-el">
          Fora
        </button>
        <SkillOrderPopover skillOrder={sampleSkillOrder} readOnly={true} resetKey="103-mid" />
      </div>
    );

    const trigger = screen.getByTestId("skill-order-trigger");
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    expect(screen.getByTestId("skill-priority-chain")).toHaveTextContent(/Q.*>.*E.*>.*W/);
    expect(screen.queryByTestId("skill-order-popover")).not.toBeInTheDocument();

    fireEvent.click(trigger);
    expect(trigger).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByTestId("skill-order-popover")).toBeInTheDocument();
    expect(screen.getByTestId("skill-order-grid")).toBeInTheDocument();
    expect(screen.getByTestId("skill-order-stats")).toHaveTextContent(/79\.2% uso/i);

    fireEvent.click(screen.getByTestId("skill-order-close-btn"));
    expect(screen.queryByTestId("skill-order-popover")).not.toBeInTheDocument();

    fireEvent.click(trigger);
    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByTestId("skill-order-popover")).not.toBeInTheDocument();

    fireEvent.click(trigger);
    fireEvent.mouseDown(screen.getByTestId("outside-el"));
    expect(screen.queryByTestId("skill-order-popover")).not.toBeInTheDocument();

    fireEvent.click(trigger);
    expect(screen.getByTestId("skill-order-popover")).toBeInTheDocument();
    rerender(
      <div>
        <button type="button" data-testid="outside-el">
          Fora
        </button>
        <SkillOrderPopover skillOrder={sampleSkillOrder} readOnly={true} resetKey="238-mid" />
      </div>
    );
    expect(screen.queryByTestId("skill-order-popover")).not.toBeInTheDocument();
  });
});

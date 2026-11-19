// @ts-expect-error type error without @types/node package
import { readFileSync } from "node:fs";
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { IdleIsland, IdleState } from "../IdleState";

describe("IdleState & IdleIsland loading perimeter ring", () => {
  it("renders waiting state with radar ring and without loading perimeter SVG", () => {
    render(<IdleState summoner={null} errorMsg={null} loading={false} />);

    const waiting = screen.getByTestId("runes-waiting-champ");
    expect(waiting.querySelector(".idle-radar-ring")).toBeInTheDocument();
    expect(waiting.querySelector(".idle-spinner-ring")).not.toBeInTheDocument();
  });

  it("renders loading build state with concentric SVG rounded-rect perimeter trail", () => {
    render(<IdleState summoner={null} errorMsg={null} loading={true} />);

    const loading = screen.getByTestId("runes-loading-build");
    expect(loading.querySelector(".idle-radar-ring")).not.toBeInTheDocument();

    const ring = loading.querySelector(".idle-spinner-ring");
    expect(ring).toBeInTheDocument();

    const svg = ring?.querySelector("svg.idle-spinner-svg");
    expect(svg).toBeInTheDocument();
    expect(svg).toHaveAttribute("viewBox", "0 0 80 80");

    const track = svg?.querySelector("rect.idle-spinner-track");
    const trail = svg?.querySelector("rect.idle-spinner-trail");
    const beam = svg?.querySelector("rect.idle-spinner-beam");
    expect(track).toHaveAttribute("rx", "20");
    expect(trail).toHaveAttribute("pathLength", "100");
    expect(beam).toHaveAttribute("pathLength", "100");
  });

  it("renders IdleIsland loading mode with concentric SVG ring for profile loading cards", () => {
    render(
      <IdleIsland
        testId="custom-loading-island"
        mode="loading"
        title="Carregando Perfil"
        description="Buscando dados..."
        icon={<span data-testid="island-icon" />}
      />
    );

    const card = screen.getByTestId("custom-loading-island");
    expect(card.querySelector(".idle-icon-box.is-loading")).toBeInTheDocument();
    expect(card.querySelector(".idle-spinner-ring svg.idle-spinner-svg")).toBeInTheDocument();
    expect(screen.getByTestId("island-icon")).toBeInTheDocument();
  });

  it("does not spin a rounded square border and enforces inactive-tab and reduced-motion pause rules", () => {
    const champSelectCss = readFileSync(
      "src/features/champ-select/ChampSelect.css",
      "utf8"
    ) as string;
    const tokensShellCss = readFileSync(
      "src/styles/tokens-shell.css",
      "utf8"
    ) as string;

    const spinnerRingRule = champSelectCss.match(
      /\.idle-icon-box\.is-loading \.idle-spinner-ring\s*\{[^}]*\}/
    )?.[0];
    expect(spinnerRingRule).toBeDefined();
    expect(spinnerRingRule).not.toMatch(/animation\s*:\s*spin/);

    expect(champSelectCss).toContain("idleSpinnerTrailOrbit");
    expect(champSelectCss).toContain("idleSpinnerBeamOrbit");

    expect(tokensShellCss).toContain(".tab-content.inactive-tab .idle-spinner-ring");
    expect(tokensShellCss).toContain(".app-minimized .idle-spinner-ring");
    expect(tokensShellCss).toMatch(
      /@media \(prefers-reduced-motion: reduce\)\s*\{[\s\S]*\.idle-spinner-ring,[\s\S]*\.idle-spinner-trail,[\s\S]*\.idle-spinner-beam,/
    );
  });
});

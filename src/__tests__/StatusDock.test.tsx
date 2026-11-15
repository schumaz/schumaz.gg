import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { StatusDock } from "../components/StatusDock";
import { SummonerInfo } from "../types";

describe("StatusDock", () => {
  it("renders disconnected status with pulsing dot and animated dots when summoner is null", () => {
    const { container } = render(<StatusDock summoner={null} />);

    expect(screen.getByTestId("status-disconnected")).toBeInTheDocument();
    expect(
      screen.getByText("Aguardando cliente do LoL")
    ).toBeInTheDocument();

    const orangeDot = container.querySelector(".dot-orange.dot-pulsing");
    expect(orangeDot).toBeInTheDocument();

    const dots = screen.getByTestId("status-waiting-dots");
    expect(dots).toHaveAttribute("aria-hidden", "true");
    expect(dots.querySelectorAll(".loading-dot")).toHaveLength(3);
  });

  it("renders clean connected status without player nick, tagLine, or summoner level", () => {
    const mockSummoner: SummonerInfo = {
      summonerId: 12345,
      gameName: "Faker",
      tagLine: "KR1",
      displayName: "Faker",
      summonerLevel: 500,
      profileIconId: 1,
    };

    render(<StatusDock summoner={mockSummoner} />);

    const badge = screen.getByTestId("status-connected");
    expect(badge).toBeInTheDocument();
    expect(screen.getByText("Conectado ao cliente do LoL")).toBeInTheDocument();
    expect(screen.queryByTestId("status-waiting-dots")).not.toBeInTheDocument();

    expect(screen.queryByText(/Faker/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/KR1/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/500/)).not.toBeInTheDocument();
    expect(screen.queryByText(/Nível/i)).not.toBeInTheDocument();
  });
});

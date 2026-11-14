import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { StatusDock } from "../components/StatusDock";
import { SummonerInfo } from "../types";

describe("StatusDock", () => {
  it("renders disconnected status when summoner is null", () => {
    render(<StatusDock summoner={null} />);

    expect(screen.getByTestId("status-disconnected")).toBeInTheDocument();
    expect(
      screen.getByText("Aguardando cliente do LoL")
    ).toBeInTheDocument();
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

    expect(screen.queryByText(/Faker/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/KR1/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/500/)).not.toBeInTheDocument();
    expect(screen.queryByText(/Nível/i)).not.toBeInTheDocument();
  });
});

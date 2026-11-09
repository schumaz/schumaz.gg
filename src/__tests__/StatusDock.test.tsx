import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { StatusDock } from "../components/StatusDock";
import { SummonerInfo } from "../types";

describe("StatusDock", () => {
  it("renders disconnected status when summoner is null", () => {
    render(<StatusDock summoner={null} />);

    expect(screen.getByTestId("status-disconnected")).toBeInTheDocument();
    expect(
      screen.getByText("Aguardando cliente do LoL (LeagueClientUx.exe)...")
    ).toBeInTheDocument();
  });

  it("renders connected status with summoner details", () => {
    const mockSummoner: SummonerInfo = {
      summonerId: 12345,
      gameName: "Faker",
      tagLine: "KR1",
      displayName: "Faker",
      summonerLevel: 500,
      profileIconId: 1,
    };

    render(<StatusDock summoner={mockSummoner} />);

    expect(screen.getByTestId("status-connected")).toBeInTheDocument();
    expect(screen.getByText("Faker")).toBeInTheDocument();
    expect(screen.getByText(/KR1/)).toBeInTheDocument();
    expect(screen.getByText(/500/)).toBeInTheDocument();
  });
});

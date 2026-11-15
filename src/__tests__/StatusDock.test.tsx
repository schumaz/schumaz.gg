import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { StatusDock } from "../components/ui/StatusDock";
import { windowIpc } from "../services/ipc/window.ipc";
import { SummonerInfo } from "../types";

describe("StatusDock", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("renders visible brand icon/title, window controls, and disconnected status when summoner is null", () => {
    const { container } = render(<StatusDock summoner={null} />);

    expect(screen.getByText("schumaz.gg")).toBeInTheDocument();
    expect(container.querySelector(".header-brand-icon")).toBeInTheDocument();

    expect(screen.getByTestId("window-minimize-btn")).toBeInTheDocument();
    expect(screen.getByTestId("window-maximize-btn")).toBeInTheDocument();
    expect(screen.getByTestId("window-close-btn")).toBeInTheDocument();

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

  it("invokes window control IPC actions and updates maximize icon state", async () => {
    const minSpy = vi.spyOn(windowIpc, "minimizeWindow").mockResolvedValue(undefined);
    const maxSpy = vi.spyOn(windowIpc, "toggleMaximizeWindow").mockResolvedValue(undefined);
    const closeSpy = vi.spyOn(windowIpc, "closeWindow").mockResolvedValue(undefined);
    const isMaxSpy = vi.spyOn(windowIpc, "isWindowMaximized").mockResolvedValue(true);

    render(<StatusDock summoner={null} />);

    fireEvent.click(screen.getByTestId("window-minimize-btn"));
    expect(minSpy).toHaveBeenCalledTimes(1);

    const maxBtn = screen.getByTestId("window-maximize-btn");
    expect(maxBtn.querySelector("rect")).toBeInTheDocument();
    fireEvent.click(maxBtn);
    expect(maxSpy).toHaveBeenCalledTimes(1);
    await waitFor(() => {
      expect(isMaxSpy).toHaveBeenCalled();
      expect(maxBtn.querySelector("path")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId("window-close-btn"));
    expect(closeSpy).toHaveBeenCalledTimes(1);
  });
});

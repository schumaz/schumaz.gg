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

    expect(screen.getByTestId("window-minimize-btn")).toHaveAttribute(
      "aria-label",
      "Minimizar janela"
    );
    expect(screen.getByTestId("window-maximize-btn")).toHaveAttribute(
      "aria-label",
      "Maximizar janela"
    );
    expect(screen.getByTestId("window-close-btn")).toHaveAttribute(
      "aria-label",
      "Fechar janela"
    );

    expect(screen.getByTestId("status-disconnected")).toBeInTheDocument();
    expect(
      screen.getByText("Aguardando cliente do LoL")
    ).toBeInTheDocument();

    const orangeDot = container.querySelector(".dot.dot-orange");
    expect(orangeDot).toBeInTheDocument();
    expect(container.querySelector(".dot-pulsing")).not.toBeInTheDocument();
    expect(screen.queryByTestId("status-waiting-dots")).not.toBeInTheDocument();
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
    expect(maxBtn).toHaveAttribute("aria-label", "Maximizar janela");
    fireEvent.click(maxBtn);
    expect(maxSpy).toHaveBeenCalledTimes(1);
    await waitFor(() => {
      expect(isMaxSpy).toHaveBeenCalled();
      expect(maxBtn.querySelector("path")).toBeInTheDocument();
      expect(maxBtn).toHaveAttribute("aria-label", "Restaurar janela");
    });

    fireEvent.click(screen.getByTestId("window-close-btn"));
    expect(closeSpy).toHaveBeenCalledTimes(1);
  });

  it("renders installer update pill when ready and triggers restart or dismiss callbacks", () => {
    const onRestart = vi.fn();
    const onDismiss = vi.fn();

    render(
      <StatusDock
        summoner={null}
        updateState={{ status: "ready", version: "0.2.0" }}
        onRestartUpdate={onRestart}
        onDismissUpdate={onDismiss}
      />
    );

    expect(screen.getByTestId("update-pill-ready")).toBeInTheDocument();
    expect(screen.getByText("Atualização v0.2.0 pronta • Reiniciar")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("update-restart-btn"));
    expect(onRestart).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByTestId("update-dismiss-btn"));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it("renders portable update pill and triggers GitHub release opener callback", () => {
    const onOpenPortable = vi.fn();
    const onDismiss = vi.fn();

    render(
      <StatusDock
        summoner={null}
        updateState={{
          status: "portable-available",
          version: "0.2.0",
          releasesUrl: "https://github.com/schumaz/schumaz.gg/releases/latest",
        }}
        onOpenPortableRelease={onOpenPortable}
        onDismissUpdate={onDismiss}
      />
    );

    expect(screen.getByTestId("update-pill-portable")).toBeInTheDocument();
    expect(
      screen.getByText("Nova versão v0.2.0 disponível • Baixar no GitHub")
    ).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("update-portable-btn"));
    expect(onOpenPortable).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByTestId("update-dismiss-btn"));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });
});

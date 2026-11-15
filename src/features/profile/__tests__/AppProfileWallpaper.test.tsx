import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import App from "../../../App";
import { invoke } from "@tauri-apps/api/core";
import { defaultAppInvokeMock, mockAhriBuild } from "../../../__tests__/appTestFixtures";

let eventListeners: Record<string, (event: { payload: unknown }) => void> = {};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args?: Record<string, unknown>) =>
    defaultAppInvokeMock(cmd, args)
  ),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockImplementation((event: string, cb: (payload: unknown) => void) => {
    eventListeners[event] = cb;
    return Promise.resolve(() => {
      delete eventListeners[event];
    });
  }),
}));

describe("App — Profile Wallpaper, Blur & Tab Visibility", () => {
  beforeEach(() => {
    eventListeners = {};
    localStorage.clear();
    localStorage.setItem("schumaz_language", "pt_BR");
  });

  it("renders full-screen profile wallpaper only on Perfil tab, applies blur slider from Configurações, and enforces 30s refresh cooldown", async () => {
    vi.mocked(invoke).mockImplementation((cmd: string, args?: unknown) => {
      if (cmd === "get_summoner_status") return Promise.resolve(null);
      if (cmd === "get_player_profile" || cmd === "get_cached_player_profile") {
        return Promise.resolve({
          summoner: {
            summonerId: 987654321,
            gameName: "Schuma",
            tagLine: "BR1",
            displayName: "Schuma",
            summonerLevel: 350,
            profileIconId: 548,
          },
          profile_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/profileicon/548.png",
          background_skin_id: 103015,
          background_skin_name: "",
          background_splash_url: "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_15.jpg",
          background_splash_fallback_url: "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_0.jpg",
          ranked_stats: [],
          top_masteries: [],
          recent_matches: [],
        });
      }
      if (cmd === "get_recommended_build_for_champion") return Promise.resolve(mockAhriBuild);
      return defaultAppInvokeMock(cmd, args as Record<string, unknown>);
    });

    render(<App />);

    await waitFor(() => {
      expect(eventListeners["lcu-connected"]).toBeDefined();
    });

    await act(async () => {
      eventListeners["lcu-connected"]({
        payload: {
          summonerId: 987654321,
          gameName: "Schuma",
          tagLine: "BR1",
          displayName: "Schuma",
          summonerLevel: 350,
          profileIconId: 548,
        },
      });
    });

    await waitFor(() => {
      expect(screen.getByTestId("profile-wallpaper")).toBeInTheDocument();
    });

    const wpImg = screen.getByTestId("profile-wallpaper").querySelector("img")!;
    expect(wpImg.style.filter).toContain("blur(1px)");
    expect(screen.getByTestId("view-profile").style.getPropertyValue("--profile-ui-blur")).toBe("3.2px");
    expect(screen.getByTestId("top-masteries-card").style.backdropFilter).toBe("blur(3.2px)");

    fireEvent.click(screen.getByRole("button", { name: /^configurações$/i }));
    expect(screen.queryByTestId("profile-wallpaper")).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId("profile-blur-value"));
    const numInput = screen.getByTestId("profile-blur-input");
    fireEvent.change(numInput, { target: { value: "100" } });
    fireEvent.keyDown(numInput, { key: "Enter" });
    expect(localStorage.getItem("schumaz_profile_ui_blur")).toBe("100");
    expect(screen.getByTestId("profile-blur-value")).toHaveTextContent("100%");

    fireEvent.click(screen.getByRole("button", { name: /^perfil$/i }));
    const wpImgAfter = screen.getByTestId("profile-wallpaper").querySelector("img")!;
    expect(wpImgAfter.style.filter).toContain("blur(1px)");
    expect(screen.getByTestId("view-profile").style.getPropertyValue("--profile-ui-blur")).toBe("32px");
    expect(screen.getByTestId("top-masteries-card").style.backdropFilter).toBe("blur(32px)");

    fireEvent.click(screen.getByRole("button", { name: /^configurações$/i }));
    fireEvent.click(screen.getByTestId("profile-blur-value"));
    const zeroInput = screen.getByTestId("profile-blur-input");
    fireEvent.change(zeroInput, { target: { value: "0" } });
    fireEvent.keyDown(zeroInput, { key: "Enter" });
    fireEvent.click(screen.getByRole("button", { name: /^perfil$/i }));
    expect(screen.getByTestId("top-masteries-card").style.backdropFilter).toBe("none");
    expect(document.querySelector("main.profile-blur-off")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /^configurações$/i }));
    fireEvent.click(screen.getByTestId("wallpaper-visible-toggle"));
    expect(localStorage.getItem("schumaz_profile_wallpaper_visible")).toBe("false");
    fireEvent.click(screen.getByRole("button", { name: /^perfil$/i }));
    expect(screen.queryByTestId("profile-wallpaper")).not.toBeInTheDocument();

    const refreshBtn = screen.getByTestId("refresh-profile-btn");
    await act(async () => {
      fireEvent.click(refreshBtn);
    });
    expect(refreshBtn).toBeDisabled();
    expect(refreshBtn).toHaveTextContent("Atualizar (30s)");
  });

  it("keeps view-settings inactive and hidden unless Configurações tab is selected", () => {
    render(<App />);

    expect(screen.getByTestId("view-profile")).toHaveClass("active-tab");
    expect(screen.getByTestId("view-settings")).toHaveClass("inactive-tab");
    expect(screen.getByTestId("view-settings")).toHaveAttribute("aria-hidden", "true");

    fireEvent.click(screen.getByRole("button", { name: /^runas$/i }));
    expect(screen.getByTestId("view-runes")).toHaveClass("active-tab");
    expect(screen.getByTestId("view-settings")).toHaveClass("inactive-tab");
    expect(screen.getByTestId("view-settings")).toHaveAttribute("aria-hidden", "true");

    fireEvent.click(screen.getByRole("button", { name: /^configurações$/i }));
    expect(screen.getByTestId("view-settings")).toHaveClass("active-tab");
    expect(screen.getByTestId("view-settings")).toHaveAttribute("aria-hidden", "false");
    expect(screen.getByTestId("view-profile")).toHaveClass("inactive-tab");
  });

  it("pauses NavigationDock border beam when GPU hardware acceleration is disabled", async () => {
    vi.mocked(invoke).mockImplementation((cmd: string, args?: unknown) => {
      if (cmd === "get_hardware_acceleration_status") {
        return Promise.resolve({ enabled: false, active_at_startup: false });
      }
      return defaultAppInvokeMock(cmd, args as Record<string, unknown>);
    });

    const { container } = render(<App />);

    await waitFor(() => {
      expect(container.querySelector("main.gpu-effects-disabled")).toBeInTheDocument();
    });
    expect(container.querySelector(".dock-beam-wrap")).not.toBeInTheDocument();
  });
});

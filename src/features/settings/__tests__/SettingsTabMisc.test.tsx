import { useState } from "react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { AppLocale, I18nProvider } from "../../../i18n";
import { SettingsTab } from "../SettingsTab";
import { DEFAULT_PREFERENCES } from "../../../types";

vi.mock("@tauri-apps/api/core", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@tauri-apps/api/core")>();
  return {
    ...actual,
    invoke: vi.fn(),
  };
});

describe("SettingsTab — General, Automation & Appearance sections", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  afterEach(() => {
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  it("renders GPU acceleration toggle and restart banner inside Appearance & Performance", () => {
    const handleGpuChange = vi.fn();
    const handleRestart = vi.fn();

    const { rerender } = render(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={vi.fn()}
        hardwareAcceleration={true}
        onHardwareAccelerationChange={handleGpuChange}
        gpuRestartRequired={false}
        onRestartApp={handleRestart}
      />
    );

    const appearanceGroup = screen.getByTestId("settings-group-appearance");
    expect(appearanceGroup).toHaveTextContent("Aparência e Desempenho");

    const gpuToggle = screen.getByTestId("hardware-acceleration-toggle");
    expect(appearanceGroup).toContainElement(gpuToggle);
    expect(gpuToggle).toBeChecked();
    expect(screen.queryByTestId("gpu-restart-banner")).not.toBeInTheDocument();

    fireEvent.click(gpuToggle);
    expect(handleGpuChange).toHaveBeenCalledWith(false);

    rerender(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={vi.fn()}
        hardwareAcceleration={false}
        onHardwareAccelerationChange={handleGpuChange}
        gpuRestartRequired={true}
        onRestartApp={handleRestart}
      />
    );

    expect(screen.getByTestId("hardware-acceleration-toggle")).not.toBeChecked();
    expect(screen.getByTestId("gpu-restart-banner")).toBeInTheDocument();
    fireEvent.click(screen.getByTestId("gpu-restart-btn"));
    expect(handleRestart).toHaveBeenCalledTimes(1);
  });

  it("renders daily missions auto-read toggle disabled by default inside Automation & Client", () => {
    const handleUpdate = vi.fn();

    const { rerender } = render(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={handleUpdate}
      />
    );

    const automationGroup = screen.getByTestId("settings-group-automation");
    const dailyMissionsToggle = screen.getByTestId("auto-mark-daily-missions-toggle");
    expect(automationGroup).toContainElement(dailyMissionsToggle);

    expect(DEFAULT_PREFERENCES.autoMarkDailyMissionsRead).toBe(false);
    expect(dailyMissionsToggle).not.toBeChecked();

    fireEvent.click(dailyMissionsToggle);
    expect(handleUpdate).toHaveBeenCalledWith("autoMarkDailyMissionsRead", true);

    rerender(
      <SettingsTab
        preferences={{ ...DEFAULT_PREFERENCES, autoMarkDailyMissionsRead: true }}
        onUpdatePreference={handleUpdate}
      />
    );

    const enabledToggle = screen.getByTestId("auto-mark-daily-missions-toggle");
    expect(enabledToggle).toBeChecked();
    fireEvent.click(enabledToggle);
    expect(handleUpdate).toHaveBeenCalledWith("autoMarkDailyMissionsRead", false);
  });

  it("renders language selector in General and switches all section titles dynamically", () => {
    const handleLanguageChange = vi.fn();
    const handleUpdate = vi.fn();

    function Wrapper() {
      const [locale, setLocale] = useState<AppLocale>("pt_BR");
      return (
        <I18nProvider
          locale={locale}
          onLocaleChange={(next) => {
            setLocale(next);
            handleLanguageChange(next);
          }}
        >
          <SettingsTab
            preferences={{ ...DEFAULT_PREFERENCES, language: locale }}
            onUpdatePreference={handleUpdate}
            locale={locale}
            onChangeLanguage={(next) => {
              setLocale(next);
              handleLanguageChange(next);
            }}
          />
        </I18nProvider>
      );
    }

    render(<Wrapper />);

    const generalGroup = screen.getByTestId("settings-group-general");
    const langSubgroup = screen.getByTestId("settings-subgroup-language");
    expect(generalGroup).toContainElement(langSubgroup);

    const ptBtn = screen.getByTestId("settings-lang-pt_BR");
    const enBtn = screen.getByTestId("settings-lang-en_US");
    expect(ptBtn).toHaveAttribute("aria-pressed", "true");
    expect(enBtn).toHaveAttribute("aria-pressed", "false");
    expect(generalGroup).toHaveTextContent("Geral");

    fireEvent.click(enBtn);
    expect(handleLanguageChange).toHaveBeenCalledWith("en_US");
    expect(generalGroup).toHaveTextContent("General");
    expect(langSubgroup).toHaveTextContent("App Language");
    expect(screen.getByTestId("settings-group-automation")).toHaveTextContent("Automation & Client");
    expect(screen.getByTestId("settings-group-appearance")).toHaveTextContent("Appearance & Performance");
    expect(screen.getByTestId("settings-group-system")).toHaveTextContent("System & Storage");
    expect(screen.getByLabelText("Auto-Inject Runes")).toBeInTheDocument();

    fireEvent.click(ptBtn);
    expect(handleLanguageChange).toHaveBeenCalledWith("pt_BR");
    expect(generalGroup).toHaveTextContent("Geral");
    expect(langSubgroup).toHaveTextContent("Idioma do Aplicativo");
  });

  it("renders closeToTray and autostart toggles in General, blocking autostart on Portable builds", () => {
    const handleCloseToTray = vi.fn();
    const handleAutostart = vi.fn();

    const { rerender } = render(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={vi.fn()}
        closeToTray={false}
        onCloseToTrayChange={handleCloseToTray}
        autostartStatus={{ enabled: false, supported: true, isPortable: false }}
        onAutostartChange={handleAutostart}
      />
    );

    const trayToggle = screen.getByTestId("close-to-tray-toggle");
    expect(trayToggle).not.toBeChecked();
    fireEvent.click(trayToggle);
    expect(handleCloseToTray).toHaveBeenCalledWith(true);

    const autostartToggle = screen.getByTestId("autostart-toggle");
    expect(autostartToggle).toBeEnabled();
    expect(autostartToggle).not.toBeChecked();
    expect(screen.queryByTestId("autostart-portable-badge")).not.toBeInTheDocument();
    fireEvent.click(autostartToggle);
    expect(handleAutostart).toHaveBeenCalledWith(true);

    // Portable mode disables toggle and explains why
    rerender(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={vi.fn()}
        closeToTray={true}
        onCloseToTrayChange={handleCloseToTray}
        autostartStatus={{ enabled: false, supported: false, isPortable: true }}
        onAutostartChange={handleAutostart}
      />
    );

    expect(screen.getByTestId("close-to-tray-toggle")).toBeChecked();
    expect(screen.getByTestId("autostart-toggle")).toBeDisabled();
    expect(screen.getByTestId("autostart-portable-badge")).toHaveTextContent("Somente Instalado");
    expect(screen.getByTestId("autostart-desc")).toHaveTextContent(/Portable/i);
  });

  it("queries get_autostart_status lazily only when SettingsTab becomes active", async () => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === "get_autostart_status") {
        return { enabled: false, supported: true, isPortable: false };
      }
      if (cmd === "set_autostart_enabled") {
        return { enabled: true, supported: true, isPortable: false };
      }
      return null;
    });

    const { rerender } = render(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={vi.fn()}
        isActive={false}
        cacheInfo={{ totalBytes: 0, assetsBytes: 0, dataBytes: 0, fileCount: 0 }}
      />
    );

    expect(invoke).not.toHaveBeenCalledWith("get_autostart_status");

    rerender(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={vi.fn()}
        isActive={true}
        cacheInfo={{ totalBytes: 0, assetsBytes: 0, dataBytes: 0, fileCount: 0 }}
      />
    );

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_autostart_status");
    });

    fireEvent.click(screen.getByTestId("autostart-toggle"));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("set_autostart_enabled", { enabled: true });
    });
    expect(screen.getByTestId("autostart-toggle")).toBeChecked();
  });
});

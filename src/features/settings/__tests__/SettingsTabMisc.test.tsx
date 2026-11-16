import { useState } from "react";
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { AppLocale, I18nProvider } from "../../../i18n";
import { SettingsTab } from "../SettingsTab";
import { DEFAULT_PREFERENCES } from "../../../types";

describe("SettingsTab — Miscellaneous section", () => {
  it("renders Diversos GPU acceleration toggle and restart banner when restart is required", () => {
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

    const miscGroup = screen.getByTestId("settings-group-misc");
    expect(miscGroup).toHaveTextContent("Diversos");

    const gpuToggle = screen.getByTestId("hardware-acceleration-toggle");
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

  it("renders daily missions auto-read toggle disabled by default below GPU option in Diversos", () => {
    const handleUpdate = vi.fn();

    const { rerender } = render(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={handleUpdate}
      />
    );

    const miscCard = screen.getByTestId("settings-subgroup-misc");
    const miscSubsections = Array.from(
      miscCard.querySelectorAll(".settings-subsection")
    ).map((el) => el.getAttribute("data-testid"));
    expect(miscSubsections).toEqual([
      "settings-subgroup-language",
      "settings-subgroup-graphics",
      "settings-subgroup-daily-missions",
      "settings-subgroup-updates",
    ]);

    const gpuToggle = screen.getByTestId("hardware-acceleration-toggle");
    const dailyMissionsToggle = screen.getByTestId("auto-mark-daily-missions-toggle");
    expect(miscCard).toContainElement(dailyMissionsToggle);
    expect(
      gpuToggle.compareDocumentPosition(dailyMissionsToggle) & Node.DOCUMENT_POSITION_FOLLOWING
    ).toBeTruthy();

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

  it("renders language selector with Brazil & USA flags in Diversos and switches UI language dynamically", () => {
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

    const miscGroup = screen.getByTestId("settings-group-misc");
    const langSubgroup = screen.getByTestId("settings-subgroup-language");
    expect(miscGroup).toContainElement(langSubgroup);

    const ptBtn = screen.getByTestId("settings-lang-pt_BR");
    const enBtn = screen.getByTestId("settings-lang-en_US");
    expect(ptBtn).toHaveAttribute("aria-pressed", "true");
    expect(enBtn).toHaveAttribute("aria-pressed", "false");
    expect(ptBtn.querySelector("img")).toHaveAttribute("src", expect.stringMatching(/br\.svg|data:image/i));
    expect(enBtn.querySelector("img")).toHaveAttribute("src", expect.stringMatching(/us\.svg|data:image/i));
    expect(miscGroup).toHaveTextContent("Diversos");

    fireEvent.click(enBtn);
    expect(handleLanguageChange).toHaveBeenCalledWith("en_US");
    expect(enBtn).toHaveAttribute("aria-pressed", "true");
    expect(ptBtn).toHaveAttribute("aria-pressed", "false");
    expect(miscGroup).toHaveTextContent("Miscellaneous");
    expect(langSubgroup).toHaveTextContent("App Language");
    expect(screen.getByLabelText("Auto-Inject Runes")).toBeInTheDocument();

    fireEvent.click(ptBtn);
    expect(handleLanguageChange).toHaveBeenCalledWith("pt_BR");
    expect(ptBtn).toHaveAttribute("aria-pressed", "true");
    expect(miscGroup).toHaveTextContent("Diversos");
    expect(langSubgroup).toHaveTextContent("Idioma do Aplicativo");
  });

  it("renders Updates & Version subsection for installed (NSIS) mode across check, up-to-date, error, and ready states", () => {
    const onCheck = vi.fn();
    const onRestart = vi.fn();
    const installedDist = {
      isPortable: false,
      currentVersion: "0.1.0-beta.1",
      releasesUrl: "https://github.com/schumaz/schumaz.gg/releases/latest",
    };

    const { rerender } = render(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={vi.fn()}
        distributionInfo={installedDist}
        updateState={{ status: "idle" }}
        lastUpdateCheckResult="idle"
        onCheckForUpdates={onCheck}
        onRestartToUpdate={onRestart}
      />
    );

    const updatesSubgroup = screen.getByTestId("settings-subgroup-updates");
    expect(updatesSubgroup).toHaveTextContent("Atualizações e Versão");
    expect(screen.getByTestId("settings-updates-version")).toHaveTextContent("Versão v0.1.0-beta.1");
    expect(screen.getByTestId("settings-updates-mode-badge")).toHaveTextContent(
      "Instalado · Auto-update"
    );
    expect(screen.getByTestId("settings-updates-status")).toHaveTextContent(
      "Atualizações verificadas automaticamente ao iniciar."
    );

    const checkBtn = screen.getByTestId("settings-updates-check-btn");
    expect(checkBtn).toHaveTextContent("Verificar atualizações");
    expect(checkBtn).toBeEnabled();
    fireEvent.click(checkBtn);
    expect(onCheck).toHaveBeenCalledTimes(1);

    // Checking state disables button
    rerender(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={vi.fn()}
        distributionInfo={installedDist}
        updateState={{ status: "checking" }}
        lastUpdateCheckResult="idle"
        onCheckForUpdates={onCheck}
        onRestartToUpdate={onRestart}
      />
    );
    expect(screen.getByTestId("settings-updates-status")).toHaveTextContent(
      "Verificando atualizações..."
    );
    expect(screen.getByTestId("settings-updates-check-btn")).toBeDisabled();

    // Up to date state
    rerender(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={vi.fn()}
        distributionInfo={installedDist}
        updateState={{ status: "idle" }}
        lastUpdateCheckResult="up-to-date"
        onCheckForUpdates={onCheck}
        onRestartToUpdate={onRestart}
      />
    );
    expect(screen.getByTestId("settings-updates-status")).toHaveTextContent(
      "Você está na versão mais recente."
    );

    // Error state
    rerender(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={vi.fn()}
        distributionInfo={installedDist}
        updateState={{ status: "idle" }}
        lastUpdateCheckResult="error"
        onCheckForUpdates={onCheck}
        onRestartToUpdate={onRestart}
      />
    );
    expect(screen.getByTestId("settings-updates-status")).toHaveTextContent(
      "Não foi possível verificar atualizações."
    );

    // Ready state shows Restart to update button
    rerender(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={vi.fn()}
        distributionInfo={installedDist}
        updateState={{ status: "ready", version: "0.2.0" }}
        lastUpdateCheckResult="idle"
        onCheckForUpdates={onCheck}
        onRestartToUpdate={onRestart}
      />
    );
    expect(screen.getByTestId("settings-updates-status")).toHaveTextContent(
      "Atualização v0.2.0 pronta para instalar."
    );
    const restartBtn = screen.getByTestId("settings-updates-restart-btn");
    expect(restartBtn).toHaveTextContent("Reiniciar para atualizar");
    fireEvent.click(restartBtn);
    expect(onRestart).toHaveBeenCalledTimes(1);
  });

  it("renders Updates & Version subsection for portable mode and English translations", () => {
    const onOpenGithub = vi.fn();
    const portableDist = {
      isPortable: true,
      currentVersion: "0.1.0-beta.1",
      releasesUrl: "https://github.com/schumaz/schumaz.gg/releases/latest",
    };

    render(
      <I18nProvider locale="en_US">
        <SettingsTab
          preferences={{ ...DEFAULT_PREFERENCES, language: "en_US" }}
          onUpdatePreference={vi.fn()}
          locale="en_US"
          distributionInfo={portableDist}
          updateState={{
            status: "portable-available",
            version: "0.2.0",
            releasesUrl: portableDist.releasesUrl,
          }}
          lastUpdateCheckResult="idle"
          onOpenPortableRelease={onOpenGithub}
        />
      </I18nProvider>
    );

    const updatesSubgroup = screen.getByTestId("settings-subgroup-updates");
    expect(updatesSubgroup).toHaveTextContent("Updates & Version");
    expect(screen.getByTestId("settings-updates-version")).toHaveTextContent("Version v0.1.0-beta.1");
    expect(screen.getByTestId("settings-updates-mode-badge")).toHaveTextContent(
      "Portable · Manual"
    );
    expect(screen.getByTestId("settings-updates-status")).toHaveTextContent(
      "New version v0.2.0 available on GitHub."
    );

    const githubBtn = screen.getByTestId("settings-updates-github-btn");
    expect(githubBtn).toHaveTextContent("Download on GitHub");
    fireEvent.click(githubBtn);
    expect(onOpenGithub).toHaveBeenCalledTimes(1);
  });
});

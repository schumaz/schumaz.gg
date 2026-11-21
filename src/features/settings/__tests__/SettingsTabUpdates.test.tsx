import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { I18nProvider } from "../../../i18n";
import { SettingsTab } from "../SettingsTab";
import { DEFAULT_PREFERENCES } from "../../../types";

describe("SettingsTab — Updates & Version subsection", () => {
  it("renders Updates & Version subsection for installed (NSIS) mode across idle, last-checked, check, up-to-date, error, and ready states", () => {
    const onCheck = vi.fn();
    const onRestart = vi.fn();
    const installedDist = {
      isPortable: false,
      currentVersion: "0.1.0-beta.1",
      releasesUrl: "https://github.com/schumaz/schumaz.gg/releases/latest",
      lastCheckEpochSecs: null,
      checkCooldownSecs: 86400,
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
      "Atualizações verificadas automaticamente a cada 24h."
    );

    const checkBtn = screen.getByTestId("settings-updates-check-btn");
    expect(checkBtn).toHaveTextContent("Verificar atualizações");
    expect(checkBtn).toBeEnabled();
    fireEvent.click(checkBtn);
    expect(onCheck).toHaveBeenCalledTimes(1);

    // Idle with persisted last-check timestamp shows 24h cadence + last check label
    rerender(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={vi.fn()}
        distributionInfo={installedDist}
        updateState={{ status: "idle" }}
        lastUpdateCheckResult="idle"
        lastUpdateCheckEpochSecs={1_760_000_000}
        onCheckForUpdates={onCheck}
        onRestartToUpdate={onRestart}
      />
    );
    expect(screen.getByTestId("settings-updates-status")).toHaveTextContent(
      /Verificação automática a cada 24h · Última verificação:/
    );

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
      lastCheckEpochSecs: null,
      checkCooldownSecs: 86400,
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

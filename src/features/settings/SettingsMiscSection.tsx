import { AppLocale, UserPreferences } from "../../types";
import type { AppUpdateState, UpdateCheckResult } from "../../hooks/useAppUpdater";
import type { AppDistributionInfo } from "../../services/ipc";
import { useTranslation } from "../../i18n";
import { LanguageFlag } from "../../components/ui/LanguageFlag";

interface SettingsMiscSectionProps {
  activeLocale: AppLocale;
  onSelectLanguage: (locale: AppLocale) => void;
  hardwareAcceleration: boolean;
  onHardwareAccelerationChange?: (enabled: boolean) => void;
  gpuRestartRequired: boolean;
  onRestartApp?: () => void;
  autoMarkDailyMissionsRead: boolean;
  onUpdatePreference: <K extends keyof UserPreferences>(
    key: K,
    value: UserPreferences[K]
  ) => void;
  updateState?: AppUpdateState;
  distributionInfo?: AppDistributionInfo | null;
  lastUpdateCheckResult?: UpdateCheckResult;
  onCheckForUpdates?: () => void;
  onRestartToUpdate?: () => void;
  onOpenPortableRelease?: () => void;
}

export function SettingsMiscSection({
  activeLocale,
  onSelectLanguage,
  hardwareAcceleration,
  onHardwareAccelerationChange,
  gpuRestartRequired,
  onRestartApp,
  autoMarkDailyMissionsRead,
  onUpdatePreference,
  updateState = { status: "idle" },
  distributionInfo = null,
  lastUpdateCheckResult = "idle",
  onCheckForUpdates,
  onRestartToUpdate,
  onOpenPortableRelease,
}: SettingsMiscSectionProps) {
  const { t } = useTranslation();

  const versionLabel = distributionInfo?.currentVersion
    ? t("settings.updatesVersionLabel", { version: distributionInfo.currentVersion })
    : t("settings.updatesVersionUnknown");

  let statusText = distributionInfo?.isPortable
    ? t("settings.updatesStatusIdlePortable")
    : t("settings.updatesStatusIdleInstalled");
  let statusTone: "neutral" | "success" | "ready" | "error" = "neutral";

  if (updateState.status === "checking") {
    statusText = t("settings.updatesStatusChecking");
  } else if (updateState.status === "downloading") {
    statusText = t("settings.updatesStatusDownloading", { version: updateState.version });
  } else if (updateState.status === "deferred") {
    statusText = t("settings.updatesStatusDeferred", { version: updateState.version });
  } else if (updateState.status === "ready") {
    statusText = t("settings.updatesStatusReady", { version: updateState.version });
    statusTone = "ready";
  } else if (updateState.status === "installing") {
    statusText = t("settings.updatesStatusInstalling");
    statusTone = "ready";
  } else if (updateState.status === "portable-available") {
    statusText = t("settings.updatesStatusPortableAvailable", { version: updateState.version });
    statusTone = "ready";
  } else if (lastUpdateCheckResult === "up-to-date") {
    statusText = t("settings.updatesStatusUpToDate");
    statusTone = "success";
  } else if (lastUpdateCheckResult === "error") {
    statusText = t("settings.updatesStatusError");
    statusTone = "error";
  }

  return (
    <div className="settings-section-group" data-testid="settings-group-misc">
      <h2 className="settings-section-title">{t("settings.miscGroupTitle")}</h2>

      <section className="card settings-group-card" data-testid="settings-subgroup-misc">
        <div className="settings-subsection" data-testid="settings-subgroup-language">
          <div className="settings-group-header">
            <h3>{t("settings.languageTitle")}</h3>
            <p className="card-desc">{t("settings.languageDesc")}</p>
          </div>

          <div
            className="settings-language-selector"
            role="group"
            aria-label={t("settings.languageAria")}
          >
            <button
              type="button"
              className={`settings-language-btn ${activeLocale === "pt_BR" ? "active" : ""}`}
              data-testid="settings-lang-pt_BR"
              aria-pressed={activeLocale === "pt_BR"}
              onClick={() => onSelectLanguage("pt_BR")}
            >
              <span className="settings-language-flag-wrap">
                <LanguageFlag locale="pt_BR" variant="compact" className="settings-language-flag" />
              </span>
              <span className="settings-language-label">{t("settings.languagePtBr")}</span>
            </button>

            <button
              type="button"
              className={`settings-language-btn ${activeLocale === "en_US" ? "active" : ""}`}
              data-testid="settings-lang-en_US"
              aria-pressed={activeLocale === "en_US"}
              onClick={() => onSelectLanguage("en_US")}
            >
              <span className="settings-language-flag-wrap">
                <LanguageFlag locale="en_US" variant="compact" className="settings-language-flag" />
              </span>
              <span className="settings-language-label">{t("settings.languageEnUs")}</span>
            </button>
          </div>
        </div>

        <div className="settings-card-divider" />

        <div className="settings-subsection" data-testid="settings-subgroup-graphics">
          <div className="settings-group-header">
            <h3>{t("settings.graphicsTitle")}</h3>
            <p className="card-desc">{t("settings.graphicsDesc")}</p>
          </div>

          <div className="settings-list">
            <label className="settings-item">
              <div className="settings-item-text">
                <span className="settings-item-title">{t("settings.gpuTitle")}</span>
                <span className="settings-item-desc">{t("settings.gpuDesc")}</span>
              </div>
              <input
                type="checkbox"
                className="toggle-switch"
                aria-label={t("settings.gpuTitle")}
                data-testid="hardware-acceleration-toggle"
                checked={hardwareAcceleration}
                onChange={(e) => onHardwareAccelerationChange?.(e.target.checked)}
              />
            </label>
          </div>

          {gpuRestartRequired && (
            <div
              className="settings-restart-banner"
              role="status"
              data-testid="gpu-restart-banner"
            >
              <span className="settings-restart-text">{t("settings.gpuRestartBanner")}</span>
              <button
                type="button"
                className="settings-restart-btn"
                data-testid="gpu-restart-btn"
                onClick={() => onRestartApp?.()}
              >
                {t("settings.gpuRestartBtn")}
              </button>
            </div>
          )}
        </div>

        <div className="settings-card-divider" />

        <div className="settings-subsection" data-testid="settings-subgroup-daily-missions">
          <div className="settings-group-header">
            <h3>{t("settings.missionsTitle")}</h3>
            <p className="card-desc">{t("settings.missionsDesc")}</p>
          </div>

          <div className="settings-list">
            <label className="settings-item">
              <div className="settings-item-text">
                <span className="settings-item-title">{t("settings.missionsToggleTitle")}</span>
                <span className="settings-item-desc">{t("settings.missionsToggleDesc")}</span>
              </div>
              <input
                type="checkbox"
                className="toggle-switch"
                aria-label={t("settings.missionsToggleTitle")}
                data-testid="auto-mark-daily-missions-toggle"
                checked={autoMarkDailyMissionsRead}
                onChange={(e) =>
                  onUpdatePreference("autoMarkDailyMissionsRead", e.target.checked)
                }
              />
            </label>
          </div>
        </div>

        <div className="settings-card-divider" />

        <div className="settings-subsection" data-testid="settings-subgroup-updates">
          <div className="settings-group-header">
            <h3>{t("settings.updatesTitle")}</h3>
            <p className="card-desc">{t("settings.updatesDesc")}</p>
          </div>

          <div className="settings-list">
            <div className="settings-item settings-updates-row">
              <div className="settings-item-text">
                <div className="settings-updates-version-row">
                  <span
                    className="settings-item-title"
                    data-testid="settings-updates-version"
                  >
                    {versionLabel}
                  </span>
                  {distributionInfo && (
                    <span
                      className={`settings-updates-mode-badge ${
                        distributionInfo.isPortable ? "is-portable" : "is-installed"
                      }`}
                      data-testid="settings-updates-mode-badge"
                    >
                      {distributionInfo.isPortable
                        ? t("settings.updatesModePortable")
                        : t("settings.updatesModeInstalled")}
                    </span>
                  )}
                </div>
                <span
                  className={`settings-item-desc settings-updates-status is-${statusTone}`}
                  data-testid="settings-updates-status"
                  role="status"
                >
                  {statusText}
                </span>
              </div>

              <div className="settings-updates-actions">
                {updateState.status === "ready" || updateState.status === "installing" ? (
                  <button
                    type="button"
                    className="settings-updates-btn is-primary"
                    data-testid="settings-updates-restart-btn"
                    disabled={updateState.status === "installing"}
                    onClick={() => onRestartToUpdate?.()}
                  >
                    {updateState.status === "installing"
                      ? t("settings.updatesRestartingBtn")
                      : t("settings.updatesRestartBtn")}
                  </button>
                ) : updateState.status === "portable-available" ? (
                  <button
                    type="button"
                    className="settings-updates-btn is-primary"
                    data-testid="settings-updates-github-btn"
                    onClick={() => onOpenPortableRelease?.()}
                  >
                    {t("settings.updatesGithubBtn")}
                  </button>
                ) : (
                  <button
                    type="button"
                    className="settings-updates-btn is-secondary"
                    data-testid="settings-updates-check-btn"
                    disabled={
                      updateState.status === "checking" || updateState.status === "downloading"
                    }
                    onClick={() => onCheckForUpdates?.()}
                  >
                    {updateState.status === "checking"
                      ? t("settings.updatesCheckingBtn")
                      : t("settings.updatesCheckBtn")}
                  </button>
                )}
              </div>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}

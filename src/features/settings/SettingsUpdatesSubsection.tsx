import type { AppUpdateState, UpdateCheckResult } from "../../hooks/useAppUpdater";
import type { AppDistributionInfo } from "../../services/ipc";
import { useTranslation } from "../../i18n";

interface SettingsUpdatesSubsectionProps {
  updateState?: AppUpdateState;
  distributionInfo?: AppDistributionInfo | null;
  lastUpdateCheckResult?: UpdateCheckResult;
  lastUpdateCheckEpochSecs?: number | null;
  onCheckForUpdates?: () => void;
  onRestartToUpdate?: () => void;
  onOpenPortableRelease?: () => void;
}

function formatLastCheckDate(epochSecs: number, locale: string): string {
  const bcp47 = locale === "pt_BR" ? "pt-BR" : "en-US";
  try {
    return new Intl.DateTimeFormat(bcp47, {
      dateStyle: "short",
      timeStyle: "short",
    }).format(new Date(epochSecs * 1000));
  } catch {
    return new Date(epochSecs * 1000).toLocaleString(bcp47);
  }
}

export function SettingsUpdatesSubsection({
  updateState = { status: "idle" },
  distributionInfo = null,
  lastUpdateCheckResult = "idle",
  lastUpdateCheckEpochSecs = null,
  onCheckForUpdates,
  onRestartToUpdate,
  onOpenPortableRelease,
}: SettingsUpdatesSubsectionProps) {
  const { t, locale } = useTranslation();

  const versionLabel = distributionInfo?.currentVersion
    ? t("settings.updatesVersionLabel", { version: distributionInfo.currentVersion })
    : t("settings.updatesVersionUnknown");

  const effectiveLastCheck =
    lastUpdateCheckEpochSecs ?? distributionInfo?.lastCheckEpochSecs ?? null;

  let statusText = distributionInfo?.isPortable
    ? t("settings.updatesStatusIdlePortable")
    : typeof effectiveLastCheck === "number" && effectiveLastCheck > 0
      ? t("settings.updatesStatusIdleLastChecked", {
          date: formatLastCheckDate(effectiveLastCheck, locale),
        })
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
  );
}

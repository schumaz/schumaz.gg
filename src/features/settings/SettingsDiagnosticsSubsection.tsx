import { useTranslation } from "../../i18n";
import type { DiagnosticsFeedbackStatus } from "./useDiagnostics";

interface SettingsDiagnosticsSubsectionProps {
  copying?: boolean;
  openingFolder?: boolean;
  status?: DiagnosticsFeedbackStatus;
  onCopySummary?: () => void;
  onOpenLogsFolder?: () => void;
}

export function SettingsDiagnosticsSubsection({
  copying = false,
  openingFolder = false,
  status = "idle",
  onCopySummary,
  onOpenLogsFolder,
}: SettingsDiagnosticsSubsectionProps) {
  const { t } = useTranslation();

  let statusText = t("settings.diagnosticsStatusIdle");
  let statusTone: "neutral" | "success" | "error" = "neutral";

  if (status === "copied") {
    statusText = t("settings.diagnosticsStatusCopied");
    statusTone = "success";
  } else if (status === "copy-error") {
    statusText = t("settings.diagnosticsStatusCopyError");
    statusTone = "error";
  } else if (status === "folder-error") {
    statusText = t("settings.diagnosticsStatusFolderError");
    statusTone = "error";
  }

  return (
    <div className="settings-subsection" data-testid="settings-subgroup-diagnostics">
      <div className="settings-group-header">
        <h3>{t("settings.diagnosticsTitle")}</h3>
        <p className="card-desc">{t("settings.diagnosticsDesc")}</p>
      </div>

      <div className="settings-list">
        <div className="settings-item settings-updates-row">
          <div className="settings-item-text">
            <span className="settings-item-title">{t("settings.diagnosticsRowTitle")}</span>
            <span
              className={`settings-item-desc settings-updates-status is-${statusTone}`}
              data-testid="settings-diagnostics-status"
              role="status"
            >
              {statusText}
            </span>
          </div>

          <div className="settings-updates-actions">
            <button
              type="button"
              className="settings-updates-btn is-secondary"
              data-testid="settings-diagnostics-copy-btn"
              disabled={copying}
              onClick={() => onCopySummary?.()}
            >
              {copying ? t("settings.diagnosticsCopyingBtn") : t("settings.diagnosticsCopyBtn")}
            </button>
            <button
              type="button"
              className="settings-updates-btn is-secondary"
              data-testid="settings-diagnostics-open-logs-btn"
              disabled={openingFolder}
              onClick={() => onOpenLogsFolder?.()}
            >
              {openingFolder
                ? t("settings.diagnosticsOpeningLogsBtn")
                : t("settings.diagnosticsOpenLogsBtn")}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

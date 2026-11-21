import { useTranslation } from "../../i18n";

interface SettingsGraphicsSubsectionProps {
  hardwareAcceleration: boolean;
  onHardwareAccelerationChange?: (enabled: boolean) => void;
  gpuRestartRequired: boolean;
  onRestartApp?: () => void;
}

export function SettingsGraphicsSubsection({
  hardwareAcceleration,
  onHardwareAccelerationChange,
  gpuRestartRequired,
  onRestartApp,
}: SettingsGraphicsSubsectionProps) {
  const { t } = useTranslation();

  return (
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
  );
}

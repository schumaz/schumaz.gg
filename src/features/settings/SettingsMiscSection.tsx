import { AppLocale, UserPreferences } from "../../types";
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
}: SettingsMiscSectionProps) {
  const { t } = useTranslation();

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
      </section>
    </div>
  );
}

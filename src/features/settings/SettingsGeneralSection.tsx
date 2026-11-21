import type { AppLocale, AutostartStatus } from "../../types";
import type { AppDistributionInfo } from "../../services/ipc";
import { useTranslation } from "../../i18n";
import { LanguageFlag } from "../../components/ui/LanguageFlag";

interface SettingsGeneralSectionProps {
  activeLocale: AppLocale;
  onSelectLanguage: (locale: AppLocale) => void;
  autostartStatus?: AutostartStatus | null;
  autostartLoading?: boolean;
  autostartUpdating?: boolean;
  autostartError?: boolean;
  onAutostartChange?: (enabled: boolean) => void;
  distributionInfo?: AppDistributionInfo | null;
  closeToTray: boolean;
  onCloseToTrayChange?: (enabled: boolean) => void;
}

export function SettingsGeneralSection({
  activeLocale,
  onSelectLanguage,
  autostartStatus = null,
  autostartLoading = false,
  autostartUpdating = false,
  autostartError = false,
  onAutostartChange,
  distributionInfo = null,
  closeToTray,
  onCloseToTrayChange,
}: SettingsGeneralSectionProps) {
  const { t } = useTranslation();

  const isPortable = Boolean(autostartStatus?.isPortable || distributionInfo?.isPortable);
  const autostartSupported = autostartStatus ? autostartStatus.supported : !isPortable;
  const autostartDisabled = isPortable || !autostartSupported || autostartLoading || autostartUpdating;
  const autostartChecked = !isPortable && Boolean(autostartStatus?.enabled);

  return (
    <div className="settings-section-group" data-testid="settings-group-general">
      <h2 className="settings-section-title">{t("settings.generalGroupTitle")}</h2>

      <section className="card settings-group-card" data-testid="settings-subgroup-general">
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

        <div className="settings-subsection" data-testid="settings-subgroup-startup-tray">
          <div className="settings-group-header">
            <h3>{t("settings.startupTrayTitle")}</h3>
            <p className="card-desc">{t("settings.startupTrayDesc")}</p>
          </div>

          <div className="settings-list">
            <label
              className={`settings-item ${isPortable || !autostartSupported ? "is-disabled" : ""}`}
              aria-disabled={autostartDisabled}
            >
              <div className="settings-item-text">
                <div className="settings-updates-version-row">
                  <span className="settings-item-title">{t("settings.autostartTitle")}</span>
                  {isPortable && (
                    <span
                      className="settings-updates-mode-badge is-portable"
                      data-testid="autostart-portable-badge"
                    >
                      {t("settings.autostartPortableBadge")}
                    </span>
                  )}
                </div>
                <span
                  className={`settings-item-desc ${autostartError ? "settings-updates-status is-error" : ""}`}
                  data-testid="autostart-desc"
                >
                  {autostartError
                    ? t("settings.autostartError")
                    : isPortable
                    ? t("settings.autostartPortableDesc")
                    : t("settings.autostartDesc")}
                </span>
              </div>
              <input
                type="checkbox"
                className="toggle-switch"
                aria-label={t("settings.autostartTitle")}
                data-testid="autostart-toggle"
                checked={autostartChecked}
                disabled={autostartDisabled}
                onChange={(e) => {
                  if (autostartDisabled) return;
                  onAutostartChange?.(e.target.checked);
                }}
              />
            </label>

            <label className="settings-item">
              <div className="settings-item-text">
                <span className="settings-item-title">{t("settings.closeToTrayTitle")}</span>
                <span className="settings-item-desc">{t("settings.closeToTrayDesc")}</span>
              </div>
              <input
                type="checkbox"
                className="toggle-switch"
                aria-label={t("settings.closeToTrayTitle")}
                data-testid="close-to-tray-toggle"
                checked={closeToTray}
                onChange={(e) => onCloseToTrayChange?.(e.target.checked)}
              />
            </label>
          </div>
        </div>
      </section>
    </div>
  );
}

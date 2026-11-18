import { AppLocale, CacheClearScope, LocalCacheInfo, UserPreferences } from "../../types";
import type { AppUpdateState, UpdateCheckResult } from "../../hooks/useAppUpdater";
import type { AppDistributionInfo } from "../../services/ipc";
import { useTranslation } from "../../i18n";
import { LanguageFlag } from "../../components/ui/LanguageFlag";
import { SettingsCacheSubsection } from "./SettingsCacheSubsection";
import { SettingsUpdatesSubsection } from "./SettingsUpdatesSubsection";
import type { LocalCacheStatus } from "./useLocalCache";

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
  cacheInfo?: LocalCacheInfo | null;
  cacheLoading?: boolean;
  cacheClearing?: boolean;
  cacheStatus?: LocalCacheStatus;
  onClearCache?: (scope: CacheClearScope) => void;
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
  cacheInfo = null,
  cacheLoading = false,
  cacheClearing = false,
  cacheStatus = "idle",
  onClearCache,
  updateState = { status: "idle" },
  distributionInfo = null,
  lastUpdateCheckResult = "idle",
  onCheckForUpdates,
  onRestartToUpdate,
  onOpenPortableRelease,
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

        <div className="settings-card-divider" />

        <SettingsCacheSubsection
          cacheInfo={cacheInfo}
          cacheLoading={cacheLoading}
          cacheClearing={cacheClearing}
          cacheStatus={cacheStatus}
          onClearCache={onClearCache}
        />

        <div className="settings-card-divider" />

        <SettingsUpdatesSubsection
          updateState={updateState}
          distributionInfo={distributionInfo}
          lastUpdateCheckResult={lastUpdateCheckResult}
          onCheckForUpdates={onCheckForUpdates}
          onRestartToUpdate={onRestartToUpdate}
          onOpenPortableRelease={onOpenPortableRelease}
        />
      </section>
    </div>
  );
}

import { UserPreferences } from "../../types";
import { useTranslation } from "../../i18n";

interface SettingsRunesSectionProps {
  preferences: UserPreferences;
  patch48hProtection: boolean;
  onUpdatePreference: <K extends keyof UserPreferences>(
    key: K,
    value: UserPreferences[K]
  ) => void;
  onRequestDisablePatchProtection: () => void;
}

export function SettingsRunesSection({
  preferences,
  patch48hProtection,
  onUpdatePreference,
  onRequestDisablePatchProtection,
}: SettingsRunesSectionProps) {
  const { t } = useTranslation();

  return (
    <div className="settings-section-group" data-testid="settings-group-runes">
      <h2 className="settings-section-title">{t("settings.runesGroupTitle")}</h2>

      <section
        className="card settings-group-card settings-runes-card"
        data-testid="settings-subgroup-runes"
      >
        <div className="settings-subsection" data-testid="settings-subgroup-flash">
          <div className="settings-group-header">
            <h3>{t("settings.flashTitle")}</h3>
            <p className="card-desc">{t("settings.flashDesc")}</p>
          </div>

          <div className="segmented-control-container">
            <div
              className="segmented-control"
              role="group"
              aria-label={t("settings.flashAria")}
            >
              <button
                type="button"
                className={`segment-btn ${preferences.flashKey === "D" ? "active" : ""}`}
                onClick={() => onUpdatePreference("flashKey", "D")}
                aria-pressed={preferences.flashKey === "D"}
                aria-label="D"
              >
                <span>D</span>
              </button>
              <button
                type="button"
                className={`segment-btn ${preferences.flashKey === "F" ? "active" : ""}`}
                onClick={() => onUpdatePreference("flashKey", "F")}
                aria-pressed={preferences.flashKey === "F"}
                aria-label="F"
              >
                <span>F</span>
              </button>
            </div>
          </div>
        </div>

        <div className="settings-card-divider" />

        <div className="settings-subsection" data-testid="settings-subgroup-auto-inject">
          <div className="settings-group-header">
            <h3>{t("settings.autoInjectTitle")}</h3>
            <p className="card-desc">{t("settings.autoInjectDesc")}</p>
          </div>

          <div className="settings-list">
            <label className="settings-item">
              <div className="settings-item-text">
                <span className="settings-item-title">{t("settings.autoRunesTitle")}</span>
                <span className="settings-item-desc">{t("settings.autoRunesDesc")}</span>
              </div>
              <input
                type="checkbox"
                className="toggle-switch"
                aria-label={t("settings.autoRunesTitle")}
                checked={preferences.autoInjectRunes}
                onChange={(e) => onUpdatePreference("autoInjectRunes", e.target.checked)}
              />
            </label>

            <label className="settings-item">
              <div className="settings-item-text">
                <span className="settings-item-title">{t("settings.autoSpellsTitle")}</span>
                <span className="settings-item-desc">{t("settings.autoSpellsDesc")}</span>
              </div>
              <input
                type="checkbox"
                className="toggle-switch"
                aria-label={t("settings.autoSpellsTitle")}
                checked={preferences.autoInjectSpells}
                onChange={(e) => onUpdatePreference("autoInjectSpells", e.target.checked)}
              />
            </label>

            <label className="settings-item">
              <div className="settings-item-text">
                <span className="settings-item-title">{t("settings.autoItemsTitle")}</span>
                <span className="settings-item-desc">{t("settings.autoItemsDesc")}</span>
              </div>
              <input
                type="checkbox"
                className="toggle-switch"
                aria-label={t("settings.autoItemsTitle")}
                checked={preferences.autoInjectItems}
                onChange={(e) => onUpdatePreference("autoInjectItems", e.target.checked)}
              />
            </label>
          </div>
        </div>

        <div className="settings-card-divider" />

        <div className="settings-subsection" data-testid="settings-subgroup-patch-protection">
          <div className="settings-group-header">
            <h3>{t("settings.patchProtectionSectionTitle")}</h3>
            <p className="card-desc">{t("settings.patchProtectionSectionDesc")}</p>
          </div>

          <div className="settings-list">
            <label className="settings-item">
              <div className="settings-item-text">
                <span className="settings-item-title">
                  {t("settings.patchProtectionToggleTitle")}
                </span>
                <span className="settings-item-desc">
                  {t("settings.patchProtectionToggleDesc")}
                </span>
              </div>
              <input
                type="checkbox"
                className="toggle-switch"
                aria-label={t("settings.patchProtectionToggleTitle")}
                data-testid="patch-48h-protection-toggle"
                checked={patch48hProtection}
                onChange={(e) => {
                  if (!e.target.checked && patch48hProtection) {
                    onRequestDisablePatchProtection();
                    return;
                  }
                  onUpdatePreference("patch48hProtection", e.target.checked);
                }}
              />
            </label>
          </div>
        </div>
      </section>
    </div>
  );
}

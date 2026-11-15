import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { AppLocale, UserPreferences } from "../types";
import { clampProfileBlur } from "../hooks/usePreferences";
import { useTranslation } from "../i18n";
import { LanguageFlag } from "./LanguageFlag";

export function isBlurSliderDisabled(wallpaperVisible: boolean): boolean {
  return !wallpaperVisible;
}

interface SettingsTabProps {
  preferences: UserPreferences;
  onUpdatePreference: <K extends keyof UserPreferences>(
    key: K,
    value: UserPreferences[K]
  ) => void;
  locale?: AppLocale;
  onChangeLanguage?: (locale: AppLocale) => void;
  profileBlur?: number;
  onProfileBlurChange?: (blurPercent: number) => void;
  wallpaperVisible?: boolean;
  onWallpaperVisibleChange?: (visible: boolean) => void;
  hardwareAcceleration?: boolean;
  onHardwareAccelerationChange?: (enabled: boolean) => void;
  gpuRestartRequired?: boolean;
  onRestartApp?: () => void;
  isActive?: boolean;
}

export function SettingsTab({
  preferences,
  onUpdatePreference,
  locale: propLocale,
  onChangeLanguage,
  profileBlur = 10,
  onProfileBlurChange,
  wallpaperVisible = true,
  onWallpaperVisibleChange,
  hardwareAcceleration = true,
  onHardwareAccelerationChange,
  gpuRestartRequired = false,
  onRestartApp,
  isActive = true,
}: SettingsTabProps) {
  const { t, locale: contextLocale, setLocale } = useTranslation();
  const activeLocale = propLocale ?? preferences.language ?? contextLocale;

  const [isEditingBlur, setIsEditingBlur] = useState(false);
  const [draftBlur, setDraftBlur] = useState(String(profileBlur));
  const [showDisablePatchProtectionModal, setShowDisablePatchProtectionModal] = useState(false);
  const blurInputRef = useRef<HTMLInputElement | null>(null);
  const originalBlurRef = useRef(profileBlur);
  const blurDisabled = isBlurSliderDisabled(wallpaperVisible);
  const patch48hProtection = preferences.patch48hProtection ?? true;
  const autoMarkDailyMissionsRead = preferences.autoMarkDailyMissionsRead ?? false;

  useEffect(() => {
    if (!isEditingBlur) {
      setDraftBlur(String(profileBlur));
    }
  }, [profileBlur, isEditingBlur]);

  useEffect(() => {
    if (blurDisabled && isEditingBlur) {
      setIsEditingBlur(false);
      setDraftBlur(String(profileBlur));
    }
  }, [blurDisabled, isEditingBlur, profileBlur]);

  useEffect(() => {
    if (isEditingBlur) {
      blurInputRef.current?.focus();
      blurInputRef.current?.select();
    }
  }, [isEditingBlur]);

  useEffect(() => {
    if (!isActive || !patch48hProtection) {
      setShowDisablePatchProtectionModal(false);
    }
  }, [isActive, patch48hProtection]);

  useEffect(() => {
    if (!showDisablePatchProtectionModal) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setShowDisablePatchProtectionModal(false);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [showDisablePatchProtectionModal]);

  const handleProfileBlurChange = (nextBlur: number) => {
    if (blurDisabled) return;
    onProfileBlurChange?.(nextBlur);
  };

  const startEditingBlur = () => {
    if (blurDisabled) return;
    originalBlurRef.current = profileBlur;
    setDraftBlur(String(profileBlur));
    setIsEditingBlur(true);
  };

  const commitBlurDraft = () => {
    if (blurDisabled) {
      setIsEditingBlur(false);
      return;
    }
    const trimmed = draftBlur.trim();
    if (trimmed === "") {
      handleProfileBlurChange(0);
    } else {
      const parsed = Number(trimmed);
      if (Number.isFinite(parsed)) {
        handleProfileBlurChange(clampProfileBlur(parsed));
      } else {
        handleProfileBlurChange(originalBlurRef.current);
      }
    }
    setIsEditingBlur(false);
  };

  const cancelBlurEdit = () => {
    if (!blurDisabled) {
      handleProfileBlurChange(originalBlurRef.current);
    }
    setDraftBlur(String(originalBlurRef.current));
    setIsEditingBlur(false);
  };

  const handleSelectLanguage = (nextLocale: AppLocale) => {
    if (onChangeLanguage) {
      onChangeLanguage(nextLocale);
    } else {
      setLocale(nextLocale);
      onUpdatePreference("language", nextLocale);
    }
  };

  return (
    <div
      className={`tab-content ${isActive ? "active-tab" : "inactive-tab"}`}
      data-testid="view-settings"
      aria-hidden={!isActive}
    >
      <div className="settings-container">
        {/* Group 1: Profile */}
        <div className="settings-section-group" data-testid="settings-group-profile">
          <h2 className="settings-section-title">{t("settings.profileGroupTitle")}</h2>

          <section className="card settings-group-card" data-testid="settings-subgroup-appearance">
            <div className="settings-group-header">
              <h3>{t("settings.profileAppearanceTitle")}</h3>
              <p className="card-desc">{t("settings.profileAppearanceDesc")}</p>
            </div>

            <div className="settings-list">
              <div
                className={`settings-item settings-slider-item ${blurDisabled ? "is-disabled" : ""}`}
                aria-disabled={blurDisabled}
              >
                <div className="settings-item-text">
                  <span className="settings-item-title">{t("settings.blurTitle")}</span>
                  <span className="settings-item-desc">
                    {blurDisabled
                      ? t("settings.blurDescDisabled")
                      : t("settings.blurDescEnabled")}
                  </span>
                </div>
                <div className="settings-slider-control">
                  <input
                    type="range"
                    min={0}
                    max={100}
                    step={1}
                    value={profileBlur}
                    disabled={blurDisabled}
                    onChange={(e) => handleProfileBlurChange(Number(e.target.value))}
                    className="blur-range-slider"
                    title={blurDisabled ? t("settings.blurDisabledTitle") : undefined}
                    aria-label={t("settings.blurSliderAria")}
                    data-testid="profile-blur-slider"
                  />
                  {isEditingBlur && !blurDisabled ? (
                    <div className="slider-value-edit-wrap" data-testid="profile-blur-edit">
                      <input
                        ref={blurInputRef}
                        type="number"
                        min={0}
                        max={100}
                        step={1}
                        value={draftBlur}
                        onChange={(e) => {
                          const raw = e.target.value;
                          setDraftBlur(raw);
                          if (raw.trim() !== "") {
                            const parsed = Number(raw);
                            if (Number.isFinite(parsed)) {
                              handleProfileBlurChange(clampProfileBlur(parsed));
                            }
                          }
                        }}
                        onBlur={commitBlurDraft}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") {
                            commitBlurDraft();
                          } else if (e.key === "Escape") {
                            cancelBlurEdit();
                          }
                        }}
                        className="slider-value-input"
                        aria-label={t("settings.blurInputAria")}
                        data-testid="profile-blur-input"
                      />
                      <span className="slider-value-percent">%</span>
                    </div>
                  ) : (
                    <button
                      type="button"
                      className="slider-value-badge slider-value-btn"
                      onClick={startEditingBlur}
                      disabled={blurDisabled}
                      title={
                        blurDisabled
                          ? t("settings.blurDisabledTitle")
                          : t("settings.blurEditTitle")
                      }
                      aria-label={t("settings.blurEditAria")}
                      data-testid="profile-blur-value"
                    >
                      {profileBlur}%
                    </button>
                  )}
                </div>
              </div>

              <label className="settings-item">
                <div className="settings-item-text">
                  <span className="settings-item-title">{t("settings.wallpaperTitle")}</span>
                  <span className="settings-item-desc">{t("settings.wallpaperDesc")}</span>
                </div>
                <input
                  type="checkbox"
                  className="toggle-switch"
                  aria-label={t("settings.wallpaperTitle")}
                  data-testid="wallpaper-visible-toggle"
                  checked={wallpaperVisible}
                  onChange={(e) => onWallpaperVisibleChange?.(e.target.checked)}
                />
              </label>
            </div>
          </section>
        </div>

        {/* Group 2: Runes */}
        <div className="settings-section-group" data-testid="settings-group-runes">
          <h2 className="settings-section-title">{t("settings.runesGroupTitle")}</h2>

          <section
            className="card settings-group-card settings-runes-card"
            data-testid="settings-subgroup-runes"
          >
            {/* Subsection 2.1: Flash Placement */}
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

            {/* Subsection 2.2: Auto Injection */}
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

            {/* Subsection 2.3: Patch Protection (48h) */}
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
                        setShowDisablePatchProtectionModal(true);
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

        {/* Group 3: Miscellaneous */}
        <div className="settings-section-group" data-testid="settings-group-misc">
          <h2 className="settings-section-title">{t("settings.miscGroupTitle")}</h2>

          <section className="card settings-group-card" data-testid="settings-subgroup-misc">
            {/* Subsection 3.1: Language */}
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
                  onClick={() => handleSelectLanguage("pt_BR")}
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
                  onClick={() => handleSelectLanguage("en_US")}
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
      </div>

      {showDisablePatchProtectionModal &&
        createPortal(
          <div
            className="settings-modal-backdrop"
            data-testid="patch-48h-disable-modal-backdrop"
            onClick={(e) => {
              if (e.target === e.currentTarget) {
                setShowDisablePatchProtectionModal(false);
              }
            }}
          >
            <div
              className="card settings-modal-dialog"
              role="dialog"
              aria-modal="true"
              aria-labelledby="patch-48h-modal-title"
              aria-describedby="patch-48h-modal-desc"
              data-testid="patch-48h-protection-warning"
            >
              <div className="settings-modal-header">
                <span className="settings-warning-icon settings-modal-icon" aria-hidden="true">
                  !
                </span>
                <h3 id="patch-48h-modal-title" className="settings-modal-title">
                  {t("settings.patchModalTitle")}
                </h3>
              </div>

              <p id="patch-48h-modal-desc" className="settings-modal-desc">
                <strong>{t("settings.patchModalStrong")}</strong>
                {t("settings.patchModalBody")}
              </p>

              <div className="settings-modal-actions">
                <button
                  type="button"
                  className="settings-modal-btn settings-modal-btn-revert"
                  data-testid="patch-48h-revert-btn"
                  onClick={() => setShowDisablePatchProtectionModal(false)}
                >
                  {t("settings.patchModalRevert")}
                </button>
                <button
                  type="button"
                  className="settings-modal-btn settings-modal-btn-accept"
                  data-testid="patch-48h-accept-btn"
                  onClick={() => {
                    setShowDisablePatchProtectionModal(false);
                    onUpdatePreference("patch48hProtection", false);
                  }}
                >
                  {t("settings.patchModalAccept")}
                </button>
              </div>
            </div>
          </div>,
          document.body
        )}
    </div>
  );
}

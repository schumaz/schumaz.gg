import { useEffect, useRef, useState } from "react";
import { clampProfileBlur } from "./preferencesStorage";
import { useTranslation } from "../../i18n";
import { SettingsGraphicsSubsection } from "./SettingsGraphicsSubsection";

export function isBlurSliderDisabled(wallpaperVisible: boolean): boolean {
  return !wallpaperVisible;
}

interface SettingsAppearanceSectionProps {
  profileBlur: number;
  onProfileBlurChange?: (blurPercent: number) => void;
  wallpaperVisible: boolean;
  onWallpaperVisibleChange?: (visible: boolean) => void;
  hardwareAcceleration: boolean;
  onHardwareAccelerationChange?: (enabled: boolean) => void;
  gpuRestartRequired: boolean;
  onRestartApp?: () => void;
}

export function SettingsAppearanceSection({
  profileBlur,
  onProfileBlurChange,
  wallpaperVisible,
  onWallpaperVisibleChange,
  hardwareAcceleration,
  onHardwareAccelerationChange,
  gpuRestartRequired,
  onRestartApp,
}: SettingsAppearanceSectionProps) {
  const { t } = useTranslation();
  const [isEditingBlur, setIsEditingBlur] = useState(false);
  const [draftBlur, setDraftBlur] = useState(String(profileBlur));
  const blurInputRef = useRef<HTMLInputElement | null>(null);
  const originalBlurRef = useRef(profileBlur);
  const blurDisabled = isBlurSliderDisabled(wallpaperVisible);

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

  return (
    <div className="settings-section-group" data-testid="settings-group-appearance">
      <h2 className="settings-section-title">{t("settings.appearanceGroupTitle")}</h2>

      <section className="card settings-group-card" data-testid="settings-subgroup-appearance">
        <div className="settings-subsection" data-testid="settings-subgroup-profile-appearance">
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
        </div>

        <div className="settings-card-divider" />

        <SettingsGraphicsSubsection
          hardwareAcceleration={hardwareAcceleration}
          onHardwareAccelerationChange={onHardwareAccelerationChange}
          gpuRestartRequired={gpuRestartRequired}
          onRestartApp={onRestartApp}
        />
      </section>
    </div>
  );
}

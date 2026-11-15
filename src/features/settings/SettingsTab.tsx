import { useEffect, useState } from "react";
import { AppLocale, UserPreferences } from "../../types";
import { useTranslation } from "../../i18n";
import { SettingsProfileSection, isBlurSliderDisabled } from "./SettingsProfileSection";
import { SettingsRunesSection } from "./SettingsRunesSection";
import { SettingsMiscSection } from "./SettingsMiscSection";
import { PatchProtectionModal } from "./PatchProtectionModal";

export { isBlurSliderDisabled };

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
  const { locale: contextLocale, setLocale } = useTranslation();
  const activeLocale = propLocale ?? preferences.language ?? contextLocale;

  const [showDisablePatchProtectionModal, setShowDisablePatchProtectionModal] = useState(false);
  const patch48hProtection = preferences.patch48hProtection ?? true;
  const autoMarkDailyMissionsRead = preferences.autoMarkDailyMissionsRead ?? false;

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
        <SettingsProfileSection
          profileBlur={profileBlur}
          onProfileBlurChange={onProfileBlurChange}
          wallpaperVisible={wallpaperVisible}
          onWallpaperVisibleChange={onWallpaperVisibleChange}
        />

        <SettingsRunesSection
          preferences={preferences}
          patch48hProtection={patch48hProtection}
          onUpdatePreference={onUpdatePreference}
          onRequestDisablePatchProtection={() => setShowDisablePatchProtectionModal(true)}
        />

        <SettingsMiscSection
          activeLocale={activeLocale}
          onSelectLanguage={handleSelectLanguage}
          hardwareAcceleration={hardwareAcceleration}
          onHardwareAccelerationChange={onHardwareAccelerationChange}
          gpuRestartRequired={gpuRestartRequired}
          onRestartApp={onRestartApp}
          autoMarkDailyMissionsRead={autoMarkDailyMissionsRead}
          onUpdatePreference={onUpdatePreference}
        />
      </div>

      {showDisablePatchProtectionModal && (
        <PatchProtectionModal
          onRevert={() => setShowDisablePatchProtectionModal(false)}
          onAccept={() => {
            setShowDisablePatchProtectionModal(false);
            onUpdatePreference("patch48hProtection", false);
          }}
        />
      )}
    </div>
  );
}

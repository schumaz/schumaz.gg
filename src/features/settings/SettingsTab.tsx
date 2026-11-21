import { useEffect, useState } from "react";
import {
  AppLocale,
  AutostartStatus,
  CacheClearScope,
  LocalCacheInfo,
  UserPreferences,
} from "../../types";
import type { AppUpdateState, UpdateCheckResult } from "../../hooks/useAppUpdater";
import type { AppDistributionInfo } from "../../services/ipc";
import { useTranslation } from "../../i18n";
import { SettingsGeneralSection } from "./SettingsGeneralSection";
import { SettingsAutomationSection } from "./SettingsAutomationSection";
import { SettingsAppearanceSection, isBlurSliderDisabled } from "./SettingsAppearanceSection";
import { SettingsSystemSection } from "./SettingsSystemSection";
import { PatchProtectionModal } from "./PatchProtectionModal";
import { useLocalCache, type LocalCacheStatus } from "./useLocalCache";
import { useAutostart } from "./useAutostart";

export { isBlurSliderDisabled };

interface SettingsTabProps {
  preferences: UserPreferences;
  onUpdatePreference: <K extends keyof UserPreferences>(
    key: K,
    value: UserPreferences[K]
  ) => void;
  locale?: AppLocale;
  onChangeLanguage?: (locale: AppLocale) => void;
  closeToTray?: boolean;
  onCloseToTrayChange?: (enabled: boolean) => void;
  autostartStatus?: AutostartStatus | null;
  autostartLoading?: boolean;
  autostartUpdating?: boolean;
  autostartError?: boolean;
  onAutostartChange?: (enabled: boolean) => void;
  profileBlur?: number;
  onProfileBlurChange?: (blurPercent: number) => void;
  wallpaperVisible?: boolean;
  onWallpaperVisibleChange?: (visible: boolean) => void;
  hardwareAcceleration?: boolean;
  onHardwareAccelerationChange?: (enabled: boolean) => void;
  gpuRestartRequired?: boolean;
  onRestartApp?: () => void;
  isActive?: boolean;
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

export function SettingsTab({
  preferences,
  onUpdatePreference,
  locale: propLocale,
  onChangeLanguage,
  closeToTray = false,
  onCloseToTrayChange,
  autostartStatus: propAutostartStatus,
  autostartLoading: propAutostartLoading,
  autostartUpdating: propAutostartUpdating,
  autostartError: propAutostartError,
  onAutostartChange: propOnAutostartChange,
  profileBlur = 10,
  onProfileBlurChange,
  wallpaperVisible = true,
  onWallpaperVisibleChange,
  hardwareAcceleration = true,
  onHardwareAccelerationChange,
  gpuRestartRequired = false,
  onRestartApp,
  isActive = true,
  cacheInfo: propCacheInfo,
  cacheLoading: propCacheLoading,
  cacheClearing: propCacheClearing,
  cacheStatus: propCacheStatus,
  onClearCache: propOnClearCache,
  updateState,
  distributionInfo,
  lastUpdateCheckResult,
  onCheckForUpdates,
  onRestartToUpdate,
  onOpenPortableRelease,
}: SettingsTabProps) {
  const { locale: contextLocale, setLocale } = useTranslation();
  const activeLocale = propLocale ?? preferences.language ?? contextLocale;

  const autostart = useAutostart(isActive && propAutostartStatus === undefined);
  const autostartStatus =
    propAutostartStatus !== undefined ? propAutostartStatus : autostart.status;
  const autostartLoading = propAutostartLoading ?? autostart.loading;
  const autostartUpdating = propAutostartUpdating ?? autostart.updating;
  const autostartError = propAutostartError ?? autostart.error;
  const onAutostartChange =
    propOnAutostartChange ??
    ((enabled: boolean) => {
      void autostart.setAutostartEnabled(enabled);
    });

  const localCache = useLocalCache(isActive && propCacheInfo === undefined);
  const cacheInfo = propCacheInfo !== undefined ? propCacheInfo : localCache.cacheInfo;
  const cacheLoading = propCacheLoading ?? localCache.loading;
  const cacheClearing = propCacheClearing ?? localCache.clearing;
  const cacheStatus = propCacheStatus ?? localCache.status;
  const onClearCache =
    propOnClearCache ??
    ((scope: CacheClearScope) => {
      void localCache.clearCache(scope);
    });

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
        <SettingsGeneralSection
          activeLocale={activeLocale}
          onSelectLanguage={handleSelectLanguage}
          autostartStatus={autostartStatus}
          autostartLoading={autostartLoading}
          autostartUpdating={autostartUpdating}
          autostartError={autostartError}
          onAutostartChange={onAutostartChange}
          distributionInfo={distributionInfo}
          closeToTray={closeToTray}
          onCloseToTrayChange={onCloseToTrayChange}
        />

        <SettingsAutomationSection
          preferences={preferences}
          patch48hProtection={patch48hProtection}
          autoMarkDailyMissionsRead={autoMarkDailyMissionsRead}
          onUpdatePreference={onUpdatePreference}
          onRequestDisablePatchProtection={() => setShowDisablePatchProtectionModal(true)}
        />

        <SettingsAppearanceSection
          profileBlur={profileBlur}
          onProfileBlurChange={onProfileBlurChange}
          wallpaperVisible={wallpaperVisible}
          onWallpaperVisibleChange={onWallpaperVisibleChange}
          hardwareAcceleration={hardwareAcceleration}
          onHardwareAccelerationChange={onHardwareAccelerationChange}
          gpuRestartRequired={gpuRestartRequired}
          onRestartApp={onRestartApp}
        />

        <SettingsSystemSection
          cacheInfo={cacheInfo}
          cacheLoading={cacheLoading}
          cacheClearing={cacheClearing}
          cacheStatus={cacheStatus}
          onClearCache={onClearCache}
          updateState={updateState}
          distributionInfo={distributionInfo}
          lastUpdateCheckResult={lastUpdateCheckResult}
          onCheckForUpdates={onCheckForUpdates}
          onRestartToUpdate={onRestartToUpdate}
          onOpenPortableRelease={onOpenPortableRelease}
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

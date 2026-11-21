import type { CacheClearScope, LocalCacheInfo } from "../../types";
import type { AppUpdateState, UpdateCheckResult } from "../../hooks/useAppUpdater";
import type { AppDistributionInfo } from "../../services/ipc";
import { useTranslation } from "../../i18n";
import { SettingsCacheSubsection } from "./SettingsCacheSubsection";
import { SettingsUpdatesSubsection } from "./SettingsUpdatesSubsection";
import type { LocalCacheStatus } from "./useLocalCache";

interface SettingsSystemSectionProps {
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

export function SettingsSystemSection({
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
}: SettingsSystemSectionProps) {
  const { t } = useTranslation();

  return (
    <div className="settings-section-group" data-testid="settings-group-system">
      <h2 className="settings-section-title">{t("settings.systemGroupTitle")}</h2>

      <section className="card settings-group-card" data-testid="settings-subgroup-system">
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

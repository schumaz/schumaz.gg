import { useState } from "react";
import type { CacheClearScope, LocalCacheInfo } from "../../types";
import type { LocalCacheStatus } from "./useLocalCache";
import { ClearCacheModal } from "./ClearCacheModal";
import { useTranslation } from "../../i18n";

export function formatCacheBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unitIdx = 0;
  while (value >= 1024 && unitIdx < units.length - 1) {
    value /= 1024;
    unitIdx += 1;
  }
  const formatted =
    unitIdx === 0 ? Math.round(value).toString() : value.toFixed(value >= 100 ? 0 : 1);
  return `${formatted} ${units[unitIdx]}`;
}

interface SettingsCacheSubsectionProps {
  cacheInfo?: LocalCacheInfo | null;
  cacheLoading?: boolean;
  cacheClearing?: boolean;
  cacheStatus?: LocalCacheStatus;
  onClearCache?: (scope: CacheClearScope) => void;
}

export function SettingsCacheSubsection({
  cacheInfo = null,
  cacheLoading = false,
  cacheClearing = false,
  cacheStatus = "idle",
  onClearCache,
}: SettingsCacheSubsectionProps) {
  const { t } = useTranslation();
  const [modalOpen, setModalOpen] = useState(false);

  const sizeLabel =
    cacheLoading && !cacheInfo
      ? t("settings.cacheSizeLoading")
      : t("settings.cacheSizeLabel", {
          size: formatCacheBytes(cacheInfo?.totalBytes ?? 0),
        });

  let statusText = t("settings.cacheEmptyDesc");
  let statusTone: "neutral" | "success" | "error" = "neutral";

  if (cacheStatus === "error") {
    statusText = t("settings.cacheStatusError");
    statusTone = "error";
  } else if (cacheStatus === "cleared") {
    statusText = t("settings.cacheStatusCleared");
    statusTone = "success";
  } else if (cacheInfo && cacheInfo.totalBytes > 0) {
    statusText = t("settings.cacheBreakdown", {
      assets: formatCacheBytes(cacheInfo.assetsBytes),
      data: formatCacheBytes(cacheInfo.dataBytes),
    });
  }

  const isEmpty = cacheInfo !== null && cacheInfo.totalBytes === 0;
  const isDisabled = cacheClearing || cacheLoading || isEmpty;

  return (
    <div className="settings-subsection" data-testid="settings-subgroup-cache">
      <div className="settings-group-header">
        <h3>{t("settings.cacheTitle")}</h3>
        <p className="card-desc">{t("settings.cacheDesc")}</p>
      </div>

      <div className="settings-list">
        <div className="settings-item settings-updates-row">
          <div className="settings-item-text">
            <span className="settings-item-title" data-testid="settings-cache-size">
              {sizeLabel}
            </span>
            <span
              className={`settings-item-desc settings-updates-status is-${statusTone}`}
              data-testid="settings-cache-status"
              role="status"
            >
              {statusText}
            </span>
          </div>

          <div className="settings-updates-actions">
            <button
              type="button"
              className="settings-updates-btn is-secondary"
              data-testid="settings-cache-clear-btn"
              disabled={isDisabled}
              onClick={() => setModalOpen(true)}
            >
              {cacheClearing ? t("settings.cacheClearingBtn") : t("settings.cacheClearBtn")}
            </button>
          </div>
        </div>
      </div>

      {modalOpen && (
        <ClearCacheModal
          cacheInfo={cacheInfo}
          formatBytes={formatCacheBytes}
          onSelectScope={(scope) => {
            setModalOpen(false);
            onClearCache?.(scope);
          }}
          onClose={() => setModalOpen(false)}
        />
      )}
    </div>
  );
}

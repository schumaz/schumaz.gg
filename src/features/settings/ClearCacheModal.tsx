import { useEffect } from "react";
import { createPortal } from "react-dom";
import type { CacheClearScope, LocalCacheInfo } from "../../types";
import { useTranslation } from "../../i18n";

interface ClearCacheModalProps {
  cacheInfo: LocalCacheInfo | null;
  formatBytes: (bytes: number) => string;
  onSelectScope: (scope: CacheClearScope) => void;
  onClose: () => void;
}

export function ClearCacheModal({
  cacheInfo,
  formatBytes,
  onSelectScope,
  onClose,
}: ClearCacheModalProps) {
  const { t } = useTranslation();

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  const dataBytes = cacheInfo?.dataBytes ?? 0;
  const assetsBytes = cacheInfo?.assetsBytes ?? 0;
  const totalBytes = cacheInfo?.totalBytes ?? 0;

  return createPortal(
    <div
      className="settings-modal-backdrop"
      data-testid="clear-cache-modal-backdrop"
      onClick={(e) => {
        if (e.target === e.currentTarget) {
          onClose();
        }
      }}
    >
      <div
        className="card settings-modal-dialog settings-cache-modal-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="clear-cache-modal-title"
        aria-describedby="clear-cache-modal-desc"
        data-testid="clear-cache-modal"
      >
        <div className="settings-modal-header">
          <h3 id="clear-cache-modal-title" className="settings-modal-title">
            {t("settings.cacheModalTitle")}
          </h3>
        </div>

        <p id="clear-cache-modal-desc" className="settings-modal-desc">
          {t("settings.cacheModalDesc")}
        </p>

        <div className="settings-cache-modal-options">
          <button
            type="button"
            className="settings-cache-option-btn is-recommended"
            data-testid="clear-cache-option-builds"
            disabled={cacheInfo !== null && dataBytes === 0}
            onClick={() => onSelectScope("builds")}
          >
            <div className="settings-cache-option-head">
              <span className="settings-cache-option-title">
                {t("settings.cacheOptionBuildsTitle")}
              </span>
              <span className="settings-updates-mode-badge is-installed">
                {t("settings.cacheRecommendedBadge")}
              </span>
            </div>
            <span className="settings-cache-option-desc">
              {t("settings.cacheOptionBuildsDesc", { size: formatBytes(dataBytes) })}
            </span>
          </button>

          <button
            type="button"
            className="settings-cache-option-btn"
            data-testid="clear-cache-option-images"
            disabled={cacheInfo !== null && assetsBytes === 0}
            onClick={() => onSelectScope("images")}
          >
            <div className="settings-cache-option-head">
              <span className="settings-cache-option-title">
                {t("settings.cacheOptionImagesTitle")}
              </span>
              <span className="settings-updates-mode-badge is-portable">
                {t("settings.cacheNotRecommendedBadge")}
              </span>
            </div>
            <span className="settings-cache-option-desc">
              {t("settings.cacheOptionImagesDesc", { size: formatBytes(assetsBytes) })}
            </span>
          </button>

          <button
            type="button"
            className="settings-cache-option-btn"
            data-testid="clear-cache-option-all"
            disabled={cacheInfo !== null && totalBytes === 0}
            onClick={() => onSelectScope("all")}
          >
            <div className="settings-cache-option-head">
              <span className="settings-cache-option-title">
                {t("settings.cacheOptionAllTitle")}
              </span>
            </div>
            <span className="settings-cache-option-desc">
              {t("settings.cacheOptionAllDesc", { size: formatBytes(totalBytes) })}
            </span>
          </button>
        </div>

        <div className="settings-modal-actions">
          <button
            type="button"
            className="settings-updates-btn is-secondary"
            data-testid="clear-cache-cancel-btn"
            onClick={onClose}
          >
            {t("settings.cacheModalCancel")}
          </button>
        </div>
      </div>
    </div>,
    document.body
  );
}

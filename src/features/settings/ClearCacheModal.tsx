import { useEffect, useState } from "react";
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
  const [pendingScope, setPendingScope] = useState<CacheClearScope | null>(null);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (pendingScope !== null) {
          setPendingScope(null);
        } else {
          onClose();
        }
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose, pendingScope]);

  const dataBytes = cacheInfo?.dataBytes ?? 0;
  const assetsBytes = cacheInfo?.assetsBytes ?? 0;
  const totalBytes = cacheInfo?.totalBytes ?? 0;

  const confirmDetails: Record<
    CacheClearScope,
    { title: string; body: string; size: string }
  > = {
    builds: {
      title: t("settings.cacheOptionBuildsTitle"),
      body: t("settings.cacheConfirmBuildsBody", { size: formatBytes(dataBytes) }),
      size: formatBytes(dataBytes),
    },
    images: {
      title: t("settings.cacheOptionImagesTitle"),
      body: t("settings.cacheConfirmImagesBody", { size: formatBytes(assetsBytes) }),
      size: formatBytes(assetsBytes),
    },
    all: {
      title: t("settings.cacheOptionAllTitle"),
      body: t("settings.cacheConfirmAllBody", { size: formatBytes(totalBytes) }),
      size: formatBytes(totalBytes),
    },
  };

  const activeConfirm = pendingScope ? confirmDetails[pendingScope] : null;

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
        className={`card settings-modal-dialog ${
          pendingScope ? "settings-cache-confirm-dialog" : "settings-cache-modal-dialog"
        }`}
        role="dialog"
        aria-modal="true"
        aria-labelledby={pendingScope ? "clear-cache-confirm-title" : "clear-cache-modal-title"}
        aria-describedby={pendingScope ? "clear-cache-confirm-desc" : "clear-cache-modal-desc"}
        data-testid="clear-cache-modal"
      >
        {pendingScope && activeConfirm ? (
          <div className="settings-cache-confirm-content" data-testid="clear-cache-confirm-prompt">
            <div className="settings-modal-header">
              <span className="settings-warning-icon settings-modal-icon" aria-hidden="true">
                !
              </span>
              <h3 id="clear-cache-confirm-title" className="settings-modal-title">
                {t("settings.cacheConfirmTitle")}
              </h3>
            </div>

            <div className="settings-cache-confirm-scope">
              <span className="settings-cache-confirm-scope-title">{activeConfirm.title}</span>
              <span className="settings-cache-confirm-scope-size">{activeConfirm.size}</span>
            </div>

            <p id="clear-cache-confirm-desc" className="settings-modal-desc">
              {activeConfirm.body}
            </p>

            <div className="settings-modal-actions">
              <button
                type="button"
                className="settings-modal-btn settings-modal-btn-secondary"
                data-testid="clear-cache-confirm-back-btn"
                onClick={() => setPendingScope(null)}
              >
                {t("settings.cacheConfirmBack")}
              </button>
              <button
                type="button"
                className="settings-modal-btn settings-modal-btn-accept"
                data-testid="clear-cache-confirm-btn"
                onClick={() => onSelectScope(pendingScope)}
              >
                {t("settings.cacheConfirmAction")}
              </button>
            </div>
          </div>
        ) : (
          <>
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
                onClick={() => setPendingScope("builds")}
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
                onClick={() => setPendingScope("images")}
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
                onClick={() => setPendingScope("all")}
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
          </>
        )}
      </div>
    </div>,
    document.body
  );
}

import { createPortal } from "react-dom";
import { useTranslation } from "../../i18n";

interface PatchProtectionModalProps {
  onRevert: () => void;
  onAccept: () => void;
}

export function PatchProtectionModal({
  onRevert,
  onAccept,
}: PatchProtectionModalProps) {
  const { t } = useTranslation();

  return createPortal(
    <div
      className="settings-modal-backdrop"
      data-testid="patch-48h-disable-modal-backdrop"
      onClick={(e) => {
        if (e.target === e.currentTarget) {
          onRevert();
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
            onClick={onRevert}
          >
            {t("settings.patchModalRevert")}
          </button>
          <button
            type="button"
            className="settings-modal-btn settings-modal-btn-accept"
            data-testid="patch-48h-accept-btn"
            onClick={onAccept}
          >
            {t("settings.patchModalAccept")}
          </button>
        </div>
      </div>
    </div>,
    document.body
  );
}

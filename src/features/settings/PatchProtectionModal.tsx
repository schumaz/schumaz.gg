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
      className="modal-backdrop"
      data-testid="patch-48h-disable-modal-backdrop"
      onClick={(e) => {
        if (e.target === e.currentTarget) {
          onRevert();
        }
      }}
    >
      <div
        className="card modal-dialog is-danger"
        role="dialog"
        aria-modal="true"
        aria-labelledby="patch-48h-modal-title"
        aria-describedby="patch-48h-modal-desc"
        data-testid="patch-48h-protection-warning"
      >
        <div className="modal-header">
          <span className="modal-warning-icon" aria-hidden="true">
            !
          </span>
          <h3 id="patch-48h-modal-title" className="modal-title">
            {t("settings.patchModalTitle")}
          </h3>
        </div>

        <p id="patch-48h-modal-desc" className="modal-desc is-danger">
          <strong>{t("settings.patchModalStrong")}</strong>
          {t("settings.patchModalBody")}
        </p>

        <div className="modal-actions">
          <button
            type="button"
            className="modal-btn modal-btn-primary"
            data-testid="patch-48h-revert-btn"
            onClick={onRevert}
          >
            {t("settings.patchModalRevert")}
          </button>
          <button
            type="button"
            className="modal-btn modal-btn-danger"
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

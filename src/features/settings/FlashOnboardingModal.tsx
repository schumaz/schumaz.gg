import flashSpellIconUrl from "../../assets/spells/SummonerFlash.png";
import { useTranslation } from "../../i18n";
import { FlashKeyPreference } from "../../types";

interface FlashOnboardingModalProps {
  selectedFlashKey: FlashKeyPreference;
  onSelectFlashKey: (flashKey: FlashKeyPreference) => void;
  onConfirm: () => void;
}

export function FlashOnboardingModal({
  selectedFlashKey,
  onSelectFlashKey,
  onConfirm,
}: FlashOnboardingModalProps) {
  const { t } = useTranslation();

  return (
    <div
      className="language-onboarding-backdrop flash-onboarding-backdrop"
      data-testid="flash-onboarding-modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="flash-onboarding-title"
      aria-describedby="flash-onboarding-subtitle"
    >
      <div className="card language-onboarding-card flash-onboarding-card">
        <span className="idle-brand-pill language-onboarding-badge">{t("onboarding.badge")}</span>

        <h2 id="flash-onboarding-title" className="language-onboarding-title">
          {t("onboarding.flashTitle")}
        </h2>
        <p id="flash-onboarding-subtitle" className="language-onboarding-subtitle">
          {t("onboarding.flashSubtitle")}
        </p>

        <div
          className="language-onboarding-grid"
          role="radiogroup"
          aria-label={t("onboarding.flashAriaGroup")}
        >
          <button
            type="button"
            role="radio"
            aria-checked={selectedFlashKey === "D"}
            className={`language-flag-card flash-option-card ${selectedFlashKey === "D" ? "active" : ""}`}
            data-testid="onboarding-flash-D"
            onClick={() => onSelectFlashKey("D")}
          >
            <div className="flash-option-visual">
              <div className="flash-spell-frame">
                <img
                  src={flashSpellIconUrl}
                  alt=""
                  aria-hidden="true"
                  className="flash-spell-icon"
                  draggable={false}
                />
              </div>
              <span className="flash-keycap-badge">D</span>
            </div>
            <div className="language-flag-meta">
              <strong className="language-flag-name">{t("onboarding.flashOptionDTitle")}</strong>
              <span className="language-flag-region">{t("onboarding.flashOptionDDesc")}</span>
            </div>
          </button>

          <button
            type="button"
            role="radio"
            aria-checked={selectedFlashKey === "F"}
            className={`language-flag-card flash-option-card ${selectedFlashKey === "F" ? "active" : ""}`}
            data-testid="onboarding-flash-F"
            onClick={() => onSelectFlashKey("F")}
          >
            <div className="flash-option-visual">
              <div className="flash-spell-frame">
                <img
                  src={flashSpellIconUrl}
                  alt=""
                  aria-hidden="true"
                  className="flash-spell-icon"
                  draggable={false}
                />
              </div>
              <span className="flash-keycap-badge">F</span>
            </div>
            <div className="language-flag-meta">
              <strong className="language-flag-name">{t("onboarding.flashOptionFTitle")}</strong>
              <span className="language-flag-region">{t("onboarding.flashOptionFDesc")}</span>
            </div>
          </button>
        </div>

        <button
          type="button"
          className="language-continue-btn"
          data-testid="flash-onboarding-continue-btn"
          onClick={onConfirm}
        >
          {t("onboarding.flashConfirmBtn")}
        </button>
      </div>
    </div>
  );
}

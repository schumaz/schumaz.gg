import { AppLocale, useTranslation } from "../../i18n";
import { LanguageFlag } from "../../components/ui/LanguageFlag";

interface LanguageOnboardingModalProps {
  selectedLocale: AppLocale;
  onSelectLocale: (locale: AppLocale) => void;
  onConfirm: () => void;
}

export function LanguageOnboardingModal({
  selectedLocale,
  onSelectLocale,
  onConfirm,
}: LanguageOnboardingModalProps) {
  const { t } = useTranslation();

  return (
    <div
      className="language-onboarding-backdrop"
      data-testid="language-onboarding-modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="language-onboarding-title"
      aria-describedby="language-onboarding-subtitle"
    >
      <div className="card language-onboarding-card">
        <span className="idle-brand-pill language-onboarding-badge">{t("onboarding.badge")}</span>

        <h2 id="language-onboarding-title" className="language-onboarding-title">
          {t("onboarding.title")}
        </h2>
        <p id="language-onboarding-subtitle" className="language-onboarding-subtitle">
          {t("onboarding.subtitle")}
        </p>

        <div
          className="language-onboarding-grid"
          role="radiogroup"
          aria-label={t("onboarding.ariaGroup")}
        >
          <button
            type="button"
            role="radio"
            aria-checked={selectedLocale === "en_US"}
            className={`language-flag-card ${selectedLocale === "en_US" ? "active" : ""}`}
            data-testid="onboarding-lang-en_US"
            onClick={() => onSelectLocale("en_US")}
          >
            <div className="language-flag-frame">
              <LanguageFlag locale="en_US" className="language-flag-hero" />
            </div>
            <div className="language-flag-meta">
              <strong className="language-flag-name">{t("onboarding.englishName")}</strong>
              <span className="language-flag-region">{t("onboarding.englishRegion")}</span>
            </div>
          </button>

          <button
            type="button"
            role="radio"
            aria-checked={selectedLocale === "pt_BR"}
            className={`language-flag-card ${selectedLocale === "pt_BR" ? "active" : ""}`}
            data-testid="onboarding-lang-pt_BR"
            onClick={() => onSelectLocale("pt_BR")}
          >
            <div className="language-flag-frame">
              <LanguageFlag locale="pt_BR" className="language-flag-hero" />
            </div>
            <div className="language-flag-meta">
              <strong className="language-flag-name">{t("onboarding.portugueseName")}</strong>
              <span className="language-flag-region">{t("onboarding.portugueseRegion")}</span>
            </div>
          </button>
        </div>

        <button
          type="button"
          className="language-continue-btn"
          data-testid="onboarding-continue-btn"
          onClick={onConfirm}
        >
          {t("onboarding.continueBtn")}
        </button>
      </div>
    </div>
  );
}

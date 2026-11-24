import { useTranslation } from "../../i18n";

/**
 * Notice card rendered in place of the rune tree for modes without lobby runes (e.g. ARAM Mayhem).
 */
export function RuneMayhemNotice() {
  const { t } = useTranslation();

  return (
    <section className="card runes-tree-card mayhem-notice-card" data-testid="runes-tree">
      <div className="mayhem-notice-content">
        <div className="mayhem-badge">{t("runes.mayhemBadge")}</div>
        <h3 className="mayhem-title">{t("runes.mayhemDisabledTitle")}</h3>
        <p className="card-desc">
          {t("runes.mayhemDisabledPrefix")}
          <strong>{t("runes.mayhemDisabledMode")}</strong>
          {t("runes.mayhemDisabledMiddle")}
          <strong>{t("runes.mayhemDisabledAugments")}</strong>
          {t("runes.mayhemDisabledSuffix")}
        </p>
        <div className="mayhem-info-box">
          <div className="mayhem-info-item">
            <span className="mayhem-check">✓</span>
            <span>{t("runes.mayhemInfoSpells")}</span>
          </div>
          <div className="mayhem-info-item">
            <span className="mayhem-check">✓</span>
            <span>{t("runes.mayhemInfoItems")}</span>
          </div>
        </div>
      </div>
    </section>
  );
}

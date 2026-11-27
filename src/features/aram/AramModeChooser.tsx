import { useTranslation } from "../../i18n";
import { AramMode } from "../../types";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { LEAGUE_ARAM_ICON_URL, LEAGUE_ARAM_MAYHEM_ICON_URL } from "./aramHelpers";

interface AramModeChooserProps {
  savedMode: AramMode | null;
  onSelectMode: (mode: AramMode) => void;
}

export function AramModeChooser({ savedMode, onSelectMode }: AramModeChooserProps) {
  const { t } = useTranslation();

  return (
    <div className="aram-chooser-wrapper" data-testid="aram-mode-chooser">
      <div className="aram-chooser-header">
        <h2 className="aram-chooser-title">{t("aram.chooserTitle")}</h2>
        <p className="aram-chooser-subtitle">{t("aram.chooserSubtitle")}</p>
      </div>

      <div className="aram-chooser-grid">
        <button
          type="button"
          className={`aram-choice-card ${savedMode === "aram" ? "is-saved" : ""}`}
          data-testid="aram-choose-aram"
          onClick={() => onSelectMode("aram")}
        >
          <div className="aram-choice-top">
            <div className="aram-choice-icon" aria-hidden="true">
              <img
                src={resolveCachedAssetUrl(LEAGUE_ARAM_ICON_URL)}
                alt=""
                className="aram-choice-icon-img"
                decoding="async"
              />
            </div>
            {savedMode === "aram" && (
              <span className="aram-saved-pill">{t("aram.savedBadge")}</span>
            )}
          </div>
          <h3 className="aram-choice-name">{t("aram.aramTitle")}</h3>
          <p className="aram-choice-desc">{t("aram.aramDesc")}</p>
        </button>

        <button
          type="button"
          className={`aram-choice-card ${savedMode === "aram_mayhem" ? "is-saved" : ""}`}
          data-testid="aram-choose-mayhem"
          onClick={() => onSelectMode("aram_mayhem")}
        >
          <div className="aram-choice-top">
            <div className="aram-choice-icon is-mayhem" aria-hidden="true">
              <img
                src={resolveCachedAssetUrl(LEAGUE_ARAM_MAYHEM_ICON_URL)}
                alt=""
                className="aram-choice-icon-img"
                decoding="async"
              />
            </div>
            {savedMode === "aram_mayhem" && (
              <span className="aram-saved-pill">{t("aram.savedBadge")}</span>
            )}
          </div>
          <h3 className="aram-choice-name">{t("aram.mayhemTitle")}</h3>
          <p className="aram-choice-desc">{t("aram.mayhemDesc")}</p>
        </button>
      </div>
    </div>
  );
}

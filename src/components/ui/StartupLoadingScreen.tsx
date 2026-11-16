import appIconUrl from "../../assets/app-icon.png";
import type { StartupStageKey } from "../../hooks/useAppBootstrap";
import { useTranslation } from "../../i18n";
import "./StartupLoadingScreen.css";

interface StartupLoadingScreenProps {
  progress: number;
  stageKey: StartupStageKey;
  isExiting: boolean;
}

export function StartupLoadingScreen({
  progress,
  stageKey,
  isExiting,
}: StartupLoadingScreenProps) {
  const { t } = useTranslation();
  const clampedProgress = Math.max(0, Math.min(100, Math.round(progress)));
  const statusLabel = t(`startup.stages.${stageKey}`);

  return (
    <div
      className={`startup-overlay ${isExiting ? "is-exiting" : ""}`.trim()}
      role="dialog"
      aria-modal="true"
      aria-label={t("startup.ariaLabel")}
      data-testid="startup-loading-screen"
    >
      <div className="startup-drag-strip" data-tauri-drag-region />

      <div className="startup-card">
        <div className="startup-brand-wordmark">
          schumaz<span className="startup-brand-tld">.gg</span>
        </div>

        <div className="startup-icon-stage" aria-hidden="true">
          <div className="startup-icon-halo" />
          <div className="startup-icon-frame">
            <img
              src={appIconUrl}
              alt=""
              className="startup-icon-img"
              draggable={false}
            />
          </div>
        </div>

        <div className="startup-progress-section">
          <div
            className="startup-progress-track"
            role="progressbar"
            aria-label={t("startup.ariaLabel")}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={clampedProgress}
            data-testid="startup-progress-bar"
          >
            <div
              className="startup-progress-fill"
              style={{ width: `${clampedProgress}%` }}
            />
          </div>

          <div className="startup-status-row">
            <span className="startup-status-text" data-testid="startup-status-text">
              {statusLabel}
            </span>
            <span className="startup-status-percent" data-testid="startup-status-percent">
              {clampedProgress}%
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}

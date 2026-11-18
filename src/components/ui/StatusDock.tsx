import { useEffect, useState } from "react";
import appIconUrl from "../../assets/app-icon.png";
import type { AppUpdateState } from "../../hooks/useAppUpdater";
import { useTranslation } from "../../i18n";
import { isTauriWindowAvailable, windowIpc } from "../../services/ipc/window.ipc";
import type { SummonerInfo } from "../../types";
import { LoadingDots } from "./LoadingIndicators";

interface StatusDockProps {
  summoner: SummonerInfo | null;
  updateState?: AppUpdateState;
  onRestartUpdate?: () => void;
  onOpenPortableRelease?: () => void;
  onDismissUpdate?: () => void;
}

export function StatusDock({
  summoner,
  updateState,
  onRestartUpdate,
  onOpenPortableRelease,
  onDismissUpdate,
}: StatusDockProps) {
  const { t } = useTranslation();
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    if (!isTauriWindowAvailable()) {
      return;
    }

    let cancelled = false;
    let unlisten: (() => void) | undefined;

    const syncMaximized = async () => {
      const maximized = await windowIpc.isWindowMaximized();
      if (!cancelled) {
        setIsMaximized(maximized);
      }
    };

    void syncMaximized();
    void windowIpc
      .onWindowResized(() => {
        void syncMaximized();
      })
      .then((fn) => {
        if (cancelled) {
          fn();
        } else {
          unlisten = fn;
        }
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const handleMinimize = () => {
    void windowIpc.minimizeWindow();
  };

  const handleToggleMaximize = () => {
    void windowIpc.toggleMaximizeWindow().then(async () => {
      const maximized = await windowIpc.isWindowMaximized();
      setIsMaximized(maximized);
    });
  };

  const handleClose = () => {
    void windowIpc.closeWindow();
  };

  const showInstallerUpdatePill =
    updateState?.status === "ready" || updateState?.status === "installing";
  const showPortableUpdatePill = updateState?.status === "portable-available";

  return (
    <header className="header" data-tauri-drag-region>
      <div className="header-brand" data-tauri-drag-region>
        <img
          src={appIconUrl}
          alt=""
          aria-hidden="true"
          className="header-brand-icon"
          draggable={false}
          data-tauri-drag-region
        />
        <h1 className="title header-brand-title" data-tauri-drag-region>
          schumaz.gg
        </h1>
      </div>

      <div className="status-badge-container" data-tauri-drag-region>
        {summoner ? (
          <div className="status-badge connected" data-testid="status-connected">
            <span className="dot dot-green"></span>
            <span>{t("status.connected")}</span>
          </div>
        ) : (
          <div className="status-badge disconnected" data-testid="status-disconnected">
            <span className="dot dot-orange dot-pulsing"></span>
            <span>{t("status.waiting")}</span>
            <LoadingDots variant="warning" size="sm" testId="status-waiting-dots" />
          </div>
        )}

        {showInstallerUpdatePill && (
          <div className="update-pill update-pill-ready" data-testid="update-pill-ready">
            <button
              type="button"
              className="update-pill-action"
              data-testid="update-restart-btn"
              disabled={updateState.status === "installing"}
              onClick={() => onRestartUpdate?.()}
            >
              <span className="update-pill-dot" aria-hidden="true" />
              <span>
                {updateState.status === "installing"
                  ? t("updater.restarting")
                  : t("updater.readyToRestart", { version: updateState.version })}
              </span>
            </button>
            {updateState.status === "ready" && (
              <button
                type="button"
                className="update-pill-dismiss"
                data-testid="update-dismiss-btn"
                aria-label={t("updater.dismissAria")}
                onClick={() => onDismissUpdate?.()}
              >
                ×
              </button>
            )}
          </div>
        )}

        {showPortableUpdatePill && (
          <div className="update-pill update-pill-portable" data-testid="update-pill-portable">
            <button
              type="button"
              className="update-pill-action"
              data-testid="update-portable-btn"
              onClick={() => onOpenPortableRelease?.()}
            >
              <span className="update-pill-dot" aria-hidden="true" />
              <span>{t("updater.portableAvailable", { version: updateState.version })}</span>
            </button>
            <button
              type="button"
              className="update-pill-dismiss"
              data-testid="update-dismiss-btn"
              aria-label={t("updater.dismissAria")}
              onClick={() => onDismissUpdate?.()}
            >
              ×
            </button>
          </div>
        )}
      </div>

      <div className="window-controls">
        <button
          type="button"
          className="window-control-btn"
          data-testid="window-minimize-btn"
          aria-label={t("window.minimizeAria")}
          onClick={handleMinimize}
        >
          <svg
            width="10"
            height="10"
            viewBox="0 0 10 10"
            fill="none"
            aria-hidden="true"
            shapeRendering="crispEdges"
          >
            <line x1="0" y1="5" x2="10" y2="5" stroke="currentColor" strokeWidth="1" />
          </svg>
        </button>

        <button
          type="button"
          className="window-control-btn"
          data-testid="window-maximize-btn"
          aria-label={isMaximized ? t("window.restoreAria") : t("window.maximizeAria")}
          onClick={handleToggleMaximize}
        >
          {isMaximized ? (
            <svg
              width="10"
              height="10"
              viewBox="0 0 10 10"
              fill="none"
              aria-hidden="true"
              shapeRendering="crispEdges"
            >
              <path
                d="M2.5 2.5V0.5H9.5V7.5H7.5M0.5 2.5H7.5V9.5H0.5Z"
                stroke="currentColor"
                strokeWidth="1"
              />
            </svg>
          ) : (
            <svg
              width="10"
              height="10"
              viewBox="0 0 10 10"
              fill="none"
              aria-hidden="true"
              shapeRendering="crispEdges"
            >
              <rect x="0.5" y="0.5" width="9" height="9" stroke="currentColor" strokeWidth="1" />
            </svg>
          )}
        </button>

        <button
          type="button"
          className="window-control-btn window-control-close"
          data-testid="window-close-btn"
          aria-label={t("window.closeAria")}
          onClick={handleClose}
        >
          <svg width="10" height="10" viewBox="0 0 10 10" fill="none" aria-hidden="true">
            <path
              d="M0.5 0.5L9.5 9.5M9.5 0.5L0.5 9.5"
              stroke="currentColor"
              strokeWidth="1.15"
              strokeLinecap="square"
            />
          </svg>
        </button>
      </div>
    </header>
  );
}

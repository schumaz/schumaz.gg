import { ReactNode } from "react";
import { ChampSelectedPayload, SummonerInfo } from "../../types";
import { useTranslation } from "../../i18n";
import { LoadingDots } from "../../components/ui/LoadingIndicators";

interface IdleIslandProps {
  badge?: string;
  title: string;
  description: string;
  icon: ReactNode;
  mode?: "waiting" | "loading";
  errorMsg?: string | null;
  testId?: string;
}

export function IdleIsland({
  badge,
  title,
  description,
  icon,
  mode = "waiting",
  errorMsg,
  testId,
}: IdleIslandProps) {
  const { t } = useTranslation();
  const isWaiting = mode === "waiting";
  const isLoading = mode === "loading";

  return (
    <div className="idle-state-container" data-testid={testId}>
      <section className={`card idle-card ${isLoading ? "is-loading" : "is-waiting"}`}>
        <div
          className={`idle-icon-box ${isLoading ? "is-loading" : "is-waiting"}`}
          aria-hidden="true"
        >
          {isWaiting && <span className="idle-radar-ring" />}
          {isLoading && (
            <span className="idle-spinner-ring">
              <svg
                className="idle-spinner-svg"
                viewBox="0 0 80 80"
                fill="none"
                aria-hidden="true"
              >
                <rect
                  className="idle-spinner-track"
                  x="2"
                  y="2"
                  width="76"
                  height="76"
                  rx="20"
                  ry="20"
                />
                <rect
                  className="idle-spinner-trail"
                  x="2"
                  y="2"
                  width="76"
                  height="76"
                  rx="20"
                  ry="20"
                  pathLength={100}
                />
                <rect
                  className="idle-spinner-beam"
                  x="2"
                  y="2"
                  width="76"
                  height="76"
                  rx="20"
                  ry="20"
                  pathLength={100}
                />
              </svg>
            </span>
          )}
          <span className="idle-icon-inner">{icon}</span>
        </div>
        {badge && <span className="idle-brand-pill">{badge}</span>}
        <h2 className="idle-title">{title}</h2>
        <p className="card-desc idle-desc">{description}</p>

        {isLoading && (
          <div className="idle-activity-row" aria-hidden="true">
            <LoadingDots
              variant="accent"
              size="md"
              testId={testId ? `${testId}-dots` : "idle-waiting-dots"}
            />
          </div>
        )}

        {errorMsg && (
          <div className="alert alert-error idle-alert" role="alert">
            <strong>{t("common.errorPrefix")} </strong> {errorMsg}
          </div>
        )}
      </section>
    </div>
  );
}

interface IdleStateProps {
  summoner: SummonerInfo | null;
  errorMsg: string | null;
  champSelect?: ChampSelectedPayload | null;
  loading?: boolean;
}

export function IdleState({
  summoner: _summoner,
  errorMsg,
  champSelect,
  loading,
}: IdleStateProps) {
  const { t } = useTranslation();
  const isFetchingBuild = Boolean(champSelect || loading);

  return (
    <IdleIsland
      testId={isFetchingBuild ? "runes-loading-build" : "runes-waiting-champ"}
      badge={t("idle.runesBadge")}
      mode={isFetchingBuild ? "loading" : "waiting"}
      title={isFetchingBuild ? t("idle.loadingTitle") : t("idle.waitingTitle")}
      description={isFetchingBuild ? t("idle.loadingDesc") : t("idle.waitingDesc")}
      errorMsg={errorMsg}
      icon={
        <svg
          viewBox="0 0 24 24"
          width="42"
          height="42"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.75"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <polygon points="12 2 2 7 12 12 22 7 12 2" />
          <polyline points="2 17 12 22 22 17" />
          <polyline points="2 12 12 17 22 12" />
        </svg>
      }
    />
  );
}

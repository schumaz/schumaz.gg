import { memo, useEffect, useState } from "react";
import { useTranslation } from "../../i18n";
import { LoadingDots } from "../../components/ui/LoadingIndicators";

interface ProfileRefreshButtonProps {
  disabled: boolean;
  loading: boolean;
  cooldownUntilMs?: number;
  cooldownSecsOverride?: number;
  onRefresh: () => void;
}

function computeRemainingSecs(untilMs: number): number {
  return untilMs > 0 ? Math.max(0, Math.ceil((untilMs - Date.now()) / 1000)) : 0;
}

export const ProfileRefreshButton = memo(function ProfileRefreshButton({
  disabled,
  loading,
  cooldownUntilMs = 0,
  cooldownSecsOverride,
  onRefresh,
}: ProfileRefreshButtonProps) {
  const { t } = useTranslation();
  const [timerState, setTimerState] = useState(() => ({
    untilMs: cooldownUntilMs,
    secs: computeRemainingSecs(cooldownUntilMs),
  }));

  useEffect(() => {
    const initial = computeRemainingSecs(cooldownUntilMs);
    setTimerState({ untilMs: cooldownUntilMs, secs: initial });
    if (initial <= 0) return;

    const timer = window.setInterval(() => {
      const next = computeRemainingSecs(cooldownUntilMs);
      setTimerState({ untilMs: cooldownUntilMs, secs: next });
      if (next <= 0) {
        window.clearInterval(timer);
      }
    }, 1000);

    return () => window.clearInterval(timer);
  }, [cooldownUntilMs]);

  const syncedSecs =
    timerState.untilMs === cooldownUntilMs
      ? timerState.secs
      : computeRemainingSecs(cooldownUntilMs);
  const activeCooldownSecs =
    cooldownSecsOverride !== undefined ? cooldownSecsOverride : syncedSecs;

  return (
    <button
      type="button"
      className="profile-action-btn primary"
      data-testid="refresh-profile-btn"
      onClick={onRefresh}
      disabled={disabled || loading || activeCooldownSecs > 0}
    >
      {loading ? (
        <>
          <span>{t("profile.refreshing")}</span>
          <LoadingDots variant="inline" size="sm" testId="profile-refresh-dots" />
        </>
      ) : activeCooldownSecs > 0 ? (
        t("profile.refreshCooldown", { secs: activeCooldownSecs })
      ) : (
        t("profile.refresh")
      )}
    </button>
  );
});

import { memo } from "react";
import { PlayerProfile, SummonerInfo } from "../../types";
import { useTranslation } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { LoadingBar } from "../../components/ui/LoadingIndicators";
import { ProfileRefreshButton } from "./ProfileRefreshButton";
import { getRankedCrestUrl, getRankedMiniCrestUrl } from "./profileHelpers";

interface ProfileHeroCardProps {
  summoner: SummonerInfo | null;
  displaySummoner: SummonerInfo;
  profile: PlayerProfile | null;
  loading: boolean;
  refreshCooldownUntilMs?: number;
  refreshCooldownSecs?: number;
  onRefresh: () => void;
}

export const ProfileHeroCard = memo(function ProfileHeroCard({
  summoner,
  displaySummoner,
  profile,
  loading,
  refreshCooldownUntilMs,
  refreshCooldownSecs,
  onRefresh,
}: ProfileHeroCardProps) {
  const { t } = useTranslation();

  return (
    <section className="card profile-hero-card">
      <LoadingBar active={loading} testId="profile-loading-bar" />

      <div className="profile-hero-main">
        <div className="profile-identity">
          <div className="profile-avatar-wrap">
            {profile?.profile_icon_url ? (
              <div className="dd-icon-frame profile-avatar-frame">
                <img
                  src={resolveCachedAssetUrl(profile.profile_icon_url)}
                  alt={t("profile.summonerIconAlt")}
                  className="profile-avatar-img"
                  decoding="async"
                />
              </div>
            ) : (
              <div className="profile-avatar-placeholder">
                {(displaySummoner.gameName || displaySummoner.displayName || "?")
                  .charAt(0)
                  .toUpperCase()}
              </div>
            )}
            <span className="profile-level-badge" data-testid="profile-level">
              {t("profile.levelPrefix", { level: displaySummoner.summonerLevel })}
            </span>
          </div>

          <div className="profile-riot-id">
            <h2
              className="profile-summoner-name"
              title={`${displaySummoner.gameName || displaySummoner.displayName}${
                displaySummoner.tagLine ? `#${displaySummoner.tagLine}` : ""
              }`}
            >
              {displaySummoner.gameName || displaySummoner.displayName}
            </h2>
            {displaySummoner.tagLine && (
              <span className="profile-tagline">#{displaySummoner.tagLine}</span>
            )}
          </div>
        </div>

        <div className="profile-hero-right">
          {profile?.ranked_stats && profile.ranked_stats.length > 0 && (
            <div className="profile-ranked-row" data-testid="profile-ranked-row">
              {profile.ranked_stats.map((rank) => {
                const isUnranked = rank.tier === "UNRANKED";
                const totalRanked = rank.wins + rank.losses;
                const rankedWr =
                  totalRanked > 0 ? Math.round((rank.wins / totalRanked) * 100) : 0;
                const rankDisplay = isUnranked
                  ? t("profile.unranked")
                  : `${rank.tier_label}${rank.division ? ` ${rank.division}` : ""}`;
                const lpLabel = t("profile.lpLabel");
                const winShort = t("profile.winShort");
                const lossShort = t("profile.lossShort");

                return (
                  <div
                    key={rank.queue_type}
                    className={`ranked-pill ${isUnranked ? "unranked" : ""}`}
                    data-testid={`ranked-pill-${rank.queue_type}`}
                    title={
                      isUnranked
                        ? `${rank.queue_label}: ${t("profile.unranked")}`
                        : `${rank.queue_label}: ${rankDisplay} — ${rank.league_points} ${lpLabel} · ${rank.wins}${winShort} ${rank.losses}${lossShort} (${rankedWr}%)`
                    }
                  >
                    <div className="ranked-crest-wrap">
                      <img
                        src={getRankedCrestUrl(rank.tier)}
                        alt={t("profile.rankIconAlt", {
                          queue: rank.queue_label,
                          rank: rankDisplay,
                        })}
                        className={`ranked-crest-img ${isUnranked ? "unranked" : ""}`}
                        decoding="async"
                        onError={(e) => {
                          const img = e.currentTarget;
                          if (!img.dataset.fallbackUsed) {
                            img.dataset.fallbackUsed = "1";
                            img.src = getRankedMiniCrestUrl(rank.tier);
                          } else {
                            img.style.display = "none";
                          }
                        }}
                      />
                    </div>
                    <div className="ranked-info">
                      <span className="ranked-queue-label">{rank.queue_label}</span>
                      <span className="ranked-tier-text">{rankDisplay}</span>
                      {!isUnranked && (
                        <span className="ranked-lp-meta">
                          {rank.league_points} {lpLabel} · {rank.wins}
                          {winShort} {rank.losses}
                          {lossShort} ({rankedWr}%)
                        </span>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          )}

          <div className="profile-hero-actions">
            <ProfileRefreshButton
              disabled={!summoner}
              loading={loading}
              cooldownUntilMs={refreshCooldownUntilMs}
              cooldownSecsOverride={refreshCooldownSecs}
              onRefresh={onRefresh}
            />
          </div>
        </div>
      </div>
    </section>
  );
});

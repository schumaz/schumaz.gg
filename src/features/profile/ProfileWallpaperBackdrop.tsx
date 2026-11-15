import { useEffect, useState } from "react";
import { PlayerProfile, SummonerInfo } from "../../types";
import { resolveCachedAssetUrl } from "../../utils/assets";

interface ProfileWallpaperBackdropProps {
  activeTab: string;
  summoner: SummonerInfo | null;
  profile: PlayerProfile | null;
  wallpaperVisible: boolean;
}

export function ProfileWallpaperBackdrop({
  activeTab,
  summoner,
  profile,
  wallpaperVisible,
}: ProfileWallpaperBackdropProps) {
  const [splashFallbackUsed, setSplashFallbackUsed] = useState(false);
  const [splashBroken, setSplashBroken] = useState(false);

  useEffect(() => {
    setSplashFallbackUsed(false);
    setSplashBroken(false);
  }, [profile?.background_splash_url]);

  const activeSplashUrl = splashFallbackUsed
    ? profile?.background_splash_fallback_url || ""
    : profile?.background_splash_url || "";
  const resolvedActiveSplashUrl = resolveCachedAssetUrl(activeSplashUrl);
  const displaySummoner = summoner ?? profile?.summoner ?? null;

  const showFullScreenWallpaper =
    activeTab === "profile" &&
    Boolean(displaySummoner) &&
    wallpaperVisible &&
    Boolean(resolvedActiveSplashUrl) &&
    !splashBroken;

  if (!showFullScreenWallpaper) {
    return null;
  }

  return (
    <div className="profile-wallpaper-backdrop" data-testid="profile-wallpaper">
      <img
        src={resolvedActiveSplashUrl}
        alt=""
        aria-hidden="true"
        className="profile-wallpaper-img"
        style={{ filter: "blur(1px) saturate(1.05)" }}
        onError={() => {
          if (
            !splashFallbackUsed &&
            profile?.background_splash_fallback_url &&
            profile.background_splash_fallback_url !== activeSplashUrl
          ) {
            setSplashFallbackUsed(true);
          } else {
            setSplashBroken(true);
          }
        }}
      />
      <div className="profile-wallpaper-scrim" />
    </div>
  );
}

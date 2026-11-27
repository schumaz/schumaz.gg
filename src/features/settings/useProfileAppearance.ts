import { useCallback, useEffect, useRef, useState } from "react";
import { settingsIpc } from "../../services/ipc";
import { DEFAULT_PROFILE_BLUR, clampProfileBlur } from "./preferencesStorage";

export const PROFILE_BLUR_PERSIST_DEBOUNCE_MS = 200;

export interface ProfileBlurUpdateOptions {
  immediate?: boolean;
}

export function useProfileAppearance() {
  const [profileBlur, setProfileBlur] = useState<number>(DEFAULT_PROFILE_BLUR);
  const [wallpaperVisible, setWallpaperVisible] = useState<boolean>(true);

  const profileBlurRef = useRef<number>(DEFAULT_PROFILE_BLUR);
  const wallpaperVisibleRef = useRef<boolean>(true);
  const blurPersistTimerRef = useRef<number | undefined>(undefined);
  const blurPersistDirtyRef = useRef<boolean>(false);
  const lastPersistedRef = useRef<{ blur: number; wallpaperVisible: boolean } | null>(null);

  const clearPendingBlurTimer = () => {
    if (blurPersistTimerRef.current !== undefined) {
      window.clearTimeout(blurPersistTimerRef.current);
      blurPersistTimerRef.current = undefined;
    }
  };

  const persistNow = (blur: number, visible: boolean) => {
    clearPendingBlurTimer();
    blurPersistDirtyRef.current = false;
    if (
      lastPersistedRef.current?.blur === blur &&
      lastPersistedRef.current?.wallpaperVisible === visible
    ) {
      return;
    }
    lastPersistedRef.current = { blur, wallpaperVisible: visible };
    settingsIpc.updateProfileAppearance(blur, visible).catch(() => {});
  };

  useEffect(() => {
    return () => {
      clearPendingBlurTimer();
      if (blurPersistDirtyRef.current) {
        persistNow(profileBlurRef.current, wallpaperVisibleRef.current);
      }
    };
  }, []);

  const hydrateProfileAppearance = useCallback((blur: number, visible: boolean) => {
    clearPendingBlurTimer();
    blurPersistDirtyRef.current = false;
    profileBlurRef.current = blur;
    wallpaperVisibleRef.current = visible;
    lastPersistedRef.current = { blur, wallpaperVisible: visible };
    setProfileBlur(blur);
    setWallpaperVisible(visible);
  }, []);

  const updateProfileBlur = useCallback(
    (blurPercent: number, options?: ProfileBlurUpdateOptions) => {
      const clamped = clampProfileBlur(blurPercent);
      profileBlurRef.current = clamped;
      setProfileBlur(clamped);
      clearPendingBlurTimer();

      if (options?.immediate) {
        persistNow(clamped, wallpaperVisibleRef.current);
        return;
      }

      blurPersistDirtyRef.current = true;
      blurPersistTimerRef.current = window.setTimeout(() => {
        blurPersistTimerRef.current = undefined;
        persistNow(profileBlurRef.current, wallpaperVisibleRef.current);
      }, PROFILE_BLUR_PERSIST_DEBOUNCE_MS);
    },
    []
  );

  const updateWallpaperVisible = useCallback((visible: boolean) => {
    wallpaperVisibleRef.current = visible;
    setWallpaperVisible(visible);
    persistNow(profileBlurRef.current, visible);
  }, []);

  return {
    profileBlur,
    wallpaperVisible,
    hydrateProfileAppearance,
    updateProfileBlur,
    updateWallpaperVisible,
  };
}

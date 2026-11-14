import { useState, useEffect } from "react";
import {
  DEFAULT_PREFERENCES,
  FlashKeyPreference,
  PREFERENCES_STORAGE_KEY,
  UserPreferences,
  commands,
} from "../types";

export const PROFILE_BLUR_STORAGE_KEY = "schumaz_profile_ui_blur";
export const WALLPAPER_VISIBLE_STORAGE_KEY = "schumaz_profile_wallpaper_visible";
export const DEFAULT_PROFILE_BLUR = 10;
export const MAX_PROFILE_BLUR_PX = 32;

export function clampProfileBlur(val: number): number {
  if (!Number.isFinite(val)) return DEFAULT_PROFILE_BLUR;
  return Math.min(100, Math.max(0, Math.round(val)));
}

export function profileBlurPercentToPx(percent: number): number {
  const clamped = clampProfileBlur(percent);
  return Math.round((clamped / 100) * MAX_PROFILE_BLUR_PX * 10) / 10;
}

export function loadInitialProfileBlur(): number {
  try {
    const raw = localStorage.getItem(PROFILE_BLUR_STORAGE_KEY);
    if (raw !== null) {
      return clampProfileBlur(Number(raw));
    }
  } catch {
    // Fallback to default blur
  }
  return DEFAULT_PROFILE_BLUR;
}

export function loadInitialWallpaperVisible(): boolean {
  try {
    const raw = localStorage.getItem(WALLPAPER_VISIBLE_STORAGE_KEY);
    if (raw === "false") return false;
  } catch {
    // Ignore storage errors
  }
  return true;
}

export function loadInitialPreferences(): UserPreferences {
  try {
    const raw = localStorage.getItem(PREFERENCES_STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return {
        autoInjectRunes:
          typeof parsed.autoInjectRunes === "boolean"
            ? parsed.autoInjectRunes
            : DEFAULT_PREFERENCES.autoInjectRunes,
        autoInjectSpells:
          typeof parsed.autoInjectSpells === "boolean"
            ? parsed.autoInjectSpells
            : DEFAULT_PREFERENCES.autoInjectSpells,
        autoInjectItems:
          typeof parsed.autoInjectItems === "boolean"
            ? parsed.autoInjectItems
            : DEFAULT_PREFERENCES.autoInjectItems,
        flashKey: parsed.flashKey === "F" ? "F" : "D",
        patch48hProtection:
          typeof parsed.patch48hProtection === "boolean"
            ? parsed.patch48hProtection
            : DEFAULT_PREFERENCES.patch48hProtection,
      };
    }
  } catch {
    // Fallback to default preferences if reading localStorage fails
  }
  return DEFAULT_PREFERENCES;
}

export function usePreferences(
  onFlashKeyChange?: (newKey: FlashKeyPreference) => void,
  onPatchProtectionChange?: (enabled: boolean) => void
) {
  const [preferences, setPreferences] = useState<UserPreferences>(loadInitialPreferences);
  const [profileBlur, setProfileBlur] = useState<number>(loadInitialProfileBlur);
  const [wallpaperVisible, setWallpaperVisible] = useState<boolean>(loadInitialWallpaperVisible);

  useEffect(() => {
    // Sync initial preferences with backend
    commands.syncUserPreferences(preferences).catch(() => {});
  }, []);

  const updatePreference = <K extends keyof UserPreferences>(
    key: K,
    value: UserPreferences[K]
  ) => {
    setPreferences((prev) => {
      const next = { ...prev, [key]: value };
      try {
        localStorage.setItem(PREFERENCES_STORAGE_KEY, JSON.stringify(next));
      } catch {
        // Ignore localStorage quota errors
      }
      commands
        .syncUserPreferences(next)
        .then(() => {
          if (key === "patch48hProtection" && onPatchProtectionChange) {
            onPatchProtectionChange(Boolean(value));
          }
        })
        .catch(() => {});
      return next;
    });

    if (key === "flashKey" && onFlashKeyChange) {
      onFlashKeyChange(value as FlashKeyPreference);
    }
  };

  const updateProfileBlur = (blurPercent: number) => {
    const clamped = clampProfileBlur(blurPercent);
    setProfileBlur(clamped);
    try {
      localStorage.setItem(PROFILE_BLUR_STORAGE_KEY, String(clamped));
    } catch {
      // Ignore localStorage quota errors
    }
  };

  const updateWallpaperVisible = (visible: boolean) => {
    setWallpaperVisible(visible);
    try {
      localStorage.setItem(WALLPAPER_VISIBLE_STORAGE_KEY, String(visible));
    } catch {
      // Ignore localStorage quota errors
    }
  };

  return {
    preferences,
    updatePreference,
    profileBlur,
    updateProfileBlur,
    wallpaperVisible,
    updateWallpaperVisible,
  };
}

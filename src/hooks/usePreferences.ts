import { useState, useEffect } from "react";
import {
  DEFAULT_PREFERENCES,
  FlashKeyPreference,
  PREFERENCES_STORAGE_KEY,
  UserPreferences,
  commands,
} from "../types";

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
      };
    }
  } catch {
    // Fallback to default preferences if reading localStorage fails
  }
  return DEFAULT_PREFERENCES;
}

export function usePreferences(onFlashKeyChange?: (newKey: FlashKeyPreference) => void) {
  const [preferences, setPreferences] = useState<UserPreferences>(loadInitialPreferences);

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
      commands.syncUserPreferences(next).catch(() => {});
      return next;
    });

    if (key === "flashKey" && onFlashKeyChange) {
      onFlashKeyChange(value as FlashKeyPreference);
    }
  };

  return { preferences, updatePreference };
}

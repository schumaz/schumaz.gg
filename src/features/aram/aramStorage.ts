import { ARAM_SUBMODE_STORAGE_KEY, AramMode, NavigationStatePatch } from "../../types";

export function loadSavedAramMode(): AramMode | null {
  try {
    const raw = localStorage.getItem(ARAM_SUBMODE_STORAGE_KEY);
    if (raw === "aram" || raw === "aram_mayhem") {
      return raw;
    }
  } catch {
    // Ignore storage access errors
  }
  return null;
}

export function saveAramMode(mode: AramMode): void {
  try {
    localStorage.setItem(ARAM_SUBMODE_STORAGE_KEY, mode);
  } catch {
    // Ignore storage quota errors
  }
}

export interface UseAramPersistenceOptions {
  iniBacked?: boolean;
  savedAramMode?: AramMode | null;
  savedLastChampion?: string | null;
  onNavigationPatch?: (patch: NavigationStatePatch) => void;
}

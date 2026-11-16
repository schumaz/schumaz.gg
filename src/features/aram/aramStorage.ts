import { AramMode, NavigationStatePatch } from "../../types";

export interface UseAramPersistenceOptions {
  iniBacked?: boolean;
  savedAramMode?: AramMode | null;
  savedLastChampion?: string | null;
  onNavigationPatch?: (patch: NavigationStatePatch) => void;
}

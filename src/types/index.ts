export type ActiveTab = "runes" | "settings";

export type {
  BuildInjected,
  ChampSelectEnded,
  ChampSelected,
  ChampSelectedPayload,
  ChampionBuild,
  FlashKeyPreference,
  InjectionReport,
  ItemBlock,
  ItemBuildOption,
  LcuConnected,
  LcuDisconnected,
  QueueMode,
  RecommendedBuild,
  RuneMetadata,
  RunePageOption,
  SpellMetadata,
  SummonerInfo,
  UnifiedBuildOption,
  UserPreferences,
} from "./bindings";

export { commands, events } from "./bindings";

import type { UserPreferences } from "./bindings";

export const DEFAULT_PREFERENCES: UserPreferences = {
  autoInjectRunes: true,
  autoInjectSpells: true,
  autoInjectItems: true,
  flashKey: "D",
};

export const PREFERENCES_STORAGE_KEY = "schumaz_user_preferences";

export type ActiveTab = "profile" | "runes" | "ranked" | "aram" | "settings";

export type {
  AppLocale,
  AppSettings,
  AramAugmentEntry,
  AramChampionRank,
  AramMode,
  BuildInjected,
  CachedProfileResult,
  ChampSelectEnded,
  ChampSelectReconciled,
  ChampSelectReconciledPayload,
  ChampSelected,
  ChampSelectedPayload,
  ChampionBuild,
  ChampionMasteryInfo,
  FlashKeyPreference,
  GameflowPhaseChanged,
  GeneralSettings,
  GraphicsSettings,
  InjectionReport,
  ItemBlock,
  ItemBuildOption,
  LcuConnected,
  LcuDisconnected,
  LegacyLocalStoragePayload,
  MatchEnded,
  NavigationState,
  NavigationStatePatch,
  PlayerProfile,
  ProfileAppearanceSettings,
  ProfileRefreshRequested,
  QueueMode,
  RankedChampionEntry,
  RankedLane,
  RankedQueueStats,
  RankedTierListResponse,
  RecentMatchInfo,
  RecommendedBuild,
  RuneMetadata,
  RunePageOption,
  SpellMetadata,
  SummonerInfo,
  UnifiedBuildOption,
  UserPreferences,
  WindowSettings,
} from "./bindings";

export { commands, events } from "./bindings";

import type { UserPreferences } from "./bindings";

export const DEFAULT_PREFERENCES: UserPreferences = {
  autoInjectRunes: true,
  autoInjectSpells: true,
  autoInjectItems: true,
  flashKey: "D",
  patch48hProtection: true,
  autoMarkDailyMissionsRead: false,
  language: "en_US",
};

export const PREFERENCES_STORAGE_KEY = "schumaz_user_preferences";
export const LANGUAGE_STORAGE_KEY = "schumaz_language";
export const ARAM_SUBMODE_STORAGE_KEY = "schumaz_aram_submode";
export const RANKED_FILTERS_STORAGE_KEY = "schumaz_ranked_filters";

export type ActiveTab = "profile" | "runes" | "settings";

export type {
  BuildInjected,
  ChampSelectEnded,
  ChampSelectReconciled,
  ChampSelectReconciledPayload,
  ChampSelected,
  ChampSelectedPayload,
  ChampionBuild,
  ChampionMasteryInfo,
  FlashKeyPreference,
  GameflowPhaseChanged,
  InjectionReport,
  ItemBlock,
  ItemBuildOption,
  LcuConnected,
  LcuDisconnected,
  MatchEnded,
  PlayerProfile,
  ProfileRefreshRequested,
  QueueMode,
  RankedQueueStats,
  RecentMatchInfo,
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
  patch48hProtection: true,
};

export const PREFERENCES_STORAGE_KEY = "schumaz_user_preferences";

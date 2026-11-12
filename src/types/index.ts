export type ActiveTab = "runes" | "settings";
export type FlashKeyPreference = "D" | "F";

export interface UserPreferences {
  autoInjectRunes: boolean;
  autoInjectSpells: boolean;
  autoInjectItems: boolean;
  flashKey: FlashKeyPreference;
}

export const DEFAULT_PREFERENCES: UserPreferences = {
  autoInjectRunes: true,
  autoInjectSpells: true,
  autoInjectItems: true,
  flashKey: "D",
};

export const PREFERENCES_STORAGE_KEY = "schumaz_user_preferences";

export interface SummonerInfo {
  summonerId: number;
  gameName: string;
  tagLine: string;
  displayName: string;
  summonerLevel: number;
  profileIconId: number;
}

export interface InjectionReport {
  champion_name: string;
  runes_injected: boolean;
  spells_injected: boolean;
  items_injected: boolean;
  message: string;
}

export interface ChampSelectedPayload {
  champion_id: number;
  queue_id: number;
  queue_mode: string;
  locked_in: boolean;
  assigned_position?: string;
}

export interface RuneMetadata {
  id: number;
  key: string;
  name: string;
  icon_url: string;
}

export interface SpellMetadata {
  id: number;
  name: string;
  icon_url: string;
}

export interface RunePageOption {
  primary_style_id: number;
  sub_style_id: number;
  selected_perk_ids: number[];
  play: number;
  win: number;
  pick_rate: number;
  primary_style_name?: string;
  primary_style_icon_url?: string;
  sub_style_name?: string;
  sub_style_icon_url?: string;
  perks?: RuneMetadata[];
}

export interface UnifiedBuildOption {
  name: string;
  core_item_ids: number[];
  core_item_icon_urls: string[];
  final_item_ids: number[];
  final_item_icon_urls: string[];
  situational_item_ids?: number[];
  situational_item_icon_urls?: string[];
  play: number;
  win: number;
  pick_rate: number;
}

export interface ItemBuildOption {
  item_ids: number[];
  item_icon_urls?: string[];
  play: number;
  win: number;
  pick_rate: number;
}

export interface ItemBlock {
  title: string;
  item_ids: number[];
  item_icon_urls?: string[];
}

export interface ChampionBuild {
  champion_id: number;
  champion_name: string;
  win_rate: number;
  pick_rate: number;
  primary_style_id: number;
  sub_style_id: number;
  selected_perk_ids: number[];
  spell1_id: number;
  spell2_id: number;
  item_blocks: ItemBlock[];
  champion_icon_url?: string;
  primary_style_name?: string;
  primary_style_icon_url?: string;
  sub_style_name?: string;
  sub_style_icon_url?: string;
  perks?: RuneMetadata[];
  spells?: SpellMetadata[];
  spell_choices?: SpellMetadata[];
  rune_pages?: RunePageOption[];
  build_options?: UnifiedBuildOption[];
  core_build_options?: ItemBuildOption[];
  final_build_options?: ItemBuildOption[];
  active_position?: string;
  active_tier?: string;
  active_region?: string;
  active_patch?: string;
  available_patches?: string[];
  patch_fallback_applied?: boolean;
  supports_runes?: boolean;
}

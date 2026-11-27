use super::arena::ArenaAugmentMetadata;
use super::build::{ItemMetadata, RuneMetadata, SpellMetadata};
use super::lcu::SummonerInfo;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;

/// Ranked queue summary (Solo/Duo or Flex) from `/lol-ranked/v1/current-ranked-stats`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
pub struct RankedQueueStats {
    pub queue_type: String,
    pub queue_label: String,
    pub tier: String,
    pub tier_label: String,
    pub division: String,
    pub league_points: i32,
    pub wins: i32,
    pub losses: i32,
}

/// Champion mastery entry from `/lol-champion-mastery/v1/local-player/champion-masteries`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
pub struct ChampionMasteryInfo {
    pub champion_id: i32,
    pub champion_name: String,
    pub champion_icon_url: String,
    pub champion_level: u32,
    pub champion_points: u64,
    #[serde(default)]
    pub last_play_time_ms: i64,
    #[serde(default)]
    pub champion_points_since_last_level: u64,
    #[serde(default)]
    pub champion_points_until_next_level: u64,
    #[serde(default)]
    pub tokens_earned: u32,
    #[serde(default)]
    pub mark_required_for_next_level: u32,
}

/// Individual recent match entry from LCU match history (Practice Tool strictly excluded).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
pub struct RecentMatchInfo {
    pub game_id: u64,
    pub queue_id: i32,
    pub game_mode_key: String,
    pub game_mode_label: String,
    pub champion_id: i32,
    pub champion_name: String,
    pub champion_icon_url: String,
    pub win: bool,
    #[serde(default)]
    pub remake: bool,
    #[serde(default)]
    pub arena_placement: Option<i32>,
    pub kills: i32,
    pub deaths: i32,
    pub assists: i32,
    pub cs: i32,
    pub game_duration_secs: i64,
    pub game_creation_ms: i64,
    pub spell1_id: i32,
    pub spell1_icon_url: String,
    pub spell2_id: i32,
    pub spell2_icon_url: String,
    #[serde(default)]
    pub primary_rune_id: i32,
    #[serde(default)]
    pub primary_rune_icon_url: String,
    #[serde(default)]
    pub primary_style_id: i32,
    #[serde(default)]
    pub primary_style_icon_url: String,
    #[serde(default)]
    pub sub_style_id: i32,
    #[serde(default)]
    pub sub_style_icon_url: String,
    #[serde(default)]
    pub augment_ids: Vec<i32>,
    pub item_ids: Vec<i32>,
    pub item_icon_urls: Vec<String>,
    #[serde(default)]
    pub trinket_id: i32,
    #[serde(default)]
    pub trinket_icon_url: String,
    pub gold_earned: i64,
    pub total_damage_dealt_to_champions: i64,
    pub vision_score: i64,
    pub wards_placed: i32,
    pub champ_level: i32,
    pub double_kills: i32,
    pub triple_kills: i32,
    pub quadra_kills: i32,
    pub penta_kills: i32,
}

/// Aggregated player profile payload for the Profile tab.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
pub struct PlayerProfile {
    pub summoner: SummonerInfo,
    pub profile_icon_url: String,
    pub background_skin_id: i32,
    pub background_skin_name: String,
    pub background_splash_url: String,
    pub background_splash_fallback_url: String,
    pub ranked_stats: Vec<RankedQueueStats>,
    pub top_masteries: Vec<ChampionMasteryInfo>,
    #[serde(default)]
    pub all_masteries: Vec<ChampionMasteryInfo>,
    pub recent_matches: Vec<RecentMatchInfo>,
    #[serde(default)]
    pub history_next_beg_index: u32,
    #[serde(default)]
    pub history_has_more: bool,
    #[serde(default)]
    pub item_metadata: HashMap<i32, ItemMetadata>,
    #[serde(default)]
    pub rune_metadata: HashMap<i32, RuneMetadata>,
    #[serde(default)]
    pub spell_metadata: HashMap<i32, SpellMetadata>,
    #[serde(default)]
    pub augment_metadata: HashMap<i32, ArenaAugmentMetadata>,
}

/// Paginated match history window returned by `get_player_match_history_page`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
pub struct MatchHistoryPage {
    pub matches: Vec<RecentMatchInfo>,
    pub next_beg_index: u32,
    pub has_more: bool,
    #[serde(default)]
    pub item_metadata: HashMap<i32, ItemMetadata>,
    #[serde(default)]
    pub rune_metadata: HashMap<i32, RuneMetadata>,
    #[serde(default)]
    pub spell_metadata: HashMap<i32, SpellMetadata>,
    #[serde(default)]
    pub augment_metadata: HashMap<i32, ArenaAugmentMetadata>,
}

/// Cached profile envelope returned by `get_cached_player_profile` for instant startup & SWR.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct CachedProfileResult {
    pub profile: PlayerProfile,
    pub created_at_unix: u64,
    pub is_stale: bool,
}

/// Full scoreboard participant row parsed from `/lol-match-history/v1/games/{gameId}`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
pub struct ScoreboardParticipant {
    pub participant_id: i32,
    pub team_id: i32,
    #[serde(default)]
    pub subteam_id: i32,
    #[serde(default)]
    pub arena_placement: Option<i32>,
    pub is_local_player: bool,
    pub riot_id_name: String,
    #[serde(default)]
    pub riot_id_tag: String,
    pub champion_id: i32,
    pub champion_name: String,
    pub champion_icon_url: String,
    pub champ_level: i32,
    pub win: bool,
    pub kills: i32,
    pub deaths: i32,
    pub assists: i32,
    pub kp_percent: i32,
    pub cs: i32,
    pub gold_earned: i64,
    pub total_damage_dealt_to_champions: i64,
    pub vision_score: i64,
    pub wards_placed: i32,
    #[serde(default)]
    pub control_wards_bought: i32,
    pub spell1_id: i32,
    pub spell1_icon_url: String,
    pub spell2_id: i32,
    pub spell2_icon_url: String,
    #[serde(default)]
    pub primary_rune_id: i32,
    #[serde(default)]
    pub primary_rune_icon_url: String,
    #[serde(default)]
    pub primary_style_id: i32,
    #[serde(default)]
    pub primary_style_icon_url: String,
    #[serde(default)]
    pub sub_style_id: i32,
    #[serde(default)]
    pub sub_style_icon_url: String,
    #[serde(default)]
    pub augment_ids: Vec<i32>,
    pub item_ids: Vec<i32>,
    pub item_icon_urls: Vec<String>,
    #[serde(default)]
    pub trinket_id: i32,
    #[serde(default)]
    pub trinket_icon_url: String,
}

/// Aggregated 5v5 / ARAM team summary (`100` Blue vs `200` Red) inside a match scoreboard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
pub struct ScoreboardTeamSummary {
    pub team_id: i32,
    pub win: bool,
    pub total_kills: i32,
    pub total_deaths: i32,
    pub total_assists: i32,
    pub total_gold: i64,
    pub towers: i32,
    pub dragons: i32,
    pub barons: i32,
    pub heralds: i32,
    #[serde(default)]
    pub voidgrubs: i32,
    pub inhibitors: i32,
    pub participants: Vec<ScoreboardParticipant>,
}

/// Aggregated Arena duo summary (`1st`..`8th` placement) inside a match scoreboard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
pub struct ScoreboardArenaTeam {
    pub subteam_id: i32,
    pub placement: i32,
    pub win: bool,
    pub total_kills: i32,
    pub total_deaths: i32,
    pub total_assists: i32,
    pub total_damage: i64,
    pub participants: Vec<ScoreboardParticipant>,
}

/// Full on-demand match scoreboard returned by `get_match_scoreboard_detail`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
pub struct MatchScoreboardDetail {
    pub game_id: u64,
    pub queue_id: i32,
    pub map_id: i32,
    pub game_mode_key: String,
    pub game_mode_label: String,
    pub game_duration_secs: i64,
    pub game_creation_ms: i64,
    pub is_arena: bool,
    #[serde(default)]
    pub remake: bool,
    pub max_damage_dealt_to_champions: i64,
    #[serde(default)]
    pub teams: Vec<ScoreboardTeamSummary>,
    #[serde(default)]
    pub arena_teams: Vec<ScoreboardArenaTeam>,
    #[serde(default)]
    pub item_metadata: HashMap<i32, ItemMetadata>,
    #[serde(default)]
    pub rune_metadata: HashMap<i32, RuneMetadata>,
    #[serde(default)]
    pub spell_metadata: HashMap<i32, SpellMetadata>,
    #[serde(default)]
    pub augment_metadata: HashMap<i32, ArenaAugmentMetadata>,
}

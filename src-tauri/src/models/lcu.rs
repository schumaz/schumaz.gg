use serde::{Deserialize, Serialize};
use specta::Type;

/// Basic summoner details parsed from `/lol-summoner/v1/current-summoner`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
pub struct SummonerInfo {
    #[serde(rename = "summonerId")]
    pub summoner_id: u64,
    #[serde(rename = "gameName", default)]
    pub game_name: String,
    #[serde(rename = "tagLine", default)]
    pub tag_line: String,
    #[serde(rename = "displayName", default)]
    pub display_name: String,
    #[serde(rename = "summonerLevel")]
    pub summoner_level: u32,
    #[serde(rename = "profileIconId")]
    pub profile_icon_id: u32,
}

/// Connection state of the LCU.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(tag = "status", content = "data")]
pub enum LcuStatus {
    Disconnected,
    Connected(SummonerInfo),
}

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

/// Top champion mastery entry from `/lol-champion-mastery/v1/local-player/champion-masteries`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
pub struct ChampionMasteryInfo {
    pub champion_id: i32,
    pub champion_name: String,
    pub champion_icon_url: String,
    pub champion_level: u32,
    pub champion_points: u64,
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
    pub item_ids: Vec<i32>,
    pub item_icon_urls: Vec<String>,
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
    pub recent_matches: Vec<RecentMatchInfo>,
}

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

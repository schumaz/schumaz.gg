use serde::{Deserialize, Serialize};
use specta::Type;

/// Individual lane matchup record between the active champion and an opponent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Type)]
pub struct LaneMatchupEntry {
    pub champion_id: i32,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub icon_url: String,
    #[serde(default)]
    pub play: i64,
    #[serde(default)]
    pub win: i64,
    /// Active champion's win rate against this opponent (`0.0..=100.0`, 1 decimal place).
    #[serde(default)]
    pub win_rate: f64,
}

/// Top hard counters and favorable lane matchups for Summoner's Rift lanes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Type)]
pub struct LaneMatchupsSummary {
    #[serde(default)]
    pub hard: Vec<LaneMatchupEntry>,
    #[serde(default)]
    pub favorable: Vec<LaneMatchupEntry>,
}

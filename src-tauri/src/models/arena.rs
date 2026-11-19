use serde::{Deserialize, Serialize};
use specta::Type;

/// Localized Arena (`CHERRY`) augment catalog entry from `cherry-augments.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
pub struct ArenaAugmentMetadata {
    pub id: i32,
    pub name: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub icon_url: String,
    /// Canonical rarity matching OP.GG (`1` = Silver, `4` = Gold, `8` = Prismatic).
    #[serde(default)]
    pub rarity: i32,
}

/// Ranked champion entry in the Arena (`CHERRY`) tier list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Type)]
pub struct ArenaChampionEntry {
    pub id: i32,
    pub key: String,
    pub name: String,
    pub icon_url: String,
    /// Canonical 0-based tier (`0` = OP, `1` = Tier 1, ..., `5` = Tier 5).
    pub tier: i32,
    pub rank: i32,
    /// Top 4 finish rate (0–100%).
    pub win_rate: Option<f64>,
    /// 1st place finish rate (0–100%).
    pub first_place_rate: Option<f64>,
    /// Average placement (e.g. `3.41`).
    pub avg_placement: Option<f64>,
    pub pick_rate: Option<f64>,
    pub ban_rate: Option<f64>,
    #[serde(default)]
    pub play: i64,
}

/// Full Arena (`CHERRY`) tier list payload including active patch and available patches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Type)]
pub struct ArenaTierListResponse {
    pub active_patch: String,
    pub available_patches: Vec<String>,
    pub patch_fallback_applied: bool,
    pub patch_protection_remaining_hours: Option<u32>,
    pub champions: Vec<ArenaChampionEntry>,
}

/// Recommended Arena (`CHERRY`) augment entry with stats and localized metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Type)]
pub struct ArenaAugmentEntry {
    pub id: i32,
    pub name: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub icon_url: String,
    /// Canonical rarity (`1` = Silver, `4` = Gold, `8` = Prismatic).
    pub rarity: i32,
    pub play: i64,
    pub win: i64,
    pub first_place: i64,
    /// Top 4 finish rate (0–100%).
    pub win_rate: f64,
    /// 1st place finish rate (0–100%).
    pub first_place_rate: f64,
    /// Average placement (e.g. `3.25`).
    pub avg_placement: f64,
    pub pick_rate: f64,
}

/// Recommended Arena (`CHERRY`) duo partner synergy entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Type)]
pub struct ArenaSynergyEntry {
    pub champion_id: i32,
    pub key: String,
    pub name: String,
    pub icon_url: String,
    pub op_rank: i32,
    pub play: i64,
    pub win: i64,
    pub first_place: i64,
    /// Top 4 finish rate (0–100%).
    pub win_rate: f64,
    /// 1st place finish rate (0–100%).
    pub first_place_rate: f64,
    /// Average placement (e.g. `2.99`).
    pub avg_placement: f64,
    pub pick_rate: f64,
}

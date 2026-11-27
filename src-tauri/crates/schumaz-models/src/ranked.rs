use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Type)]
#[serde(rename_all = "snake_case")]
pub enum RankedLane {
    Top,
    Jungle,
    Mid,
    Adc,
    Support,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Type)]
pub struct RankedChampionEntry {
    pub id: i32,
    pub key: String,
    pub name: String,
    pub icon_url: String,
    pub lane: RankedLane,
    pub tier: i32,
    pub rank: i32,
    pub overall_rank: i32,
    pub win_rate: Option<f64>,
    pub pick_rate: Option<f64>,
    pub ban_rate: Option<f64>,
    pub role_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Type)]
pub struct RankedTierListResponse {
    pub region: String,
    pub tier: String,
    pub active_patch: String,
    pub available_patches: Vec<String>,
    pub patch_fallback_applied: bool,
    pub patch_protection_remaining_hours: Option<u32>,
    pub champions: Vec<RankedChampionEntry>,
}

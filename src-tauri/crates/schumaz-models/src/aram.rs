use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Type)]
#[serde(rename_all = "snake_case")]
pub enum AramMode {
    Aram,
    AramMayhem,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Type)]
pub struct AramChampionRank {
    pub id: i32,
    pub key: String,
    pub name: String,
    pub icon_url: String,
    pub tier: i32,
    pub rank: i32,
    pub win_rate: Option<f64>,
    pub pick_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Type)]
pub struct AramAugmentEntry {
    pub id: i32,
    pub name: String,
    pub tier: i32,
    pub rarity: Option<i32>,
    pub performance: f64,
    pub popular: f64,
    pub icon_url: String,
    pub desc: String,
}

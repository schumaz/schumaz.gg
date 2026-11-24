use serde::{Deserialize, Serialize};
use specta::Type;

/// Localized metadata for a champion's active ability (`Q`, `W`, `E`, or `R`) from DataDragon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
pub struct ChampionAbilityMetadata {
    /// Canonical slot key: `"Q"`, `"W"`, `"E"`, or `"R"`.
    pub key: String,
    /// Localized ability name from DataDragon.
    pub name: String,
    /// Official DataDragon ability icon URL (`https://ddragon.leagueoflegends.com/cdn/{ver}/img/spell/{img}`).
    #[serde(default)]
    pub icon_url: String,
    /// Sanitized plain-text description from DataDragon `spells[].description` for rich tooltips.
    #[serde(default)]
    pub description: String,
    /// Cooldown progression string from DataDragon (e.g. `"7"` or `"14/13/12/11/10"`).
    #[serde(default)]
    pub cooldown_burn: String,
    /// Maximum rank for this ability (normally 5 for Q/W/E, 3 for R; 6 for kits like Jayce/Udyr/Yuumi).
    #[serde(default)]
    pub max_rank: i32,
}

/// Recommended skill maxing priority (e.g. `Q > E > W`) and level-by-level leveling order (`1..=18`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Type)]
pub struct SkillOrderRecommendation {
    /// Maxing priority order of basic abilities (e.g. `["Q", "E", "W"]`).
    #[serde(default)]
    pub priority: Vec<String>,
    /// Level-by-level skill picks from level 1 up to 18 (e.g. `["Q", "W", "E", "Q", ...]`).
    #[serde(default)]
    pub order: Vec<String>,
    /// Total games recorded for the chosen skill mastery / order.
    #[serde(default)]
    pub play: i64,
    /// Total wins recorded for the chosen skill mastery / order.
    #[serde(default)]
    pub win: i64,
    /// Pick rate normalized to percentage `0.0..=100.0` (1 decimal place).
    #[serde(default)]
    pub pick_rate: f64,
    /// Win rate normalized to percentage `0.0..=100.0` (1 decimal place).
    #[serde(default)]
    pub win_rate: f64,
    /// Localized ability metadata for `Q`, `W`, `E`, `R` hydrated from DataDragon.
    #[serde(default)]
    pub abilities: Vec<ChampionAbilityMetadata>,
}

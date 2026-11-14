use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemClassificationCache {
    pub boots: std::collections::HashSet<i32>,
    pub components: std::collections::HashSet<i32>,
    pub completed: std::collections::HashSet<i32>,
    pub evolved_fallbacks: std::collections::HashMap<i32, i32>,
    pub item_tags: std::collections::HashMap<i32, Vec<String>>,
}

/// Champion metadata translated from Riot's DataDragon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
pub struct ChampionMetadata {
    pub champion_id: i32,
    pub key_name: String,
    pub display_name: String,
    pub icon_url: String,
}

/// Metadata for a rune or style from DataDragon's `runesReforged.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
pub struct RuneMetadata {
    pub id: i32,
    pub key: String,
    pub name: String,
    pub icon_url: String,
}

/// Metadata for a summoner spell.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
pub struct SpellMetadata {
    pub id: i32,
    pub name: String,
    pub icon_url: String,
}

/// Metadata for a complete 9-perk rune page option (keystone, primary tree, secondary tree, stat mods).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Type)]
pub struct RunePageOption {
    pub primary_style_id: i32,
    pub sub_style_id: i32,
    pub selected_perk_ids: Vec<i32>,
    pub play: i64,
    pub win: i64,
    pub pick_rate: f64,
    #[serde(default)]
    pub primary_style_name: String,
    #[serde(default)]
    pub primary_style_icon_url: String,
    #[serde(default)]
    pub sub_style_name: String,
    #[serde(default)]
    pub sub_style_icon_url: String,
    #[serde(default)]
    pub perks: Vec<RuneMetadata>,
}

/// In-memory cache holding champion, rune, and summoner spell metadata from DataDragon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
pub struct DataDragonCache {
    pub version: String,
    pub champions: HashMap<i32, ChampionMetadata>,
    pub runes: HashMap<i32, RuneMetadata>,
    #[serde(default)]
    pub spells: HashMap<i32, SpellMetadata>,
}

/// Complete unified build archetype (Core 3 + matching 6-item Final Build).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Type)]
pub struct UnifiedBuildOption {
    pub name: String,
    pub core_item_ids: Vec<i32>,
    #[serde(default)]
    pub core_item_icon_urls: Vec<String>,
    pub final_item_ids: Vec<i32>,
    #[serde(default)]
    pub final_item_icon_urls: Vec<String>,
    #[serde(default)]
    pub situational_item_ids: Vec<i32>,
    #[serde(default)]
    pub situational_item_icon_urls: Vec<String>,
    pub play: i64,
    pub win: i64,
    pub pick_rate: f64,
}

/// Metadata for a multi-option item build (e.g. Core 3 or Final 6).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Type)]
pub struct ItemBuildOption {
    pub item_ids: Vec<i32>,
    #[serde(default)]
    pub item_icon_urls: Vec<String>,
    pub play: i64,
    pub win: i64,
    pub pick_rate: f64,
}

/// An individual item group for the shop (e.g. Starters, Core 3, Final Build, Boots).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
pub struct ItemBlock {
    pub title: String,
    pub item_ids: Vec<i32>,
    #[serde(default)]
    pub item_icon_urls: Vec<String>,
}

/// Complete build recommendations fetched and parsed from OP.GG enriched with DataDragon visual assets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Type)]
pub struct ChampionBuild {
    pub champion_id: i32,
    pub champion_name: String,
    pub win_rate: f64,
    pub pick_rate: f64,
    pub primary_style_id: i32,
    pub sub_style_id: i32,
    pub selected_perk_ids: Vec<i32>,
    pub spell1_id: i32,
    pub spell2_id: i32,
    pub item_blocks: Vec<ItemBlock>,
    #[serde(default)]
    pub rune_pages: Vec<RunePageOption>,
    #[serde(default)]
    pub core_build_options: Vec<ItemBuildOption>,
    #[serde(default)]
    pub build_options: Vec<UnifiedBuildOption>,
    #[serde(default)]
    pub final_build_options: Vec<ItemBuildOption>,
    #[serde(default)]
    pub champion_icon_url: String,
    #[serde(default)]
    pub primary_style_name: String,
    #[serde(default)]
    pub primary_style_icon_url: String,
    #[serde(default)]
    pub sub_style_name: String,
    #[serde(default)]
    pub sub_style_icon_url: String,
    #[serde(default)]
    pub perks: Vec<RuneMetadata>,
    #[serde(default)]
    pub spells: Vec<SpellMetadata>,
    #[serde(default)]
    pub spell_choices: Vec<SpellMetadata>,
    #[serde(default)]
    pub active_position: String,
    #[serde(default)]
    pub active_tier: String,
    #[serde(default)]
    pub active_region: String,
    #[serde(default)]
    pub active_patch: String,
    #[serde(default)]
    pub available_patches: Vec<String>,
    #[serde(default)]
    pub patch_fallback_applied: bool,
    #[serde(default)]
    pub patch_protection_remaining_hours: Option<u32>,
    #[serde(default = "default_true")]
    pub supports_runes: bool,
}

/// Detailed stats for an item or item set from OP.GG.
#[derive(Debug, Clone, Default)]
pub struct ItemStatEntry {
    pub ids: Vec<i32>,
    pub play: i64,
    pub win: i64,
    pub pick_rate: f64,
}

impl ItemStatEntry {
    pub fn win_rate(&self) -> f64 {
        if self.play > 0 {
            (self.win as f64) / (self.play as f64)
        } else {
            0.0
        }
    }
}

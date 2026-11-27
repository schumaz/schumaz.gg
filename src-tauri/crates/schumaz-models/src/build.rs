use super::arena::{ArenaAugmentEntry, ArenaSynergyEntry};
pub use super::ddragon::*;
pub use super::matchups::{LaneMatchupEntry, LaneMatchupsSummary};
pub use super::skills::{ChampionAbilityMetadata, SkillOrderRecommendation};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;

fn default_true() -> bool {
    true
}

/// Metadata for a complete 2-spell summoner pair option (e.g. Flash + Ignite vs Flash + Teleport).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Type)]
pub struct SpellPairOption {
    pub spell1_id: i32,
    pub spell2_id: i32,
    #[serde(default)]
    pub play: i64,
    #[serde(default)]
    pub win: i64,
    #[serde(default)]
    pub pick_rate: f64,
    #[serde(default)]
    pub spells: Vec<SpellMetadata>,
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

/// Semantic category of a shop item block.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
#[serde(rename_all = "snake_case")]
pub enum ItemBlockKind {
    Starter,
    Prismatic,
    Core,
    Boots,
    Final,
    Situational,
    #[default]
    Other,
}

impl ItemBlockKind {
    /// Infers the block kind from legacy localized block titles (e.g. older disk caches).
    pub fn infer_from_title(title: &str) -> Self {
        if title.contains("Itens Iniciais") || title.contains("Starter Items") {
            Self::Starter
        } else if title.contains("Itens Prismáticos") || title.contains("Prismatic Items") {
            Self::Prismatic
        } else if title.contains("Primeiros 3 Itens") || title.contains("First 3 Items") {
            Self::Core
        } else if title.contains("Botas") || title.contains("Boots") {
            Self::Boots
        } else if title.contains("Build Final") || title.contains("Final Build") {
            Self::Final
        } else if title.contains("Itens Situacionais")
            || title.contains("Úteis")
            || title.contains("Uteis")
            || title.contains("Situational")
            || title.contains("Utility")
        {
            Self::Situational
        } else {
            Self::Other
        }
    }
}

/// An individual item group for the shop (e.g. Starters, Prismatic, Core 3, Final Build, Boots).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
pub struct ItemBlock {
    #[serde(default)]
    pub kind: ItemBlockKind,
    pub title: String,
    pub item_ids: Vec<i32>,
    #[serde(default)]
    pub item_icon_urls: Vec<String>,
}

/// Complete build recommendations fetched and parsed from OP.GG enriched with DataDragon visual assets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Type)]
pub struct ChampionBuild {
    pub champion_id: i32,
    pub champion_name: String,
    pub win_rate: f64,
    pub pick_rate: f64,
    /// Canonical 0-based meta tier (`0` = OP, `1` = Tier 1, `2` = Tier 2, `3` = Tier 3, `4` = Tier 4).
    #[serde(default)]
    pub tier: Option<i32>,
    pub primary_style_id: i32,
    pub sub_style_id: i32,
    pub selected_perk_ids: Vec<i32>,
    pub spell1_id: i32,
    pub spell2_id: i32,
    pub item_blocks: Vec<ItemBlock>,
    #[serde(default)]
    pub rune_pages: Vec<RunePageOption>,
    #[serde(default)]
    pub build_options: Vec<UnifiedBuildOption>,
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
    pub spell_options: Vec<SpellPairOption>,
    #[serde(default)]
    pub item_metadata: HashMap<i32, ItemMetadata>,
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
    #[serde(default = "default_true")]
    pub supports_spells: bool,
    #[serde(default)]
    pub arena_augments: Vec<ArenaAugmentEntry>,
    #[serde(default)]
    pub arena_synergies: Vec<ArenaSynergyEntry>,
    #[serde(default)]
    pub first_place_rate: Option<f64>,
    #[serde(default)]
    pub avg_placement: Option<f64>,
    #[serde(default)]
    pub skill_order: Option<SkillOrderRecommendation>,
    #[serde(default)]
    pub lane_matchups: Option<LaneMatchupsSummary>,
}

impl Default for ChampionBuild {
    fn default() -> Self {
        Self {
            champion_id: 0,
            champion_name: String::new(),
            win_rate: 0.0,
            pick_rate: 0.0,
            tier: None,
            primary_style_id: 0,
            sub_style_id: 0,
            selected_perk_ids: Vec::new(),
            spell1_id: 0,
            spell2_id: 0,
            item_blocks: Vec::new(),
            rune_pages: Vec::new(),
            build_options: Vec::new(),
            champion_icon_url: String::new(),
            primary_style_name: String::new(),
            primary_style_icon_url: String::new(),
            sub_style_name: String::new(),
            sub_style_icon_url: String::new(),
            perks: Vec::new(),
            spells: Vec::new(),
            spell_options: Vec::new(),
            item_metadata: HashMap::new(),
            active_position: String::new(),
            active_tier: String::new(),
            active_region: String::new(),
            active_patch: String::new(),
            available_patches: Vec::new(),
            patch_fallback_applied: false,
            patch_protection_remaining_hours: None,
            supports_runes: true,
            supports_spells: true,
            arena_augments: Vec::new(),
            arena_synergies: Vec::new(),
            first_place_rate: None,
            avg_placement: None,
            skill_order: None,
            lane_matchups: None,
        }
    }
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

use super::arena::ArenaAugmentMetadata;
use super::skills::ChampionAbilityMetadata;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::{HashMap, HashSet};

/// Dynamic item classifications parsed from DataDragon `item.json` for a patch.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ItemClassificationCache {
    pub boots: HashSet<i32>,
    pub components: HashSet<i32>,
    pub completed: HashSet<i32>,
    pub evolved_fallbacks: HashMap<i32, i32>,
    pub item_tags: HashMap<i32, Vec<String>>,
    /// Arena (`CHERRY`) Prismatic items (`44xxxx` with `gold.total == 2750`).
    #[serde(default)]
    pub arena_prismatics: HashSet<i32>,
    /// All items enabled on Riot Map `30` (Arena).
    #[serde(default)]
    pub map30_items: HashSet<i32>,
    /// Arena anvils (`220000..=220011`) and `Consumable`-tagged items.
    #[serde(default)]
    pub arena_consumables: HashSet<i32>,
    /// Purchasable standard shop items enabled on Summoner's Rift (Map `11`).
    #[serde(default)]
    pub map11_items: HashSet<i32>,
    /// Purchasable standard shop items enabled on Howling Abyss / ARAM (Map `12`).
    #[serde(default)]
    pub map12_items: HashSet<i32>,
    /// Dynamic starter items (lane/jungle/support quest items, ARAM exclusives, and potions).
    #[serde(default)]
    pub starters: HashSet<i32>,
    /// Dynamic standard shop consumables / potions.
    #[serde(default)]
    pub consumables: HashSet<i32>,
    /// Total gold cost (`gold.total`) indexed by item ID.
    #[serde(default)]
    pub item_gold: HashMap<i32, i32>,
}

/// Champion metadata translated from Riot's DataDragon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
pub struct ChampionMetadata {
    pub champion_id: i32,
    pub key_name: String,
    pub display_name: String,
    pub icon_url: String,
}

/// Dynamic champion combat profile parsed from DataDragon `champion.json` / `championFull.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ChampionProfile {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub attack: i32,
    #[serde(default)]
    pub defense: i32,
    #[serde(default)]
    pub magic: i32,
    #[serde(default)]
    pub partype: String,
}

/// Rune tree layout parsed from DataDragon `runesReforged.json` for dynamic fallback selection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RuneTreeCatalog {
    pub id: i32,
    pub key: String,
    #[serde(default)]
    pub slots: Vec<Vec<i32>>,
}

/// Lightweight localized item metadata extracted from DataDragon's `item.json` for rich tooltips.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
pub struct ItemMetadata {
    pub id: i32,
    pub name: String,
    #[serde(default)]
    pub gold_total: i32,
    #[serde(default)]
    pub stats: Vec<String>,
    #[serde(default)]
    pub plaintext: String,
}

/// Metadata for a rune or style from DataDragon's `runesReforged.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
pub struct RuneMetadata {
    pub id: i32,
    pub key: String,
    pub name: String,
    pub icon_url: String,
    #[serde(default)]
    pub short_desc: String,
}

/// Metadata for a summoner spell.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
pub struct SpellMetadata {
    pub id: i32,
    pub name: String,
    pub icon_url: String,
    #[serde(default)]
    pub modes: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub cooldown_secs: Option<i32>,
}

/// In-memory cache holding champion, rune, spell, item, and Arena augment metadata from DataDragon/CDragon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DataDragonCache {
    pub version: String,
    pub champions: HashMap<i32, ChampionMetadata>,
    pub runes: HashMap<i32, RuneMetadata>,
    #[serde(default)]
    pub spells: HashMap<i32, SpellMetadata>,
    #[serde(default)]
    pub items: HashMap<i32, ItemMetadata>,
    #[serde(default)]
    pub arena_augments: HashMap<i32, ArenaAugmentMetadata>,
    #[serde(default)]
    pub champion_abilities: HashMap<i32, Vec<ChampionAbilityMetadata>>,
    #[serde(default)]
    pub champion_profiles: HashMap<i32, ChampionProfile>,
    #[serde(default)]
    pub rune_trees: Vec<RuneTreeCatalog>,
    #[serde(default)]
    pub item_classifications: ItemClassificationCache,
}

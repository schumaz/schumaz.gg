//! Module 3: Data Fetching and Win Rate Provider (OP.GG + DataDragon)
//! Handles requests to DataDragon and OP.GG champion analysis (runes, builds, spells, win rates).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::OnceCell;

pub const OPGG_MCP_ENDPOINT: &str = "https://mcp-api.op.gg/mcp";
pub const OPGG_REST_API_BASE: &str = "https://lol-api-champion.op.gg/api";
pub const DDRAGON_BASE_URL: &str = "https://ddragon.leagueoflegends.com";

static DDRAGON_CACHE: OnceCell<DataDragonCache> = OnceCell::const_new();

/// Cache holding dynamically parsed item categories from Riot's latest DataDragon item.json.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemClassificationCache {
    pub boots: std::collections::HashSet<i32>,
    pub components: std::collections::HashSet<i32>,
    pub completed: std::collections::HashSet<i32>,
    pub evolved_fallbacks: std::collections::HashMap<i32, i32>,
}

static DYNAMIC_ITEM_CLASSIFICATIONS: std::sync::RwLock<Option<ItemClassificationCache>> =
    std::sync::RwLock::new(None);

pub fn set_dynamic_item_classifications(cache: ItemClassificationCache) {
    if let Ok(mut guard) = DYNAMIC_ITEM_CLASSIFICATIONS.write() {
        *guard = Some(cache);
    }
}

pub fn get_dynamic_item_classifications() -> Option<ItemClassificationCache> {
    DYNAMIC_ITEM_CLASSIFICATIONS
        .read()
        .ok()
        .and_then(|g| g.clone())
}

/// Dynamically checks whether an item is a boot using Riot DataDragon tags or fallback catalog.
pub fn is_boot_item_dynamic(id: i32) -> bool {
    if let Ok(guard) = DYNAMIC_ITEM_CLASSIFICATIONS.read()
        && let Some(cache) = guard.as_ref()
        && cache.boots.contains(&id)
    {
        return true;
    }
    is_boot_item(id)
}

/// Dynamically checks whether an item is a component using Riot DataDragon recipe tree or fallback catalog.
pub fn is_component_item_dynamic(id: i32) -> bool {
    if let Ok(guard) = DYNAMIC_ITEM_CLASSIFICATIONS.read()
        && let Some(cache) = guard.as_ref()
        && cache.components.contains(&id)
    {
        return true;
    }
    is_component_item(id)
}

/// Normalizes an evolved item (like Muramana, Seraph's Embrace, Fimbulwinter, Diadem of Songs) to its purchasable base.
pub fn normalize_tear_item_dynamic(id: i32) -> i32 {
    if let Ok(guard) = DYNAMIC_ITEM_CLASSIFICATIONS.read()
        && let Some(cache) = guard.as_ref()
        && let Some(&base_id) = cache.evolved_fallbacks.get(&id)
    {
        return base_id;
    }
    match id {
        3042 => 3004, // Muramana -> Manamune
        3040 => 3003, // Seraph's Embrace -> Archangel's Staff
        3121 => 3119, // Fimbulwinter -> Winter's Approach
        2530 => 2526, // Diadem of Songs -> Whispering Circlet
        other => other,
    }
}

pub const TEAR_COMPLETED_ITEMS: [i32; 4] = [
    2526, // Whispering Circlet (Circuleto Sussurrante)
    3003, // Archangel's Staff (Cajado do Arcanjo)
    3004, // Manamune
    3119, // Winter's Approach (Aproximação do Inverno)
];

/// Resolves a raw Tear of the Goddess (3070) into its completed purchasable legendary item
/// by inspecting the champion's candidate/depth entries or defaulting by position and damage type.
pub fn resolve_tear_to_completed_item(
    entries: &[ItemStatEntry],
    position: &str,
    damage_type: &str,
) -> i32 {
    // 1. Look for any completed tear item in the champion's item entries
    for entry in entries {
        for &id in &entry.ids {
            let norm = normalize_tear_item_dynamic(id);
            if TEAR_COMPLETED_ITEMS.contains(&norm) {
                return norm;
            }
        }
    }

    // 2. Fallback based on champion position / archetype
    let norm_pos = position.to_lowercase();
    if norm_pos == "support" || norm_pos == "supp" || norm_pos == "utility" {
        2526 // Whispering Circlet (Circuleto Sussurrante)
    } else if damage_type == "AP" {
        3003 // Archangel's Staff (Cajado do Arcanjo)
    } else if damage_type == "TANK" || norm_pos == "top" {
        3119 // Winter's Approach (Aproximação do Inverno)
    } else {
        3004 // Manamune
    }
}

/// Normalizes a slice of item IDs by replacing transformed items with purchasable bases
/// and optionally promoting Tear of the Goddess (3070) to its completed item,
/// deduplicating while preserving original order.
pub fn resolve_tear_item_ids(ids: &[i32], tear_upgrade: i32) -> Vec<i32> {
    let mut result = Vec::new();
    for &id in ids {
        let norm = if id == 3070 {
            tear_upgrade
        } else {
            normalize_tear_item_dynamic(id)
        };
        if !result.contains(&norm) {
            result.push(norm);
        }
    }
    result
}

/// Normalizes a slice of item IDs by replacing transformed items with purchasable bases
/// and deduplicating while preserving original order.
pub fn normalize_item_ids_dedup(ids: &[i32]) -> Vec<i32> {
    let mut result = Vec::new();
    for &id in ids {
        let norm = normalize_tear_item_dynamic(id);
        if !result.contains(&norm) {
            result.push(norm);
        }
    }
    result
}

/// Champion metadata translated from Riot's DataDragon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChampionMetadata {
    pub champion_id: i32,
    pub key_name: String,
    pub display_name: String,
    pub icon_url: String,
}

/// Metadata for a rune or style from DataDragon's `runesReforged.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RuneMetadata {
    pub id: i32,
    pub key: String,
    pub name: String,
    pub icon_url: String,
}

/// Metadata for a summoner spell.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SpellMetadata {
    pub id: i32,
    pub name: String,
    pub icon_url: String,
}

/// Metadata for a complete 9-perk rune page option (keystone, primary tree, secondary tree, stat mods).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
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

/// In-memory cache holding champion and rune metadata from DataDragon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataDragonCache {
    pub version: String,
    pub champions: HashMap<i32, ChampionMetadata>,
    pub runes: HashMap<i32, RuneMetadata>,
}

/// Metadata for a multi-option item build (e.g. Core 3 or Final 6).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ItemBuildOption {
    pub item_ids: Vec<i32>,
    #[serde(default)]
    pub item_icon_urls: Vec<String>,
    pub play: i64,
    pub win: i64,
    pub pick_rate: f64,
}

/// An individual item group for the shop (e.g. Starters, Core 3, Final Build, Boots).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ItemBlock {
    pub title: String,
    pub item_ids: Vec<i32>,
    #[serde(default)]
    pub item_icon_urls: Vec<String>,
}

/// Complete build recommendations fetched and parsed from OP.GG enriched with DataDragon visual assets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
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
}

/// Maps a queue ID into an OP.GG queue argument string ("aram" or "ranked").
pub fn opgg_queue_param_from_id(queue_id: i32) -> &'static str {
    match queue_id {
        450 => "aram",
        _ => "ranked",
    }
}

/// Returns sensible default summoner spells adapted to the queue type.
/// ARAM: Flash (4) + Snowball/Mark (32); Summoner's Rift: Flash (4) + Ignite (14).
pub fn default_spells_for_queue(queue_id: i32) -> (i32, i32) {
    if queue_id == 450 { (4, 32) } else { (4, 14) }
}

/// Fallback default lane when offline or when OP.GG ranking is unavailable.
pub fn champion_default_lane(_champion_name: &str) -> &'static str {
    "mid"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpggPositionInfo {
    pub name: String,
    pub play: u64,
    pub role_rate: f64,
}

pub type ChampionPositionsMap = HashMap<i32, Vec<OpggPositionInfo>>;
pub type PositionsCache = HashMap<String, ChampionPositionsMap>;

// In-memory cache for OP.GG champion rankings by (region, tier, version)
static CHAMPION_POSITIONS_CACHE: std::sync::LazyLock<std::sync::RwLock<PositionsCache>> =
    std::sync::LazyLock::new(|| std::sync::RwLock::new(HashMap::new()));

/// Normalizes an OP.GG position name ("TOP", "JUNGLE", "MID", "ADC", "SUPPORT") into standard lowercase lane.
pub fn normalize_opgg_lane_name(raw: &str) -> Option<&'static str> {
    match raw.to_uppercase().as_str() {
        "TOP" => Some("top"),
        "JUNGLE" | "JNG" => Some("jungle"),
        "MID" | "MIDDLE" => Some("mid"),
        "ADC" | "BOTTOM" | "BOT" => Some("adc"),
        "SUPPORT" | "SUPP" | "UTILITY" => Some("support"),
        _ => None,
    }
}

/// Given a list of OP.GG positions for a champion, returns the lane with the highest game count (`stats.play`).
pub fn select_most_played_lane(positions: &[OpggPositionInfo]) -> Option<&'static str> {
    positions
        .iter()
        .filter_map(|p| {
            let lane = normalize_opgg_lane_name(&p.name)?;
            Some((lane, p.play))
        })
        .max_by_key(|&(_, play)| play)
        .map(|(lane, _)| lane)
}

/// Extracts champion position statistics from the OP.GG champions/ranked JSON payload.
pub fn parse_opgg_champion_positions_ranking(
    json: &serde_json::Value,
) -> HashMap<i32, Vec<OpggPositionInfo>> {
    let mut map = HashMap::new();
    if let Some(data_arr) = json.get("data").and_then(|d| d.as_array()) {
        for champ in data_arr {
            if let Some(champ_id) = champ.get("id").and_then(|v| v.as_i64()).map(|id| id as i32) {
                let mut positions = Vec::new();
                if let Some(pos_arr) = champ.get("positions").and_then(|v| v.as_array()) {
                    for pos in pos_arr {
                        if let Some(name) = pos.get("name").and_then(|v| v.as_str()) {
                            let play = pos
                                .pointer("/stats/play")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            let role_rate = pos
                                .pointer("/stats/role_rate")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0);
                            positions.push(OpggPositionInfo {
                                name: name.to_string(),
                                play,
                                role_rate,
                            });
                        }
                    }
                }
                map.insert(champ_id, positions);
            }
        }
    }
    map
}

/// Resolves the champion's primary lane dynamically by querying OP.GG's ranking data
/// and picking the position with the highest game count (play count).
pub async fn resolve_champion_most_played_lane(
    client: &reqwest::Client,
    champion_id: i32,
    region: &str,
    tier: &str,
    version: Option<&str>,
) -> Result<String, String> {
    let cache_key = format!("{}:{}:{}", region, tier, version.unwrap_or("latest"));

    // 1. Check cache first
    if let Ok(guard) = CHAMPION_POSITIONS_CACHE.read()
        && let Some(champ_map) = guard.get(&cache_key)
        && let Some(positions) = champ_map.get(&champion_id)
        && let Some(lane) = select_most_played_lane(positions)
    {
        return Ok(lane.to_string());
    }

    // 2. Fetch from OP.GG
    let mut url = format!(
        "{}/{}/champions/ranked?tier={}",
        OPGG_REST_API_BASE, region, tier
    );
    if let Some(v) = version
        && !v.trim().is_empty()
    {
        url.push_str(&format!("&version={}", v.trim()));
    }

    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to request champions/ranked from OP.GG: {}", e))?;

    if !res.status().is_success() {
        return Err(format!(
            "OP.GG champions/ranked returned status: {}",
            res.status()
        ));
    }

    let json_body: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse OP.GG champions/ranked JSON: {}", e))?;

    let parsed_map = parse_opgg_champion_positions_ranking(&json_body);

    // 3. Find lane for requested champion
    let chosen_lane = parsed_map
        .get(&champion_id)
        .and_then(|positions| select_most_played_lane(positions))
        .map(|s| s.to_string());

    // 4. Update cache
    if let Ok(mut guard) = CHAMPION_POSITIONS_CACHE.write() {
        guard.insert(cache_key, parsed_map);
    }

    chosen_lane.ok_or_else(|| {
        format!(
            "Champion ID {} has no active positions in OP.GG ranked stats",
            champion_id
        )
    })
}

/// Parses an ISO 8601 timestamp string (e.g. "2026-11-07T22:59:29+09:00" or "2026-11-07T22:59:29Z")
/// into a Unix epoch timestamp (seconds).
pub fn parse_iso8601_to_unix(iso_str: &str) -> Option<u64> {
    if iso_str.len() < 19 {
        return None;
    }
    let year: i32 = iso_str[0..4].parse().ok()?;
    let month: u32 = iso_str[5..7].parse().ok()?;
    let day: u32 = iso_str[8..10].parse().ok()?;
    let hour: u32 = iso_str[11..13].parse().ok()?;
    let min: u32 = iso_str[14..16].parse().ok()?;
    let sec: u32 = iso_str[17..19].parse().ok()?;

    let mut offset_secs: i64 = 0;
    if iso_str.len() > 19 {
        let rest = &iso_str[19..];
        if rest.starts_with('+') || rest.starts_with('-') {
            let sign = if rest.starts_with('-') { -1 } else { 1 };
            let off_str = &rest[1..];
            let parts: Vec<&str> = off_str.split(':').collect();
            if !parts.is_empty() {
                let off_h: i64 = parts[0].parse().unwrap_or(0);
                let off_m: i64 = parts.get(1).and_then(|m| m.parse().ok()).unwrap_or(0);
                offset_secs = sign * (off_h * 3600 + off_m * 60);
            }
        }
    }

    let days_before_month = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    let is_leap = |y: i32| (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);

    let mut total_days: i64 = 0;
    for y in 1970..year {
        total_days += if is_leap(y) { 366 } else { 365 };
    }
    total_days += days_before_month[(month - 1) as usize] as i64;
    if month > 2 && is_leap(year) {
        total_days += 1;
    }
    total_days += (day - 1) as i64;

    let total_secs_utc =
        total_days * 86400 + (hour as i64) * 3600 + (min as i64) * 60 + (sec as i64) - offset_secs;
    if total_secs_utc >= 0 {
        Some(total_secs_utc as u64)
    } else {
        None
    }
}

/// Selects the effective patch respecting the 2-day maturity rule:
/// If the latest patch was released in the last 2 days (< 48h) with low sample volume,
/// automatically select previous patch to guarantee high sample volume.
pub fn select_effective_patch_with_volume(
    versions: &[String],
    latest_created_at_iso: Option<&str>,
    prev_patch_last_created_at_iso: Option<&str>,
    play_count: Option<u64>,
    now_unix: u64,
    user_override: Option<&str>,
) -> (String, bool) {
    if let Some(user_patch) = user_override
        && !user_patch.trim().is_empty()
    {
        return (user_patch.trim().to_string(), false);
    }

    if versions.is_empty() {
        return ("".to_string(), false);
    }

    let latest = &versions[0];
    if versions.len() < 2 {
        return (latest.clone(), false);
    }

    // High sample volume: if latest patch already has >= 1000 recorded games, it is mature
    if let Some(play) = play_count
        && play >= 1000
    {
        return (latest.clone(), false);
    }

    // If previous patch ended more than 48 hours ago, current patch has been active > 48h
    if let Some(prev_iso) = prev_patch_last_created_at_iso
        && let Some(prev_unix) = parse_iso8601_to_unix(prev_iso)
    {
        let age_secs = now_unix.saturating_sub(prev_unix);
        let two_days_secs = 2 * 86_400;
        if age_secs >= two_days_secs {
            return (latest.clone(), false);
        }
    }

    // If latest_created_at_iso indicates < 48h age and sample volume is low, fall back
    if let Some(created_at) = latest_created_at_iso
        && let Some(created_unix) = parse_iso8601_to_unix(created_at)
    {
        let age_secs = now_unix.saturating_sub(created_unix);
        let two_days_secs = 2 * 86_400; // 172,800s
        if age_secs < two_days_secs {
            return (versions[1].clone(), true);
        }
    }

    (latest.clone(), false)
}

pub fn select_effective_patch(
    versions: &[String],
    latest_created_at_iso: Option<&str>,
    now_unix: u64,
    user_override: Option<&str>,
) -> (String, bool) {
    select_effective_patch_with_volume(
        versions,
        latest_created_at_iso,
        None,
        None,
        now_unix,
        user_override,
    )
}

/// Known League of Legends boot item IDs (basic, upgraded, and tier-3 boots).
pub fn is_component_item(id: i32) -> bool {
    matches!(
        id,
        1001 // Boots of Speed
        | 1018 // Cloak of Agility
        | 1026 // Blasting Wand
        | 1027 // Sapphire Crystal
        | 1028 // Ruby Crystal
        | 1029 // Cloth Armor
        | 1031 // Chain Vest
        | 1033 // Null-Magic Mantle
        | 1036 // Long Sword
        | 1037 // Pickaxe
        | 1038 // B. F. Sword
        | 1042 // Dagger
        | 1043 // Recurve Bow
        | 1052 // Amplifying Tome
        | 1057 // Negatron Cloak
        | 1058 // Needlessly Large Rod
        | 2015 // Kircheis Shard
        | 2420 // Seeker's Armguard
        | 3024 // Glacial Buckler
        | 3035 // Last Whisper
        | 3044 // Phage
        | 3051 // Hearthbound Axe
        | 3057 // Sheen
        | 3067 // Kindlegem
        | 3070 // Tear of the Goddess
        | 3076 // Bramble Vest
        | 3077 // Tiamat
        | 3082 // Warden's Mail
        | 3086 // Zeal
        | 3105 // Aegis of the Legion
        | 3108 // Fiendish Codex
        | 3113 // Aether Wisp
        | 3114 // Forbidden Idol
        | 3123 // Executioner's Calling
        | 3133 // Caulfield's Warhammer
        | 3134 // Serrated Dirk
        | 3140 // Quicksilver Sash
        | 3145 // Hextech Alternator
        | 3155 // Hexdrinker
        | 3211 // Spectre's Cowl
        | 3801 // Crystalline Bracer
        | 3802 // Lost Chapter
        | 3803 // Catalyst of Aeons
        | 3916 // Oblivion Orb
    )
}

pub fn is_starter_item(id: i32) -> bool {
    matches!(
        id,
        1054 | 1055 | 1056 | 1082 | 1083 | 1101..=1103 | 2003 | 2031 | 2033 | 2051 | 2055 | 2138 | 2139 | 2140 | 3112 | 3184 | 3865..=3877
    )
}

pub fn is_boot_item(item_id: i32) -> bool {
    matches!(
        item_id,
        1001 // Boots
        | 2422 // Slightly Magical Footwear
        | 3006 // Berserker's Greaves
        | 3008 // Symbiotic Soles / Synchronized Souls
        | 3009 // Boots of Swiftness
        | 3010 // Symbiotic Soles
        | 3013 // Synchronized Souls (empowered)
        | 3020 // Sorcerer's Shoes
        | 3047 // Plated Steelcaps
        | 3111 // Mercury's Treads
        | 3117 // Boots of Mobility
        | 3158 // Ionian Boots of Lucidity
        | 3170..=3177 // Tier 3 / Wild Rift / Arena boots / Zephyr
    )
}

/// Normalizes DataDragon champion key into OP.GG's required champion name enum format.
pub fn champion_key_to_opgg_name(key: &str) -> String {
    match key {
        "MonkeyKing" => "MONKEY_KING".to_string(),
        _ => key.to_uppercase(),
    }
}

/// Constructs a safe, universal fallback build in case OP.GG API is down or times out.
pub fn build_fallback_champion_build(
    champion_id: i32,
    champion_name: &str,
    queue_id: i32,
) -> ChampionBuild {
    let (spell1_id, spell2_id) = default_spells_for_queue(queue_id);

    // Universal standard rune page: Precision (8000) + Domination (8100)
    let selected_perk_ids = vec![
        8005, // Press the Attack
        9111, // Triumph
        9104, // Legend: Alacrity
        8014, // Coup de Grace
        8139, // Taste of Blood
        8135, // Treasure Hunter
        5008, // Adaptive Force
        5008, // Adaptive Force
        5001, // Health Scaling
    ];

    let starter_items = if queue_id == 450 {
        vec![2051, 2003] // Guardian's Horn/Blade + Potion
    } else {
        vec![1055, 2003] // Doran's Blade + Health Potion
    };

    // Core 3 items must strictly exclude any boots (e.g. Infinity Edge, Rapid Firecannon, Lord Dominik's)
    let core_items = vec![3031, 3094, 3036];
    let boots_items = vec![3006, 3047, 3111];
    // Final build: core + late-game items (e.g. Bloodthirster, Guardian Angel)
    let final_items = vec![3031, 3094, 3036, 3072, 3026];
    // Useful / situational items: high pickrate items that are NOT in core nor in final build
    let situational_items = vec![3046, 3091, 3139]; // Phantom Dancer, Wit's End, Mercurial

    ChampionBuild {
        champion_id,
        champion_name: champion_name.to_string(),
        win_rate: 50.0,
        pick_rate: 5.0,
        primary_style_id: 8000,
        sub_style_id: 8100,
        selected_perk_ids,
        spell1_id,
        spell2_id,
        item_blocks: vec![
            ItemBlock {
                title: "Itens Iniciais".to_string(),
                item_ids: starter_items,
                item_icon_urls: vec![],
            },
            ItemBlock {
                title: "Primeiros 3 Itens".to_string(),
                item_ids: core_items,
                item_icon_urls: vec![],
            },
            ItemBlock {
                title: "3 Botas Mais Feitas".to_string(),
                item_ids: boots_items,
                item_icon_urls: vec![],
            },
            ItemBlock {
                title: "Build Final".to_string(),
                item_ids: final_items,
                item_icon_urls: vec![],
            },
            ItemBlock {
                title: "Itens Situacionais / Úteis".to_string(),
                item_ids: situational_items,
                item_icon_urls: vec![],
            },
        ],
        champion_icon_url: String::new(),
        primary_style_name: String::new(),
        primary_style_icon_url: String::new(),
        sub_style_name: String::new(),
        sub_style_icon_url: String::new(),
        perks: vec![],
        spells: vec![],
        ..Default::default()
    }
}

/// Returns metadata for a known stat shard ID if valid.
pub fn get_stat_shard_metadata(id: i32) -> Option<RuneMetadata> {
    let (name, icon) = match id {
        5008 => (
            "Força Adaptativa",
            "perk-images/StatMods/StatModsAdaptiveForceIcon.png",
        ),
        5005 => (
            "Velocidade de Ataque",
            "perk-images/StatMods/StatModsAttackSpeedIcon.png",
        ),
        5007 => (
            "Aceleração de Habilidade",
            "perk-images/StatMods/StatModsCDRScalingIcon.png",
        ),
        5002 => ("Armadura", "perk-images/StatMods/StatModsArmorIcon.png"),
        5003 => (
            "Resistência Mágica",
            "perk-images/StatMods/StatModsMagicResIcon.png",
        ),
        5001 | 5013 => (
            "Vida Escalonável",
            "perk-images/StatMods/StatModsHealthScalingIcon.png",
        ),
        5010 => (
            "Velocidade de Movimento",
            "perk-images/StatMods/StatModsMovementSpeedIcon.png",
        ),
        5011 => ("Vida", "perk-images/StatMods/StatModsHealthPlusIcon.png"),
        5012 => (
            "Tenacidade",
            "perk-images/StatMods/StatModsTenacityIcon.png",
        ),
        _ => return None,
    };
    Some(RuneMetadata {
        id,
        key: format!("StatShard_{}", id),
        name: name.to_string(),
        icon_url: format!("https://ddragon.leagueoflegends.com/cdn/img/{}", icon),
    })
}

/// Resolves rune or shard metadata from the DataDragon runes cache or shard catalog.
pub fn resolve_rune_metadata(runes_map: &HashMap<i32, RuneMetadata>, perk_id: i32) -> RuneMetadata {
    if let Some(meta) = runes_map.get(&perk_id) {
        return meta.clone();
    }
    if let Some(shard) = get_stat_shard_metadata(perk_id) {
        return shard;
    }
    RuneMetadata {
        id: perk_id,
        key: format!("perk_{}", perk_id),
        name: format!("Runa {}", perk_id),
        icon_url: String::new(),
    }
}

/// Resolves summoner spell metadata and official Riot CDN icon.
pub fn get_spell_metadata(version: &str, spell_id: i32) -> SpellMetadata {
    let (name, icon_name) = match spell_id {
        4 => ("Flash", "SummonerFlash"),
        14 => ("Incendiar", "SummonerDot"),
        12 => ("Teletransporte", "SummonerTeleport"),
        11 => ("Golpear", "SummonerSmite"),
        7 => ("Curar", "SummonerHeal"),
        21 => ("Barreira", "SummonerBarrier"),
        3 => ("Exaustão", "SummonerExhaust"),
        6 => ("Fantasma", "SummonerHaste"),
        1 => ("Purificar", "SummonerBoost"),
        32 => ("Marca", "SummonerSnowball"),
        _ => ("Feitiço", "SummonerFlash"),
    };
    SpellMetadata {
        id: spell_id,
        name: name.to_string(),
        icon_url: format!(
            "https://ddragon.leagueoflegends.com/cdn/{}/img/spell/{}.png",
            version, icon_name
        ),
    }
}

/// Enriches a ChampionBuild with visual assets from DataDragon (avatars, icons, spells, items).
pub fn enrich_build_with_ddragon(build: &mut ChampionBuild, cache: &DataDragonCache) {
    if let Some(champ) = cache.champions.get(&build.champion_id) {
        build.champion_icon_url = champ.icon_url.clone();
        if build.champion_name.is_empty() || build.champion_name == "Unknown" {
            build.champion_name = champ.display_name.clone();
        }
    } else if !build.champion_name.is_empty() {
        build.champion_icon_url = format!(
            "https://ddragon.leagueoflegends.com/cdn/{}/img/champion/{}.png",
            cache.version, build.champion_name
        );
    }

    if let Some(primary) = cache.runes.get(&build.primary_style_id) {
        build.primary_style_name = primary.name.clone();
        build.primary_style_icon_url = primary.icon_url.clone();
    }
    if let Some(sub) = cache.runes.get(&build.sub_style_id) {
        build.sub_style_name = sub.name.clone();
        build.sub_style_icon_url = sub.icon_url.clone();
    }

    build.perks = build
        .selected_perk_ids
        .iter()
        .map(|id| resolve_rune_metadata(&cache.runes, *id))
        .collect();

    for page in &mut build.rune_pages {
        if let Some(primary) = cache.runes.get(&page.primary_style_id) {
            page.primary_style_name = primary.name.clone();
            page.primary_style_icon_url = primary.icon_url.clone();
        }
        if let Some(sub) = cache.runes.get(&page.sub_style_id) {
            page.sub_style_name = sub.name.clone();
            page.sub_style_icon_url = sub.icon_url.clone();
        }
        page.perks = page
            .selected_perk_ids
            .iter()
            .map(|id| resolve_rune_metadata(&cache.runes, *id))
            .collect();
    }

    // If position is jungle, ensure Smite (11) is placed in the build's spells
    if build.active_position.eq_ignore_ascii_case("jungle") {
        let prefs = crate::lcu_injector::get_user_preferences();
        let (s1, s2) = crate::lcu_injector::apply_jungle_smite_and_flash_preference(
            build.spell1_id,
            build.spell2_id,
            prefs.flash_key,
            true,
        );
        build.spell1_id = s1;
        build.spell2_id = s2;
    }

    build.spells = vec![
        get_spell_metadata(&cache.version, build.spell1_id),
        get_spell_metadata(&cache.version, build.spell2_id),
    ];

    if build.spell_choices.is_empty() {
        let is_jungle = build.active_position.eq_ignore_ascii_case("jungle");
        let non_primary = if is_jungle {
            if build.spell1_id == 11 {
                build.spell2_id
            } else {
                build.spell1_id
            }
        } else {
            if build.spell1_id == 4 {
                build.spell2_id
            } else {
                build.spell1_id
            }
        };
        let second_alt = if is_jungle {
            if non_primary == 4 { 6 } else { 4 }
        } else {
            if non_primary == 21 {
                3
            } else if non_primary == 14 {
                12
            } else {
                14
            }
        };
        build.spell_choices = vec![
            get_spell_metadata(&cache.version, non_primary),
            get_spell_metadata(&cache.version, second_alt),
        ];
    } else {
        build.spell_choices = build
            .spell_choices
            .iter()
            .map(|s| get_spell_metadata(&cache.version, s.id))
            .collect();
    }

    for block in &mut build.item_blocks {
        block.item_icon_urls = block
            .item_ids
            .iter()
            .map(|id| {
                format!(
                    "https://ddragon.leagueoflegends.com/cdn/{}/img/item/{}.png",
                    cache.version, id
                )
            })
            .collect();
    }

    for opt in &mut build.core_build_options {
        opt.item_icon_urls = opt
            .item_ids
            .iter()
            .map(|id| {
                format!(
                    "https://ddragon.leagueoflegends.com/cdn/{}/img/item/{}.png",
                    cache.version, id
                )
            })
            .collect();
    }

    for opt in &mut build.final_build_options {
        opt.item_icon_urls = opt
            .item_ids
            .iter()
            .map(|id| {
                format!(
                    "https://ddragon.leagueoflegends.com/cdn/{}/img/item/{}.png",
                    cache.version, id
                )
            })
            .collect();
    }
}

/// Parses the latest DataDragon version string from `/api/versions.json`.
pub fn parse_latest_ddragon_version(versions_json: &serde_json::Value) -> Result<String, String> {
    versions_json
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Failed to extract latest version from versions array".to_string())
}

/// Parses all champions from `champion.json` into an O(1) lookup map.
pub fn parse_all_ddragon_champions(
    ddragon_json: &serde_json::Value,
    version: &str,
) -> Result<HashMap<i32, ChampionMetadata>, String> {
    let data_map = ddragon_json
        .get("data")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "Missing 'data' object in DataDragon payload".to_string())?;

    let mut map = HashMap::with_capacity(data_map.len());

    for (_name_key, champ_data) in data_map {
        let key_str = champ_data.get("key").and_then(|k| k.as_str()).unwrap_or("");
        if let Ok(champ_id) = key_str.parse::<i32>() {
            let id_str = champ_data.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let display_name = champ_data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(id_str);
            let image_full = champ_data
                .pointer("/image/full")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let icon_url = format!(
                "https://ddragon.leagueoflegends.com/cdn/{}/img/champion/{}",
                version, image_full
            );

            map.insert(
                champ_id,
                ChampionMetadata {
                    champion_id: champ_id,
                    key_name: id_str.to_string(),
                    display_name: display_name.to_string(),
                    icon_url,
                },
            );
        }
    }

    Ok(map)
}

/// Parses DataDragon `item.json` into dynamic classifications (boots, components, completed items).
pub fn parse_ddragon_item_classifications(
    items_json: &serde_json::Value,
) -> ItemClassificationCache {
    let mut boots = std::collections::HashSet::new();
    let mut components = std::collections::HashSet::new();
    let mut completed = std::collections::HashSet::new();
    let mut evolved_fallbacks = std::collections::HashMap::new();

    // Default static fallbacks for Tear of the Goddess upgrades:
    evolved_fallbacks.insert(3042, 3004); // Muramana -> Manamune
    evolved_fallbacks.insert(3040, 3003); // Seraph's Embrace -> Archangel's Staff
    evolved_fallbacks.insert(3121, 3119); // Fimbulwinter -> Winter's Approach
    evolved_fallbacks.insert(2530, 2526); // Diadem of Songs -> Whispering Circlet

    if let Some(data) = items_json.get("data").and_then(|d| d.as_object()) {
        for (id_str, item_val) in data {
            if let Ok(id) = id_str.parse::<i32>() {
                let has_into = item_val
                    .get("into")
                    .and_then(|into_arr| into_arr.as_array())
                    .map(|a| !a.is_empty())
                    .unwrap_or(false);
                let gold = item_val
                    .pointer("/gold/total")
                    .and_then(|g| g.as_i64())
                    .unwrap_or(0);
                let tags: Vec<&str> = item_val
                    .get("tags")
                    .and_then(|t| t.as_array())
                    .map(|arr| arr.iter().filter_map(|x| x.as_str()).collect())
                    .unwrap_or_default();

                let in_store = item_val
                    .get("inStore")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let special_recipe = item_val
                    .get("specialRecipe")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32);

                if !in_store && let Some(base_id) = special_recipe {
                    evolved_fallbacks.insert(id, base_id);
                }

                if tags.contains(&"Boots") {
                    boots.insert(id);
                }
                if has_into {
                    components.insert(id);
                }
                if !has_into && gold >= 1600 && !tags.contains(&"Boots") {
                    completed.insert(id);
                }
            }
        }
    }

    ItemClassificationCache {
        boots,
        components,
        completed,
        evolved_fallbacks,
    }
}

/// Parses DataDragon `item.json` to identify completed legendary/mythic items (gold >= 1600, no 'into' recipes).
pub fn parse_ddragon_completed_items(
    items_json: &serde_json::Value,
) -> std::collections::HashSet<i32> {
    let mut completed = std::collections::HashSet::new();
    if let Some(data) = items_json.get("data").and_then(|d| d.as_object()) {
        for (id_str, item_val) in data {
            if let Ok(id) = id_str.parse::<i32>() {
                let has_into = item_val
                    .get("into")
                    .and_then(|into_arr| into_arr.as_array())
                    .map(|a| !a.is_empty())
                    .unwrap_or(false);
                let gold = item_val
                    .pointer("/gold/total")
                    .and_then(|g| g.as_i64())
                    .unwrap_or(0);
                // Completed non-boot items cost >= 1600 gold and do not build into other items
                if !has_into && gold >= 1600 && !is_boot_item(id) {
                    completed.insert(id);
                }
            }
        }
    }
    completed
}

pub fn parse_ddragon_runes(
    runes_json: &serde_json::Value,
) -> Result<HashMap<i32, RuneMetadata>, String> {
    let rune_trees = runes_json
        .as_array()
        .ok_or_else(|| "Expected array for runesReforged.json".to_string())?;

    let mut map = HashMap::new();

    for tree in rune_trees {
        if let Some(tree_id) = tree.get("id").and_then(|v| v.as_i64()).map(|id| id as i32) {
            let key = tree
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let name = tree
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&key)
                .to_string();
            let icon = tree.get("icon").and_then(|v| v.as_str()).unwrap_or("");
            let icon_url = format!("https://ddragon.leagueoflegends.com/cdn/img/{}", icon);

            map.insert(
                tree_id,
                RuneMetadata {
                    id: tree_id,
                    key,
                    name,
                    icon_url,
                },
            );
        }

        // Parse individual rune slots
        if let Some(slots) = tree.get("slots").and_then(|v| v.as_array()) {
            for slot in slots {
                if let Some(runes) = slot.get("runes").and_then(|v| v.as_array()) {
                    for rune in runes {
                        if let Some(rune_id) =
                            rune.get("id").and_then(|v| v.as_i64()).map(|id| id as i32)
                        {
                            let key = rune
                                .get("key")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let name = rune
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or(&key)
                                .to_string();
                            let icon = rune.get("icon").and_then(|v| v.as_str()).unwrap_or("");
                            let icon_url =
                                format!("https://ddragon.leagueoflegends.com/cdn/img/{}", icon);

                            map.insert(
                                rune_id,
                                RuneMetadata {
                                    id: rune_id,
                                    key,
                                    name,
                                    icon_url,
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(map)
}

/// Parses DataDragon `champion.json` to find champion metadata by numeric ID.
pub fn parse_ddragon_champion_by_id(
    ddragon_json: &serde_json::Value,
    version: &str,
    target_champ_id: i32,
) -> Result<ChampionMetadata, String> {
    let champions = parse_all_ddragon_champions(ddragon_json, version)?;
    champions.get(&target_champ_id).cloned().ok_or_else(|| {
        format!(
            "Champion ID {} not found in DataDragon data",
            target_champ_id
        )
    })
}

/// Loads the DataDragonCache from a specific base URL (mockable in tests).
pub async fn load_ddragon_cache_from_base_url(
    client: &reqwest::Client,
    base_url: &str,
) -> Result<DataDragonCache, String> {
    // 1. Fetch latest version
    let versions_url = format!("{}/api/versions.json", base_url);
    let versions_res: serde_json::Value = client
        .get(&versions_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch versions.json: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse versions.json: {}", e))?;

    let version = parse_latest_ddragon_version(&versions_res)?;

    // 2. Fetch champions
    let champions_url = format!("{}/cdn/{}/data/pt_BR/champion.json", base_url, version);
    let champions_res: serde_json::Value = client
        .get(&champions_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch champion.json: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse champion.json: {}", e))?;

    let champions = parse_all_ddragon_champions(&champions_res, &version)?;

    // 3. Fetch runes
    let runes_url = format!("{}/cdn/{}/data/pt_BR/runesReforged.json", base_url, version);
    let runes_res: serde_json::Value = client
        .get(&runes_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch runesReforged.json: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse runesReforged.json: {}", e))?;

    let runes = parse_ddragon_runes(&runes_res)?;

    // 4. Dynamically fetch and index item.json for future-proof patch adaptation (fail-safe)
    let items_url = format!("{}/cdn/{}/data/pt_BR/item.json", base_url, version);
    if let Ok(res) = client.get(&items_url).send().await
        && res.status().is_success()
        && let Ok(items_res) = res.json::<serde_json::Value>().await
    {
        let classification = parse_ddragon_item_classifications(&items_res);
        set_dynamic_item_classifications(classification);
    }

    Ok(DataDragonCache {
        version,
        champions,
        runes,
    })
}

/// Gets the cached DataDragon data or initializes it if not yet loaded.
pub async fn get_or_init_ddragon_cache(
    client: &reqwest::Client,
) -> Result<&'static DataDragonCache, String> {
    DDRAGON_CACHE
        .get_or_try_init(|| async {
            load_ddragon_cache_from_base_url(client, DDRAGON_BASE_URL).await
        })
        .await
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

/// Extracts all `CoreItems(...)` or `FourthItem(...)` entries from OP.GG MCP text.
pub fn extract_all_core_items_from_text(text: &str) -> Vec<ItemStatEntry> {
    let mut entries = Vec::new();
    let mut search_slice = text;
    let prefixes = [
        "CoreItems(",
        "FourthItem(",
        "FifthItem(",
        "SixthItem(",
        "Boots(",
        "Boot(",
    ];
    loop {
        let next_match = prefixes
            .iter()
            .filter_map(|p| search_slice.find(p).map(|idx| (idx, p.len())))
            .min_by_key(|(idx, _)| *idx);
        let Some((ci_idx, prefix_len)) = next_match else {
            break;
        };
        let after_ci = &search_slice[ci_idx + prefix_len..];
        let maybe_brackets = after_ci
            .find('[')
            .zip(after_ci.find(']'))
            .filter(|(open, close)| open < close);
        if let Some((open_bracket, close_bracket)) = maybe_brackets {
            let inside = &after_ci[open_bracket + 1..close_bracket];
            let ids: Vec<i32> = inside
                .split(',')
                .filter_map(|s| s.trim().parse::<i32>().ok())
                .collect();
            if !ids.is_empty() {
                let rest_after_bracket = &after_ci[close_bracket + 1..];
                let mut play = 1;
                let mut win = 0;
                let mut pick_rate = 0.0;
                if let Some(close_paren) = rest_after_bracket.find(')') {
                    let stats_str = &rest_after_bracket[..close_paren];
                    let parts: Vec<&str> = stats_str
                        .trim_start_matches(',')
                        .split(',')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect();
                    if let Some(Ok(p)) = parts.first().map(|s| s.parse::<i64>()) {
                        play = p;
                    }
                    if let Some(Ok(w)) = parts.get(1).map(|s| s.parse::<i64>()) {
                        win = w;
                    }
                    if let Some(Ok(pr)) = parts.get(2).map(|s| s.parse::<f64>()) {
                        pick_rate = pr;
                    }
                }
                entries.push(ItemStatEntry {
                    ids,
                    play,
                    win,
                    pick_rate,
                });
            }
            search_slice = &after_ci[close_bracket + 1..];
            continue;
        }
        break;
    }
    entries
}

/// Extracts separate item lists (`last_items`, `fourth_items`, `fifth_items`, `sixth_items`) from OP.GG MCP text.
pub fn extract_mcp_grouped_item_lists(text: &str) -> Vec<(String, Vec<ItemStatEntry>)> {
    let data_fields: Vec<String> = if let Some(def_idx) = text.find("class Data:") {
        let line_rest = &text[def_idx + 11..];
        let end_line = line_rest.find('\n').unwrap_or(line_rest.len());
        line_rest[..end_line]
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| {
                matches!(
                    s.as_str(),
                    "boots" | "last_items" | "fourth_items" | "fifth_items" | "sixth_items"
                )
            })
            .collect()
    } else {
        Vec::new()
    };

    let mut groups: Vec<Vec<ItemStatEntry>> = Vec::new();
    let Some(data_idx) = text.find("Data(") else {
        return Vec::new();
    };
    let mut slice = &text[data_idx + 5..];
    if let Some(runes_idx) = slice.find("Runes(") {
        slice = &slice[..runes_idx];
    }

    let bytes = slice.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            let rem = &slice[j..];
            if rem.starts_with("CoreItems(")
                || rem.starts_with("FourthItem(")
                || rem.starts_with("FifthItem(")
                || rem.starts_with("SixthItem(")
                || rem.starts_with("Boots(")
                || rem.starts_with("Boot(")
            {
                let mut depth = 1;
                let mut k = i + 1;
                while k < bytes.len() && depth > 0 {
                    if bytes[k] == b'[' {
                        depth += 1;
                    } else if bytes[k] == b']' {
                        depth -= 1;
                    }
                    k += 1;
                }
                let list_str = &slice[i..k];
                let items = extract_all_core_items_from_text(list_str);
                groups.push(items);
                i = k;
                continue;
            }
        }
        i += 1;
    }

    let mut result = Vec::new();
    if !data_fields.is_empty() && data_fields.len() == groups.len() {
        for (name, items) in data_fields.into_iter().zip(groups) {
            result.push((name, items));
        }
    } else {
        let default_names = ["fourth_items", "fifth_items", "sixth_items", "last_items"];
        for (idx, items) in groups.into_iter().enumerate() {
            let name = default_names
                .get(idx)
                .copied()
                .unwrap_or("other")
                .to_string();
            result.push((name, items));
        }
    }
    result
}

/// Helper to parse JSON item entries (either single object, array of ids, or array of item stat objects).
fn parse_json_item_entries(val: Option<&serde_json::Value>) -> Vec<ItemStatEntry> {
    let mut entries = Vec::new();
    let Some(v) = val else {
        return entries;
    };
    if let Some(arr) = v.as_array() {
        for elem in arr {
            if let Some(num) = elem.as_i64() {
                entries.push(ItemStatEntry {
                    ids: vec![num as i32],
                    play: 1,
                    win: 0,
                    pick_rate: 0.0,
                });
            } else if let Some(obj) = elem.as_object() {
                let ids: Vec<i32> = obj
                    .get("ids")
                    .and_then(|ids_val| ids_val.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_i64().map(|n| n as i32))
                            .collect()
                    })
                    .unwrap_or_default();
                let play = obj.get("play").and_then(|p| p.as_i64()).unwrap_or(1);
                let win = obj.get("win").and_then(|w| w.as_i64()).unwrap_or(0);
                let pick_rate = obj
                    .get("pick_rate")
                    .and_then(|pr| pr.as_f64())
                    .unwrap_or(0.0);
                if !ids.is_empty() {
                    entries.push(ItemStatEntry {
                        ids,
                        play,
                        win,
                        pick_rate,
                    });
                }
            }
        }
    } else if let Some(obj) = v.as_object() {
        let ids: Vec<i32> = obj
            .get("ids")
            .and_then(|ids_val| ids_val.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_i64().map(|n| n as i32))
                    .collect()
            })
            .unwrap_or_default();
        let play = obj.get("play").and_then(|p| p.as_i64()).unwrap_or(1);
        let win = obj.get("win").and_then(|w| w.as_i64()).unwrap_or(0);
        let pick_rate = obj
            .get("pick_rate")
            .and_then(|pr| pr.as_f64())
            .unwrap_or(0.0);
        if !ids.is_empty() {
            entries.push(ItemStatEntry {
                ids,
                play,
                win,
                pick_rate,
            });
        }
    }
    entries
}

// Builds the 5 item blocks conforming to the user's requirements:
// 1. Itens Iniciais (Starter + Potion)
// 2. Primeiros 3 Itens (Top most played Core Build combination of 3 non-boot items)
// 3. 3 Botas Mais Feitas (Always strictly ordered by most absolute games played: play descending)
// 4. Build Final (6 items: 3 Core items + #1 most played 4th item + #1 most played 5th item + #1 most played 6th item)
// 5. Itens Situacionais / Úteis (Non-boot items not in core or final build, strictly ordered by play descending)

/// Extracts up to 3 Core 3 ItemBuildOptions from candidate/core entries, sorted by play count descending.
pub fn extract_top_core_build_options(
    core_entries: &[ItemStatEntry],
    all_candidates: &[i32],
    damage_type: &str,
) -> Vec<ItemBuildOption> {
    let tear_upgrade = resolve_tear_to_completed_item(core_entries, "", damage_type);
    let mut options: Vec<ItemBuildOption> = Vec::new();

    // 1. Filter core combinations with >= 2 non-boot, non-starter, non-component items
    let mut valid_combos: Vec<&ItemStatEntry> = core_entries
        .iter()
        .filter(|e| {
            let non_boot_count = e
                .ids
                .iter()
                .filter(|&&id| {
                    !is_boot_item_dynamic(id)
                        && !is_starter_item(id)
                        && !is_component_item_dynamic(id)
                })
                .count();
            non_boot_count >= 2
        })
        .collect();

    valid_combos.sort_by_key(|e| std::cmp::Reverse(e.play));

    for combo in valid_combos {
        let mut items = Vec::new();
        for &id in &combo.ids {
            let norm_id = if id == 3070 {
                tear_upgrade
            } else {
                normalize_tear_item_dynamic(id)
            };
            if items.len() < 3
                && !is_boot_item_dynamic(norm_id)
                && !is_starter_item(norm_id)
                && !is_component_item_dynamic(norm_id)
                && !items.contains(&norm_id)
            {
                items.push(norm_id);
            }
        }
        // If combo had only 2 items, fill 3rd from top candidates
        for &cand in all_candidates {
            let norm_cand = if cand == 3070 {
                tear_upgrade
            } else {
                normalize_tear_item_dynamic(cand)
            };
            if items.len() < 3 && !items.contains(&norm_cand) {
                items.push(norm_cand);
            }
        }
        if items.len() == 3 && !options.iter().any(|opt| opt.item_ids == items) {
            options.push(ItemBuildOption {
                item_ids: items,
                item_icon_urls: Vec::new(),
                play: combo.play,
                win: combo.win,
                pick_rate: combo.pick_rate,
            });
        }
    }

    // 2. If fewer than 3 options, build distinct variations from all_candidates
    let default_core = if damage_type == "AP" {
        vec![3118, 4645, 3089, 3157, 3135]
    } else {
        vec![3031, 3094, 3036, 3072, 3026]
    };

    let mut pool: Vec<i32> = Vec::new();
    for &c in all_candidates {
        let norm_c = if c == 3070 {
            tear_upgrade
        } else {
            normalize_tear_item_dynamic(c)
        };
        if !pool.contains(&norm_c) {
            pool.push(norm_c);
        }
    }
    for d in default_core {
        if !pool.contains(&d) {
            pool.push(d);
        }
    }

    while options.len() < 3 {
        let shift = options.len();
        let mut var_items = Vec::new();
        // Option 0: items 0, 1, 2
        // Option 1: items 0, 1, 3 or items 1, 0, 2
        // Option 2: items 0, 2, 3 or items 0, 1, 4
        if shift == 0 {
            var_items.extend(pool.iter().take(3).copied());
        } else if shift == 1 {
            if pool.len() >= 4 {
                var_items.push(pool[0]);
                var_items.push(pool[1]);
                var_items.push(pool[3]);
            } else {
                var_items.extend(pool.iter().take(3).copied());
            }
        } else if shift == 2 {
            if pool.len() >= 5 {
                var_items.push(pool[0]);
                var_items.push(pool[2]);
                var_items.push(pool[4]);
            } else if pool.len() >= 4 {
                var_items.push(pool[0]);
                var_items.push(pool[2]);
                var_items.push(pool[3]);
            } else {
                var_items.extend(pool.iter().take(3).copied());
            }
        }

        if var_items.len() < 3 {
            var_items.resize(3, 3089);
        }

        let is_dup = options.iter().any(|opt| opt.item_ids == var_items);
        if !is_dup {
            options.push(ItemBuildOption {
                item_ids: var_items,
                item_icon_urls: Vec::new(),
                play: 1000 / (shift as i64 + 1),
                win: 500 / (shift as i64 + 1),
                pick_rate: 10.0 / (shift as f64 + 1.0),
            });
        } else {
            break;
        }
    }

    options.sort_by_key(|opt| std::cmp::Reverse(opt.play));
    options.truncate(3);
    options
}

/// Builds up to 3 distinct 6-item FinalBuildOptions using primary core and depth/candidate items.
pub fn build_3_final_build_options(
    primary_core: &[i32],
    fourth_entries: &[ItemStatEntry],
    fifth_entries: &[ItemStatEntry],
    sixth_entries: &[ItemStatEntry],
    all_candidates: &[i32],
    damage_type: &str,
) -> Vec<ItemBuildOption> {
    let all_for_tear: Vec<ItemStatEntry> = fourth_entries
        .iter()
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
        .cloned()
        .collect();
    let tear_upgrade = resolve_tear_to_completed_item(&all_for_tear, "", damage_type);
    let mut options: Vec<ItemBuildOption> = Vec::new();

    let pick_all_from_depth_list = |entries: &[ItemStatEntry]| -> Vec<i32> {
        let mut sorted: Vec<&ItemStatEntry> = entries.iter().collect();
        sorted.sort_by_key(|e| std::cmp::Reverse(e.play));
        let mut ids = Vec::new();
        for entry in sorted {
            for &id in &entry.ids {
                let norm_id = if id == 3070 {
                    tear_upgrade
                } else {
                    normalize_tear_item_dynamic(id)
                };
                if !ids.contains(&norm_id)
                    && !is_component_item_dynamic(norm_id)
                    && !is_starter_item(norm_id)
                    && !is_boot_item_dynamic(norm_id)
                {
                    ids.push(norm_id);
                }
            }
        }
        ids
    };

    let f_ids = pick_all_from_depth_list(fourth_entries);
    let fi_ids = pick_all_from_depth_list(fifth_entries);
    let s_ids = pick_all_from_depth_list(sixth_entries);

    let default_late = if damage_type == "AP" {
        vec![3157, 3135, 4629, 3102, 4646]
    } else {
        vec![3072, 3026, 3139, 3033, 3091]
    };

    // Variation 0: Top 4th + Top 5th + Top 6th
    // Variation 1: 2nd 4th / alternative late items
    // Variation 2: 3rd 4th / alternative defensive/situational items
    for var_idx in 0..3 {
        let mut build_6 = normalize_item_ids_dedup(primary_core);

        let mut available_late = Vec::new();
        if var_idx == 0 {
            available_late.extend(&f_ids);
            available_late.extend(&fi_ids);
            available_late.extend(&s_ids);
        } else if var_idx == 1 {
            if f_ids.len() > 1 {
                available_late.extend(&f_ids[1..]);
            }
            if fi_ids.len() > 1 {
                available_late.extend(&fi_ids[1..]);
            }
            available_late.extend(&s_ids);
            available_late.extend(&f_ids);
        } else {
            if s_ids.len() > 1 {
                available_late.extend(&s_ids[1..]);
            }
            if fi_ids.len() > 1 {
                available_late.extend(&fi_ids[1..]);
            }
            available_late.extend(&f_ids);
        }
        available_late.extend(all_candidates);
        available_late.extend(&default_late);

        for &item in &available_late {
            let norm_item = if item == 3070 {
                tear_upgrade
            } else {
                normalize_tear_item_dynamic(item)
            };
            if build_6.len() < 6
                && !build_6.contains(&norm_item)
                && !is_boot_item_dynamic(norm_item)
                && !is_starter_item(norm_item)
                && !is_component_item_dynamic(norm_item)
            {
                build_6.push(norm_item);
            }
        }

        if build_6.len() == 6 && !options.iter().any(|opt| opt.item_ids == build_6) {
            options.push(ItemBuildOption {
                item_ids: build_6,
                item_icon_urls: Vec::new(),
                play: 5000 / (var_idx as i64 + 1),
                win: 2600 / (var_idx as i64 + 1),
                pick_rate: 30.0 / (var_idx as f64 + 1.0),
            });
        }
    }

    if options.is_empty() {
        let mut fallback_6 = primary_core.to_vec();
        for &d in &default_late {
            if fallback_6.len() < 6 && !fallback_6.contains(&d) {
                fallback_6.push(d);
            }
        }
        options.push(ItemBuildOption {
            item_ids: fallback_6,
            item_icon_urls: Vec::new(),
            play: 1,
            win: 1,
            pick_rate: 100.0,
        });
    }

    options.sort_by_key(|opt| std::cmp::Reverse(opt.play));
    options.truncate(3);
    options
}

pub fn build_5_item_blocks(
    starter_entry: Option<ItemStatEntry>,
    boot_entries: Vec<ItemStatEntry>,
    candidate_entries: Vec<ItemStatEntry>,
    damage_type: &str,
) -> Vec<ItemBlock> {
    build_5_item_blocks_with_depth_lists(
        starter_entry.into_iter().collect(),
        boot_entries,
        candidate_entries,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        damage_type,
        "",
    )
}

/// Builds the 5 item blocks using explicit OP.GG Core builds and separate Fourth, Fifth, and Sixth item lists.
#[allow(clippy::too_many_arguments)]
pub fn build_5_item_blocks_with_depth_lists(
    starter_entries: Vec<ItemStatEntry>,
    boot_entries: Vec<ItemStatEntry>,
    candidate_entries: Vec<ItemStatEntry>,
    fourth_entries: Vec<ItemStatEntry>,
    fifth_entries: Vec<ItemStatEntry>,
    sixth_entries: Vec<ItemStatEntry>,
    damage_type: &str,
    position: &str,
) -> Vec<ItemBlock> {
    // 1. Itens Iniciais:
    // - Default (ADC, TOP, MID): #1 most played starter item from OP.GG list + Potion (2003 / 2031)
    // - Jungle: #1 most played Jungle Pet (1101, 1102, 1103) + Potion (2003)
    // - Support: Only Potion (2003 / 2031) since support quest item is pre-granted/bought
    let norm_pos = position.to_lowercase();
    let is_support = norm_pos == "support" || norm_pos == "supp" || norm_pos == "utility";
    let is_jungle = norm_pos == "jungle" || norm_pos == "jng";

    let starter_ids: Vec<i32> = if is_support {
        let mut potion_id = 2003;
        for entry in &starter_entries {
            for &id in &entry.ids {
                if id == 2003 || id == 2031 || id == 2033 {
                    potion_id = id;
                    break;
                }
            }
            if potion_id != 2003 {
                break;
            }
        }
        vec![potion_id]
    } else if is_jungle {
        let mut pet_counts: HashMap<i32, i64> = HashMap::new();
        let mut potion_id = 2003;
        for entry in &starter_entries {
            for &id in &entry.ids {
                if matches!(id, 1101..=1103) {
                    *pet_counts.entry(id).or_insert(0) += entry.play.max(1);
                } else if id == 2003 || id == 2031 || id == 2033 {
                    potion_id = id;
                }
            }
        }
        let best_pet = pet_counts
            .into_iter()
            .max_by_key(|&(_, play)| play)
            .map(|(id, _)| id)
            .unwrap_or(1103);
        vec![best_pet, potion_id]
    } else {
        // Standard lanes (ADC, TOP, MID, or fallback):
        let mut starter_counts: HashMap<i32, (i64, usize)> = HashMap::new();
        let mut seen_idx = 0;
        let mut potion_id = 2003;
        for entry in &starter_entries {
            for &id in &entry.ids {
                if id == 2003 || id == 2031 || id == 2033 {
                    potion_id = id;
                } else if !matches!(id, 1101..=1103) && !matches!(id, 3865..=3877) {
                    let slot = starter_counts.entry(id).or_insert_with(|| {
                        let i = seen_idx;
                        seen_idx += 1;
                        (0, i)
                    });
                    slot.0 += entry.play.max(1);
                }
            }
        }
        let mut sorted_starters: Vec<(i32, i64, usize)> = starter_counts
            .into_iter()
            .map(|(id, (p, idx))| (id, p, idx))
            .collect();
        sorted_starters.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.2.cmp(&b.2)));
        let best_starter = sorted_starters
            .first()
            .map(|(id, _, _)| *id)
            .unwrap_or_else(|| if damage_type == "AP" { 1056 } else { 1055 });
        if best_starter == 2033 {
            vec![2033]
        } else {
            vec![best_starter, potion_id]
        }
    };

    // Future-proof contextual sets directly from OP.GG structures:
    let known_boots: std::collections::HashSet<i32> = boot_entries
        .iter()
        .flat_map(|e| e.ids.iter().copied())
        .collect();

    let known_starters: std::collections::HashSet<i32> = starter_entries
        .iter()
        .flat_map(|e| e.ids.iter().copied())
        .collect();

    let is_boot = |id: i32| -> bool { known_boots.contains(&id) || is_boot_item_dynamic(id) };
    let is_starter = |id: i32| -> bool {
        known_starters.contains(&id) || starter_ids.contains(&id) || is_starter_item(id)
    };
    let is_comp = |id: i32| -> bool { is_component_item_dynamic(id) };

    // 2. 3 Botas Mais Feitas: Always ranked by absolute games played (play count descending)
    let mut boot_map: HashMap<i32, (i64, usize)> = HashMap::new();
    let mut boot_seen_counter = 0;
    for entry in &boot_entries {
        for &id in &entry.ids {
            if is_boot(id) || (!is_starter(id) && !is_comp(id)) {
                let slot = boot_map.entry(id).or_insert_with(|| {
                    let idx = boot_seen_counter;
                    boot_seen_counter += 1;
                    (0, idx)
                });
                slot.0 += entry.play.max(1);
            }
        }
    }
    // Only supplement with candidate_entries if boot_entries provided fewer than 3 boots
    if boot_map.len() < 3 {
        for entry in &candidate_entries {
            for &id in &entry.ids {
                if is_boot(id) {
                    let slot = boot_map.entry(id).or_insert_with(|| {
                        let idx = boot_seen_counter;
                        boot_seen_counter += 1;
                        (0, idx)
                    });
                    slot.0 += entry.play.max(1);
                }
            }
        }
    }
    let mut boot_pairs: Vec<(i32, i64, usize)> = boot_map
        .into_iter()
        .map(|(id, (play, idx))| (id, play, idx))
        .collect();
    boot_pairs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.2.cmp(&b.2)));
    let mut boots_list: Vec<i32> = boot_pairs.into_iter().map(|(id, _, _)| id).collect();

    let archetype_boots = if damage_type == "AP" {
        vec![3020, 3158, 3111, 3047]
    } else {
        vec![3006, 3047, 3111, 3009]
    };
    for b in archetype_boots {
        if boots_list.len() < 3 && !boots_list.contains(&b) {
            boots_list.push(b);
        }
    }
    boots_list.truncate(3);

    // 3. Aggregate all non-boot, non-starter candidate items with play counts
    #[derive(Debug, Clone)]
    struct ItemAggregate {
        id: i32,
        play: i64,
        first_seen_index: usize,
    }

    let all_entries_for_tear: Vec<ItemStatEntry> = candidate_entries
        .iter()
        .chain(fourth_entries.iter())
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
        .cloned()
        .collect();
    let tear_upgrade = resolve_tear_to_completed_item(&all_entries_for_tear, position, damage_type);

    let mut item_map: HashMap<i32, (i64, usize)> = HashMap::new();
    let mut item_seen_counter = 0;
    for entry in candidate_entries
        .iter()
        .chain(fourth_entries.iter())
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
    {
        for &id in &entry.ids {
            let norm_id = if id == 3070 {
                tear_upgrade
            } else {
                normalize_tear_item_dynamic(id)
            };
            if !is_boot(norm_id) && !is_starter(norm_id) && !is_comp(norm_id) {
                let entry_play = entry.play.max(1);
                let slot = item_map.entry(norm_id).or_insert_with(|| {
                    let idx = item_seen_counter;
                    item_seen_counter += 1;
                    (0, idx)
                });
                slot.0 += entry_play;
            }
        }
    }

    let mut all_candidates: Vec<ItemAggregate> = item_map
        .into_iter()
        .map(|(id, (play, first_seen_index))| ItemAggregate {
            id,
            play,
            first_seen_index,
        })
        .collect();

    // Sort strictly by absolute games played (play count descending):
    all_candidates.sort_by(|a, b| {
        b.play
            .cmp(&a.play)
            .then_with(|| a.first_seen_index.cmp(&b.first_seen_index))
    });

    // Block 2: Primeiros 3 Itens
    // The 3 core items must strictly appear in sequential order of progression:
    // (1st most built item -> 2nd item after the 1st -> 3rd item after the 2nd).
    let mut clean_core: Vec<i32> = Vec::new();

    // Check if candidate_entries contains multi-item core combinations (length >= 2 non-boot items)
    let mut core_combos: Vec<&ItemStatEntry> = candidate_entries
        .iter()
        .filter(|e| {
            let non_boot_count = e
                .ids
                .iter()
                .filter(|&&id| {
                    !is_boot_item_dynamic(id)
                        && !is_starter_item(id)
                        && !is_component_item_dynamic(id)
                })
                .count();
            non_boot_count >= 2
        })
        .collect();

    if !core_combos.is_empty() {
        core_combos.sort_by_key(|a| std::cmp::Reverse(a.play));
        let best_combo = core_combos[0];
        for &id in &best_combo.ids {
            let norm_id = if id == 3070 {
                tear_upgrade
            } else {
                normalize_tear_item_dynamic(id)
            };
            if clean_core.len() < 3
                && !is_boot_item_dynamic(norm_id)
                && !is_starter_item(norm_id)
                && !is_component_item_dynamic(norm_id)
                && !clean_core.contains(&norm_id)
            {
                clean_core.push(norm_id);
            }
        }
    }

    // Fill up to 3 core items from the top sorted candidates if needed
    for cand in &all_candidates {
        let norm_id = if cand.id == 3070 {
            tear_upgrade
        } else {
            normalize_tear_item_dynamic(cand.id)
        };
        if clean_core.len() < 3 && !clean_core.contains(&norm_id) {
            clean_core.push(norm_id);
        }
    }

    let default_core = if damage_type == "AP" {
        vec![3118, 4645, 3089]
    } else {
        vec![3031, 3094, 3036]
    };
    for d in default_core {
        if clean_core.len() < 3 && !clean_core.contains(&d) {
            clean_core.push(d);
        }
    }

    // Block 4: Build Final (6 items: 3 Core items + most built 4th + most built 5th + most built 6th)
    // The first 3 items MUST be identical to clean_core in the EXACT SAME sequential order,
    // followed by the top valid non-duplicate item from fourth_items, fifth_items, and sixth_items.
    let mut final_build: Vec<i32> = clean_core.clone();

    let pick_top_from_depth_list =
        |entries: &[ItemStatEntry], current_build: &[i32]| -> Option<i32> {
            let mut sorted: Vec<&ItemStatEntry> = entries.iter().collect();
            sorted.sort_by_key(|e| std::cmp::Reverse(e.play));
            for entry in sorted {
                for &id in &entry.ids {
                    let norm_id = if id == 3070 {
                        tear_upgrade
                    } else {
                        normalize_tear_item_dynamic(id)
                    };
                    if !current_build.contains(&norm_id)
                        && !is_component_item_dynamic(norm_id)
                        && !is_starter_item(norm_id)
                        && !is_boot_item_dynamic(norm_id)
                    {
                        return Some(norm_id);
                    }
                }
            }
            None
        };

    for depth_list in [&fourth_entries, &fifth_entries, &sixth_entries] {
        if final_build.len() < 6
            && let Some(item_id) = pick_top_from_depth_list(depth_list, &final_build)
        {
            final_build.push(item_id);
        }
    }

    // Fallback: fill remaining slots up to 6 items from top candidates sorted by play count
    for cand in &all_candidates {
        if final_build.len() < 6
            && !final_build.contains(&cand.id)
            && !is_component_item_dynamic(cand.id)
            && !is_starter_item(cand.id)
            && !is_boot_item_dynamic(cand.id)
        {
            final_build.push(cand.id);
        }
    }
    let default_final = if damage_type == "AP" {
        vec![3157, 3135, 4629, 3102]
    } else {
        vec![3072, 3026, 3139, 3033]
    };
    for d in default_final {
        if final_build.len() < 6 && !final_build.contains(&d) {
            final_build.push(d);
        }
    }

    // Block 5: Itens Situacionais / Úteis
    // Strictly non-boot items that are NOT in clean_core and NOT in final_build,
    // sorted strictly in descending order of absolute games played (play count)!
    let mut situational_candidates: Vec<&ItemAggregate> = all_candidates
        .iter()
        .filter(|cand| {
            let norm_id = if cand.id == 3070 {
                tear_upgrade
            } else {
                normalize_tear_item_dynamic(cand.id)
            };
            !clean_core.contains(&norm_id)
                && !final_build.contains(&norm_id)
                && !is_component_item_dynamic(norm_id)
                && !is_starter_item(norm_id)
                && !is_boot_item_dynamic(norm_id)
        })
        .collect();
    situational_candidates.sort_by(|a, b| {
        b.play
            .cmp(&a.play)
            .then_with(|| a.first_seen_index.cmp(&b.first_seen_index))
    });

    let mut situational_items: Vec<i32> = Vec::new();
    for c in situational_candidates {
        let norm_id = if c.id == 3070 {
            tear_upgrade
        } else {
            normalize_tear_item_dynamic(c.id)
        };
        if !situational_items.contains(&norm_id)
            && !clean_core.contains(&norm_id)
            && !final_build.contains(&norm_id)
        {
            situational_items.push(norm_id);
        }
        if situational_items.len() == 6 {
            break;
        }
    }

    // If candidate pool had few items, supplement with safe high-utility situational items (without duplicates)
    let archetype_situational = if damage_type == "AP" {
        vec![4629, 4646, 3102, 3165, 6653]
    } else {
        vec![3026, 3139, 3072, 3046, 3091]
    };
    for item in archetype_situational {
        if situational_items.len() < 4
            && !clean_core.contains(&item)
            && !final_build.contains(&item)
            && !situational_items.contains(&item)
        {
            situational_items.push(item);
        }
    }
    situational_items.truncate(6);

    vec![
        ItemBlock {
            title: "Itens Iniciais".to_string(),
            item_ids: starter_ids,
            item_icon_urls: vec![],
        },
        ItemBlock {
            title: "Primeiros 3 Itens".to_string(),
            item_ids: clean_core,
            item_icon_urls: vec![],
        },
        ItemBlock {
            title: "3 Botas Mais Feitas".to_string(),
            item_ids: boots_list,
            item_icon_urls: vec![],
        },
        ItemBlock {
            title: "Build Final".to_string(),
            item_ids: final_build,
            item_icon_urls: vec![],
        },
        ItemBlock {
            title: "Itens Situacionais / Úteis".to_string(),
            item_ids: situational_items,
            item_icon_urls: vec![],
        },
    ]
}

/// Parses text format returned in OP.GG MCP `result.content[0].text`.
pub fn parse_opgg_mcp_text(
    text: &str,
    champion_id: i32,
    champion_name: &str,
) -> Result<ChampionBuild, String> {
    // 1. Extract win_rate and pick_rate: AverageStats(...)
    let mut win_rate = 50.0;
    let mut pick_rate = 5.0;

    let avg_fields: Vec<&str> = if let Some(def_idx) = text.find("class AverageStats:") {
        let line_rest = &text[def_idx + 19..];
        let end_line = line_rest.find('\n').unwrap_or(line_rest.len());
        line_rest[..end_line]
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    };

    if let Some(avg_idx) = text.find("AverageStats(") {
        let after_avg = &text[avg_idx + 13..];
        if let Some(close_idx) = after_avg.find(')') {
            let parts: Vec<&str> = after_avg[..close_idx].split(',').collect();
            if !avg_fields.is_empty() && avg_fields.len() == parts.len() {
                for (field_name, val_str) in avg_fields.iter().zip(parts.iter()) {
                    if let Ok(num) = val_str.trim().parse::<f64>() {
                        if *field_name == "win_rate" {
                            win_rate = if num <= 1.0 {
                                (num * 1000.0).round() / 10.0
                            } else {
                                num
                            };
                        } else if *field_name == "pick_rate" {
                            pick_rate = if num <= 1.0 {
                                (num * 1000.0).round() / 10.0
                            } else {
                                num
                            };
                        }
                    }
                }
            } else {
                // Heuristic fallback: play count is an integer > 100, win_rate and pick_rate are <= 100
                let float_candidates: Vec<f64> = parts
                    .iter()
                    .filter_map(|s| s.trim().parse::<f64>().ok())
                    .filter(|&v| v <= 100.0)
                    .collect();
                if float_candidates.len() >= 2 {
                    let w = float_candidates[0];
                    let p = float_candidates[1];
                    win_rate = if w <= 1.0 {
                        (w * 1000.0).round() / 10.0
                    } else {
                        w
                    };
                    pick_rate = if p <= 1.0 {
                        (p * 1000.0).round() / 10.0
                    } else {
                        p
                    };
                } else if float_candidates.len() == 1 {
                    let w = float_candidates[0];
                    win_rate = if w <= 1.0 {
                        (w * 1000.0).round() / 10.0
                    } else {
                        w
                    };
                }
            }
        }
    }

    // 2. Extract Runes: Runes(8100,[8112,8139,8140,8106],8200,[8210,8226],[5005,5008,5001])
    let runes_idx = text
        .find("Runes(")
        .ok_or_else(|| "Missing Runes block in OP.GG text".to_string())?;
    let after_runes = &text[runes_idx + 6..];

    // Find primary style id
    let comma1 = after_runes
        .find(',')
        .ok_or_else(|| "Invalid Runes format".to_string())?;
    let mut primary_style_id: i32 = after_runes[..comma1]
        .trim()
        .parse()
        .map_err(|e| format!("Invalid primary style id: {}", e))?;

    // Primary perks array: [ ... ]
    let p_open = after_runes
        .find('[')
        .ok_or_else(|| "Missing primary perks [".to_string())?;
    let p_close = after_runes
        .find(']')
        .ok_or_else(|| "Missing primary perks ]".to_string())?;
    let primary_perks: Vec<i32> = after_runes[p_open + 1..p_close]
        .split(',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .collect();

    let after_primary = &after_runes[p_close + 1..];
    let after_primary_trim =
        after_primary.trim_start_matches(|c: char| c == ',' || c.is_whitespace());

    // Sub style id
    let comma2 = after_primary_trim
        .find(',')
        .ok_or_else(|| "Invalid sub style format".to_string())?;
    let mut sub_style_id: i32 = after_primary_trim[..comma2]
        .trim()
        .parse()
        .map_err(|e| format!("Invalid sub style id: {}", e))?;

    // Secondary perks array: [ ... ]
    let s_open = after_primary_trim
        .find('[')
        .ok_or_else(|| "Missing secondary perks [".to_string())?;
    let s_close = after_primary_trim
        .find(']')
        .ok_or_else(|| "Missing secondary perks ]".to_string())?;
    let secondary_perks: Vec<i32> = after_primary_trim[s_open + 1..s_close]
        .split(',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .collect();

    // Stat mods array: [ ... ]
    let after_sec = &after_primary_trim[s_close + 1..];
    let m_open = after_sec
        .find('[')
        .ok_or_else(|| "Missing stat mods [".to_string())?;
    let m_close = after_sec
        .find(']')
        .ok_or_else(|| "Missing stat mods ]".to_string())?;
    let stat_mods: Vec<i32> = after_sec[m_open + 1..m_close]
        .split(',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .collect();

    let mut selected_perk_ids = Vec::with_capacity(9);
    selected_perk_ids.extend(primary_perks);
    selected_perk_ids.extend(secondary_perks);
    selected_perk_ids.extend(stat_mods);

    if selected_perk_ids.len() != 9 {
        return Err(format!(
            "Expected 9 perks, extracted {}",
            selected_perk_ids.len()
        ));
    }

    // 3. Extract all CoreItems entries with stats
    let all_entries = extract_all_core_items_from_text(text);

    // 4. Extract summoner spells
    let mut spell1_id = 4;
    let mut spell2_id = 14;
    if let Some(ss_idx) = text.find("SummonerSpells(") {
        let after_ss = &text[ss_idx + 15..];
        let maybe_brackets = after_ss
            .find('[')
            .zip(after_ss.find(']'))
            .filter(|(open, close)| open < close);
        if let Some((o, c)) = maybe_brackets {
            let ids: Vec<i32> = after_ss[o + 1..c]
                .split(',')
                .filter_map(|s| s.trim().parse::<i32>().ok())
                .collect();
            if ids.len() >= 2 {
                spell1_id = ids[0];
                spell2_id = ids[1];
            }
        }
    } else {
        for entry in &all_entries {
            if entry.ids.len() == 2
                && matches!(entry.ids[0], 4 | 14 | 12 | 7 | 21 | 11 | 6 | 3 | 1 | 32)
            {
                spell1_id = entry.ids[0];
                spell2_id = entry.ids[1];
                break;
            }
        }
    }

    // Filter out spell arrays from items entries
    let item_candidate_entries: Vec<ItemStatEntry> = all_entries
        .into_iter()
        .filter(|entry| {
            !(entry.ids.len() == 2
                && matches!(entry.ids[0], 4 | 14 | 12 | 7 | 21 | 11 | 6 | 3 | 1 | 32))
        })
        .collect();

    let is_ap = text.contains("\"AP\"")
        || (!text.contains("\"AD\"")
            && (champion_name.to_lowercase().contains("ahri")
                || champion_name.to_lowercase().contains("lux")
                || champion_name.to_lowercase().contains("syndra")
                || champion_name.to_lowercase().contains("veigar")));
    let damage_type = if is_ap { "AP" } else { "AD" };

    let starter_entries = if let Some(e) = item_candidate_entries.get(2) {
        vec![e.clone()]
    } else {
        Vec::new()
    };

    let grouped_lists = extract_mcp_grouped_item_lists(text);
    let mut fourth_entries = Vec::new();
    let mut fifth_entries = Vec::new();
    let mut sixth_entries = Vec::new();
    let mut last_entries = Vec::new();
    let mut boots_from_mcp = Vec::new();
    for (name, items) in grouped_lists {
        match name.as_str() {
            "boots" => boots_from_mcp = items,
            "fourth_items" => fourth_entries = items,
            "fifth_items" => fifth_entries = items,
            "sixth_items" => sixth_entries = items,
            "last_items" => last_entries = items,
            _ => last_entries.extend(items),
        }
    }

    let boot_entries = if !boots_from_mcp.is_empty() {
        boots_from_mcp
    } else {
        item_candidate_entries.get(1).cloned().into_iter().collect()
    };

    let mut candidate_entries = Vec::new();
    let core_ids = if let Some(first) = item_candidate_entries.first() {
        let mut core_entry = first.clone();
        if core_entry.play == 1 {
            core_entry.play = 10000;
        }
        let ids = core_entry.ids.clone();
        candidate_entries.push(core_entry);
        ids
    } else {
        Vec::new()
    };

    if !last_entries.is_empty() {
        for entry in last_entries {
            let filtered_ids: Vec<i32> = entry
                .ids
                .iter()
                .filter(|id| !core_ids.contains(id))
                .copied()
                .collect();
            if !filtered_ids.is_empty() {
                candidate_entries.push(ItemStatEntry {
                    ids: filtered_ids,
                    play: entry.play,
                    win: entry.win,
                    pick_rate: entry.pick_rate,
                });
            }
        }
    } else if item_candidate_entries.len() > 3 {
        for entry in &item_candidate_entries[3..] {
            let filtered_ids: Vec<i32> = entry
                .ids
                .iter()
                .filter(|id| !core_ids.contains(id))
                .copied()
                .collect();
            if !filtered_ids.is_empty() {
                candidate_entries.push(ItemStatEntry {
                    ids: filtered_ids,
                    play: entry.play,
                    win: entry.win,
                    pick_rate: entry.pick_rate,
                });
            }
        }
    }

    let position = if text.contains("\"position\": \"jungle\"") || text.contains("\"jungle\"") {
        "jungle"
    } else if text.contains("\"position\": \"support\"") || text.contains("\"support\"") {
        "support"
    } else {
        ""
    };

    let all_for_tear: Vec<ItemStatEntry> = candidate_entries
        .iter()
        .chain(fourth_entries.iter())
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
        .cloned()
        .collect();
    let tear_upgrade = resolve_tear_to_completed_item(&all_for_tear, position, damage_type);

    let normalize_entries = |entries: &mut [ItemStatEntry]| {
        for entry in entries.iter_mut() {
            entry.ids = resolve_tear_item_ids(&entry.ids, tear_upgrade);
        }
    };
    normalize_entries(&mut candidate_entries);
    normalize_entries(&mut fourth_entries);
    normalize_entries(&mut fifth_entries);
    normalize_entries(&mut sixth_entries);

    let cand_ids: Vec<i32> = candidate_entries
        .iter()
        .flat_map(|e| e.ids.clone())
        .collect();
    let core_build_options =
        extract_top_core_build_options(&candidate_entries, &cand_ids, damage_type);
    let primary_core = if let Some(first_core) = core_build_options.first() {
        first_core.item_ids.clone()
    } else {
        vec![]
    };
    let final_build_options = build_3_final_build_options(
        &primary_core,
        &fourth_entries,
        &fifth_entries,
        &sixth_entries,
        &cand_ids,
        damage_type,
    );

    let item_blocks = build_5_item_blocks_with_depth_lists(
        starter_entries,
        boot_entries,
        candidate_entries,
        fourth_entries,
        fifth_entries,
        sixth_entries,
        damage_type,
        position,
    );

    let spell_choices = vec![SpellMetadata {
        id: spell2_id,
        name: String::new(),
        icon_url: String::new(),
    }];

    let mut top_rune_pages = extract_all_mcp_runes(text);
    if top_rune_pages.is_empty() {
        top_rune_pages.push(RunePageOption {
            primary_style_id,
            sub_style_id,
            selected_perk_ids: selected_perk_ids.clone(),
            play: 1,
            win: 1,
            pick_rate: 1.0,
            ..Default::default()
        });
    } else {
        let first = &top_rune_pages[0];
        primary_style_id = first.primary_style_id;
        sub_style_id = first.sub_style_id;
        selected_perk_ids = first.selected_perk_ids.clone();
    }

    Ok(ChampionBuild {
        champion_id,
        champion_name: champion_name.to_string(),
        win_rate,
        pick_rate,
        primary_style_id,
        sub_style_id,
        selected_perk_ids,
        spell1_id,
        spell2_id,
        item_blocks,
        spell_choices,
        rune_pages: top_rune_pages,
        core_build_options,
        final_build_options,
        ..Default::default()
    })
}

/// Extracts the top summoner spells and the 2 most popular choices for the secondary spell slot
/// (Option 1 = most used, Option 2 = second most used).
/// For Jungle, Smite (11) is guaranteed and spell_choices provides the top 2 companion spells (e.g. Flash vs Ghost, or Ignite vs Flash).
/// For Laners, Flash (4) is anchor and spell_choices provides the top 2 secondary spells (e.g. Barrier vs Exhaust).
pub fn extract_summoner_spell_choices(
    analysis: &serde_json::Value,
    is_jungle: bool,
) -> ((i32, i32), Vec<i32>) {
    let mut combo_entries: Vec<(i32, i32, i64)> = Vec::new();

    if let Some(arr) = analysis.get("summoner_spells").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(ids) = item.get("ids").and_then(|i| i.as_array())
                && ids.len() >= 2
            {
                let s1 = ids[0].as_i64().unwrap_or(0) as i32;
                let s2 = ids[1].as_i64().unwrap_or(0) as i32;
                let play = item.get("play").and_then(|p| p.as_i64()).unwrap_or(1);
                if s1 > 0 && s2 > 0 {
                    combo_entries.push((s1, s2, play));
                }
            } else if let Some(ids) = item.as_array()
                && ids.len() >= 2
            {
                let s1 = ids[0].as_i64().unwrap_or(0) as i32;
                let s2 = ids[1].as_i64().unwrap_or(0) as i32;
                if s1 > 0 && s2 > 0 {
                    combo_entries.push((s1, s2, 1));
                }
            }
        }
    }

    combo_entries.sort_by_key(|e| std::cmp::Reverse(e.2));

    let top_combo = if !combo_entries.is_empty() {
        (combo_entries[0].0, combo_entries[0].1)
    } else {
        extract_most_used_summoner_spells(analysis)
    };

    let mut secondary_choices: Vec<i32> = Vec::new();

    if is_jungle {
        // Smite (11) is mandatory; collect top 2 unique non-Smite companion spells ordered by play count
        let mut companion_counts: HashMap<i32, i64> = HashMap::new();
        for (s1, s2, play) in &combo_entries {
            let companion = if *s1 == 11 {
                Some(*s2)
            } else if *s2 == 11 {
                Some(*s1)
            } else {
                None
            };
            if let Some(comp) = companion
                && comp > 0
                && comp != 11
            {
                *companion_counts.entry(comp).or_insert(0) += *play;
            }
        }
        let mut sorted_companions: Vec<(i32, i64)> = companion_counts.into_iter().collect();
        sorted_companions.sort_by_key(|e| std::cmp::Reverse(e.1));
        for (comp, _) in sorted_companions {
            if !secondary_choices.contains(&comp) {
                secondary_choices.push(comp);
            }
        }
        if secondary_choices.is_empty() {
            let other = if top_combo.0 == 11 {
                top_combo.1
            } else {
                top_combo.0
            };
            if other != 11 && other > 0 {
                secondary_choices.push(other);
            }
        }
        for fallback in [4, 6, 14] {
            if secondary_choices.len() < 2 && !secondary_choices.contains(&fallback) {
                secondary_choices.push(fallback);
            }
        }
    } else {
        // Standard laners: find primary anchor spell (Flash 4 if present in top combo, else top_combo.0)
        let primary_spell = if top_combo.0 == 4 || top_combo.1 == 4 {
            4
        } else {
            top_combo.0
        };

        // Determine secondary spell from top combo
        let top_secondary = if top_combo.0 == primary_spell {
            top_combo.1
        } else {
            top_combo.0
        };
        secondary_choices.push(top_secondary);

        // Find alternative secondary spell from remaining combos
        for (s1, s2, _) in &combo_entries {
            let alt = if *s1 == primary_spell {
                *s2
            } else if *s2 == primary_spell {
                *s1
            } else {
                continue;
            };
            if alt > 0 && !secondary_choices.contains(&alt) {
                secondary_choices.push(alt);
                break;
            }
        }

        // Fallbacks if only 1 combo was played in data
        let default_alts = [21, 3, 14, 12, 7, 6, 1]; // Barrier, Exhaust, Ignite, Teleport, Heal, Ghost, Cleanse
        for fallback in default_alts {
            if secondary_choices.len() < 2
                && !secondary_choices.contains(&fallback)
                && fallback != primary_spell
            {
                secondary_choices.push(fallback);
            }
        }
    }

    secondary_choices.truncate(2);
    (top_combo, secondary_choices)
}

/// Parses a single rune page entry from an OP.GG JSON Value (REST format).
pub fn parse_single_rune_entry(entry: &serde_json::Value) -> Option<RunePageOption> {
    let primary_style_id = entry
        .get("primary_style_id")
        .or_else(|| entry.get("primary_page_id"))
        .or_else(|| entry.pointer("/primary_style_id"))
        .or_else(|| entry.pointer("/primary_page_id"))
        .and_then(|v| v.as_i64())? as i32;

    let sub_style_id = entry
        .get("sub_style_id")
        .or_else(|| entry.get("secondary_page_id"))
        .or_else(|| entry.get("sub_page_id"))
        .or_else(|| entry.pointer("/sub_style_id"))
        .or_else(|| entry.pointer("/secondary_page_id"))
        .and_then(|v| v.as_i64())? as i32;

    if primary_style_id <= 0 || sub_style_id <= 0 || primary_style_id == sub_style_id {
        return None;
    }

    let mut selected_perk_ids = Vec::with_capacity(9);

    if let Some(primary_perks) = entry
        .get("primary_perk_ids")
        .or_else(|| entry.get("primary_rune_ids"))
        .or_else(|| entry.pointer("/primary_perk_ids"))
        .or_else(|| entry.pointer("/primary_rune_ids"))
        .and_then(|v| v.as_array())
    {
        for p in primary_perks {
            if let Some(id) = p.as_i64() {
                selected_perk_ids.push(id as i32);
            }
        }
    }

    if let Some(sub_perks) = entry
        .get("sub_perk_ids")
        .or_else(|| entry.get("secondary_rune_ids"))
        .or_else(|| entry.pointer("/sub_perk_ids"))
        .or_else(|| entry.pointer("/secondary_rune_ids"))
        .and_then(|v| v.as_array())
    {
        for p in sub_perks {
            if let Some(id) = p.as_i64() {
                selected_perk_ids.push(id as i32);
            }
        }
    }

    if let Some(stat_mods) = entry
        .get("stat_shards")
        .or_else(|| entry.get("stat_mod_ids"))
        .or_else(|| entry.pointer("/stat_shards"))
        .or_else(|| entry.pointer("/stat_mod_ids"))
        .and_then(|v| v.as_array())
    {
        for s in stat_mods {
            if let Some(id) = s.as_i64() {
                selected_perk_ids.push(id as i32);
            }
        }
    }

    if selected_perk_ids.len() != 9 {
        return None;
    }

    let play = entry.get("play").and_then(|v| v.as_i64()).unwrap_or(0);
    let win = entry.get("win").and_then(|v| v.as_i64()).unwrap_or(0);
    let pick_rate = entry
        .get("pick_rate")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    Some(RunePageOption {
        primary_style_id,
        sub_style_id,
        selected_perk_ids,
        play,
        win,
        pick_rate,
        ..Default::default()
    })
}

/// Extracts up to 3 valid rune pages from OP.GG analysis JSON, sorted by most played (`play` descending).
pub fn extract_top_rune_pages(analysis: &serde_json::Value) -> Vec<RunePageOption> {
    let mut pages: Vec<RunePageOption> = Vec::new();

    let runes_val = analysis
        .get("runes")
        .or_else(|| analysis.get("rune_pages"))
        .or_else(|| analysis.pointer("/data/runes"))
        .or_else(|| analysis.pointer("/data/rune_pages"));

    if let Some(val) = runes_val {
        if let Some(arr) = val.as_array() {
            for entry in arr {
                if let Some(page) = parse_single_rune_entry(entry) {
                    pages.push(page);
                }
            }
        } else if let Some(obj) = val.as_object() {
            let mut has_numeric_keys = false;
            for (k, v) in obj {
                if k.parse::<usize>().is_ok() {
                    has_numeric_keys = true;
                    if let Some(page) = parse_single_rune_entry(v) {
                        pages.push(page);
                    }
                }
            }
            if !has_numeric_keys && let Some(page) = parse_single_rune_entry(val) {
                pages.push(page);
            }
        }
    }

    // Deduplicate identical setups
    let mut unique_pages: Vec<RunePageOption> = Vec::new();
    for p in pages {
        if !unique_pages.iter().any(|existing| {
            existing.primary_style_id == p.primary_style_id
                && existing.sub_style_id == p.sub_style_id
                && existing.selected_perk_ids == p.selected_perk_ids
        }) {
            unique_pages.push(p);
        }
    }

    // Sort by play count descending
    unique_pages.sort_by_key(|p| std::cmp::Reverse(p.play));
    unique_pages.truncate(3);

    unique_pages
}

/// Extracts all Runes(...) blocks from MCP text output into RunePageOption instances.
pub fn extract_all_mcp_runes(text: &str) -> Vec<RunePageOption> {
    let mut results = Vec::new();
    let mut cursor = 0;

    while let Some(rel_idx) = text[cursor..].find("Runes(") {
        let runes_idx = cursor + rel_idx;
        let after_runes = &text[runes_idx + 6..];

        if let Some(comma1) = after_runes.find(',')
            && let Ok(primary_style_id) = after_runes[..comma1].trim().parse::<i32>()
            && let (Some(p_open), Some(p_close)) = (after_runes.find('['), after_runes.find(']'))
            && p_open < p_close
            && p_open > comma1
        {
            let primary_perks: Vec<i32> = after_runes[p_open + 1..p_close]
                .split(',')
                .filter_map(|s| s.trim().parse::<i32>().ok())
                .collect();

            let after_primary = &after_runes[p_close + 1..];
            let after_primary_trim =
                after_primary.trim_start_matches(|c: char| c == ',' || c.is_whitespace());

            if let Some(comma2) = after_primary_trim.find(',')
                && let Ok(sub_style_id) = after_primary_trim[..comma2].trim().parse::<i32>()
                && let (Some(s_open), Some(s_close)) =
                    (after_primary_trim.find('['), after_primary_trim.find(']'))
                && s_open < s_close
                && s_open > comma2
            {
                let secondary_perks: Vec<i32> = after_primary_trim[s_open + 1..s_close]
                    .split(',')
                    .filter_map(|s| s.trim().parse::<i32>().ok())
                    .collect();

                let after_sec = &after_primary_trim[s_close + 1..];
                if let (Some(m_open), Some(m_close)) = (after_sec.find('['), after_sec.find(']'))
                    && m_open < m_close
                {
                    let stat_mods: Vec<i32> = after_sec[m_open + 1..m_close]
                        .split(',')
                        .filter_map(|s| s.trim().parse::<i32>().ok())
                        .collect();

                    let mut selected_perk_ids = Vec::with_capacity(9);
                    selected_perk_ids.extend(primary_perks);
                    selected_perk_ids.extend(secondary_perks);
                    selected_perk_ids.extend(stat_mods);

                    if selected_perk_ids.len() == 9 && primary_style_id != sub_style_id {
                        let is_dup = results.iter().any(|existing: &RunePageOption| {
                            existing.primary_style_id == primary_style_id
                                && existing.sub_style_id == sub_style_id
                                && existing.selected_perk_ids == selected_perk_ids
                        });
                        if !is_dup {
                            results.push(RunePageOption {
                                primary_style_id,
                                sub_style_id,
                                selected_perk_ids,
                                play: 1,
                                win: 1,
                                pick_rate: 1.0,
                                ..Default::default()
                            });
                        }
                    }
                }
            }
        }
        cursor = runes_idx + 6;
    }

    results.truncate(3);
    results
}

/// Helper to extract the most played summoner spells combination from OP.GG analysis JSON.
/// OP.GG REST returns `summoner_spells: [ { "ids": [4, 21], "play": 62087, "win": ... }, ... ]`
/// Falls back to legacy/MCP pointers like `/spells` or `/summoner_spells/ids`.
pub fn extract_most_used_summoner_spells(analysis: &serde_json::Value) -> (i32, i32) {
    if let Some(arr) = analysis.get("summoner_spells").and_then(|v| v.as_array()) {
        let mut entries: Vec<(i32, i32, i64, f64)> = Vec::new();
        for item in arr {
            if let Some(ids) = item.get("ids").and_then(|i| i.as_array())
                && ids.len() >= 2
            {
                let s1 = ids[0].as_i64().unwrap_or(0) as i32;
                let s2 = ids[1].as_i64().unwrap_or(0) as i32;
                let play = item.get("play").and_then(|p| p.as_i64()).unwrap_or(1);
                let win = item.get("win").and_then(|w| w.as_i64()).unwrap_or(0);
                let win_rate = if play > 0 {
                    win as f64 / play as f64
                } else {
                    0.0
                };
                if s1 > 0 && s2 > 0 {
                    entries.push((s1, s2, play, win_rate));
                }
            } else if let Some(ids) = item.as_array()
                && ids.len() >= 2
            {
                let s1 = ids[0].as_i64().unwrap_or(0) as i32;
                let s2 = ids[1].as_i64().unwrap_or(0) as i32;
                if s1 > 0 && s2 > 0 {
                    return (s1, s2);
                }
            }
        }

        if !entries.is_empty() {
            // Sort by most played combination
            entries.sort_by_key(|e| std::cmp::Reverse(e.2));
            return (entries[0].0, entries[0].1);
        }
    }

    // Direct flat array: "spells": [4, 14] or "summoner_spells": [4, 14]
    if let Some(arr) = analysis.get("spells").and_then(|v| v.as_array())
        && arr.len() >= 2
    {
        let s1 = arr[0].as_i64().unwrap_or(4) as i32;
        let s2 = arr[1].as_i64().unwrap_or(14) as i32;
        return (s1, s2);
    }

    if let Some(arr) = analysis
        .pointer("/summoner_spells/ids")
        .and_then(|v| v.as_array())
        && arr.len() >= 2
    {
        let s1 = arr[0].as_i64().unwrap_or(4) as i32;
        let s2 = arr[1].as_i64().unwrap_or(14) as i32;
        return (s1, s2);
    }

    // Single object or pointer
    let s1 = analysis
        .pointer("/summoner_spells/0/ids/0")
        .or_else(|| analysis.pointer("/spells/0"))
        .and_then(|v| v.as_i64())
        .unwrap_or(4) as i32;

    let s2 = analysis
        .pointer("/summoner_spells/0/ids/1")
        .or_else(|| analysis.pointer("/spells/1"))
        .and_then(|v| v.as_i64())
        .unwrap_or(14) as i32;

    (s1, s2)
}

/// Pure parser function to transform an OP.GG champion analysis response into `ChampionBuild`.
pub fn parse_opgg_analysis_payload(
    opgg_json: &serde_json::Value,
    champion_id: i32,
    champion_name: &str,
) -> Result<ChampionBuild, String> {
    // If the response is wrapped in MCP content text (e.g. result.content[0].text), use text parser!
    if let Some(text) = opgg_json
        .pointer("/result/content/0/text")
        .and_then(|v| v.as_str())
    {
        return parse_opgg_mcp_text(text, champion_id, champion_name);
    }

    let analysis = opgg_json
        .get("data")
        .or_else(|| opgg_json.pointer("/result/data"))
        .unwrap_or(opgg_json);

    let raw_win_rate = analysis
        .get("win_rate")
        .or_else(|| analysis.pointer("/stats/win_rate"))
        .or_else(|| analysis.pointer("/summary/average_stats/win_rate"))
        .or_else(|| analysis.pointer("/average_stats/win_rate"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let win_rate = if raw_win_rate > 0.0 && raw_win_rate <= 1.0 {
        (raw_win_rate * 1000.0).round() / 10.0
    } else {
        raw_win_rate
    };

    let raw_pick_rate = analysis
        .get("pick_rate")
        .or_else(|| analysis.pointer("/stats/pick_rate"))
        .or_else(|| analysis.pointer("/summary/average_stats/pick_rate"))
        .or_else(|| analysis.pointer("/average_stats/pick_rate"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let pick_rate = if raw_pick_rate > 0.0 && raw_pick_rate <= 1.0 {
        (raw_pick_rate * 1000.0).round() / 10.0
    } else {
        raw_pick_rate
    };

    // Extract rune styles and perks
    let mut primary_style_id = analysis
        .pointer("/runes/primary_style_id")
        .or_else(|| analysis.pointer("/runes/primary_page_id"))
        .or_else(|| analysis.pointer("/runes/0/primary_page_id"))
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "Missing primary_style_id in OP.GG runes".to_string())?
        as i32;

    let mut sub_style_id = analysis
        .pointer("/runes/sub_style_id")
        .or_else(|| analysis.pointer("/runes/secondary_page_id"))
        .or_else(|| analysis.pointer("/runes/0/secondary_page_id"))
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "Missing sub_style_id in OP.GG runes".to_string())?
        as i32;

    let mut selected_perk_ids = Vec::with_capacity(9);

    if let Some(primary_perks) = analysis
        .pointer("/runes/primary_perk_ids")
        .or_else(|| analysis.pointer("/runes/0/primary_rune_ids"))
        .and_then(|v| v.as_array())
    {
        for perk in primary_perks {
            if let Some(id) = perk.as_i64() {
                selected_perk_ids.push(id as i32);
            }
        }
    }

    if let Some(sub_perks) = analysis
        .pointer("/runes/sub_perk_ids")
        .or_else(|| analysis.pointer("/runes/0/secondary_rune_ids"))
        .and_then(|v| v.as_array())
    {
        for perk in sub_perks {
            if let Some(id) = perk.as_i64() {
                selected_perk_ids.push(id as i32);
            }
        }
    }

    if let Some(stat_shards) = analysis
        .pointer("/runes/stat_shards")
        .or_else(|| analysis.pointer("/runes/0/stat_mod_ids"))
        .and_then(|v| v.as_array())
    {
        for shard in stat_shards {
            if let Some(id) = shard.as_i64() {
                selected_perk_ids.push(id as i32);
            }
        }
    }

    if selected_perk_ids.len() != 9 {
        return Err(format!(
            "Invalid perk count in OP.GG payload: expected 9 perks, found {}",
            selected_perk_ids.len()
        ));
    }

    let is_jungle = analysis
        .get("position")
        .or_else(|| analysis.pointer("/meta/position"))
        .or_else(|| analysis.pointer("/summary/position"))
        .and_then(|v| v.as_str())
        .map(|p| p.eq_ignore_ascii_case("jungle"))
        .unwrap_or(false);

    // Extract summoner spells: selects the most played combination and top 2 choices for secondary spell slot
    let ((spell1_id, spell2_id), spell_choice_ids) =
        extract_summoner_spell_choices(analysis, is_jungle);

    let damage_type = analysis
        .get("damage_type")
        .and_then(|v| v.as_str())
        .unwrap_or("AP");

    // Extract item entries from JSON
    let starter_entries = parse_json_item_entries(
        analysis
            .pointer("/items/starters")
            .or_else(|| analysis.pointer("/starter_items")),
    );

    let boot_entries = parse_json_item_entries(
        analysis
            .pointer("/items/boots")
            .or_else(|| analysis.pointer("/boots")),
    );

    let mut candidate_entries = Vec::new();
    let mut core_entries = parse_json_item_entries(
        analysis
            .pointer("/items/core")
            .or_else(|| analysis.pointer("/core_items")),
    );
    // Select the #1 most played Core Build combination to identify core_ids
    core_entries.sort_by_key(|e| std::cmp::Reverse(e.play));
    let mut core_ids = Vec::new();
    for (idx, entry) in core_entries.iter_mut().enumerate() {
        if entry.play == 1 {
            entry.play = 10000 - (idx as i64);
        }
        if idx == 0 {
            core_ids.extend_from_slice(&entry.ids);
        }
    }
    candidate_entries.extend(core_entries.clone());

    let mut fourth_entries = parse_json_item_entries(
        analysis
            .pointer("/fourth_items")
            .or_else(|| analysis.pointer("/items/fourth_items"))
            .or_else(|| analysis.pointer("/items/final_build")),
    );
    let mut fifth_entries = parse_json_item_entries(
        analysis
            .pointer("/fifth_items")
            .or_else(|| analysis.pointer("/items/fifth_items")),
    );
    let mut sixth_entries = parse_json_item_entries(
        analysis
            .pointer("/sixth_items")
            .or_else(|| analysis.pointer("/items/sixth_items")),
    );

    let mut other_entries = Vec::new();
    other_entries.extend(parse_json_item_entries(
        analysis
            .pointer("/items/situational")
            .or_else(|| analysis.pointer("/last_items")),
    ));
    other_entries.extend(fourth_entries.clone());
    other_entries.extend(fifth_entries.clone());
    other_entries.extend(sixth_entries.clone());

    for entry in other_entries {
        let filtered_ids: Vec<i32> = entry
            .ids
            .iter()
            .filter(|id| !core_ids.contains(id))
            .copied()
            .collect();
        if !filtered_ids.is_empty() {
            candidate_entries.push(ItemStatEntry {
                ids: filtered_ids,
                play: entry.play,
                win: entry.win,
                pick_rate: entry.pick_rate,
            });
        }
    }

    let position = analysis
        .get("position")
        .or_else(|| analysis.pointer("/meta/position"))
        .or_else(|| analysis.pointer("/summary/position"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let all_for_tear: Vec<ItemStatEntry> = core_entries
        .iter()
        .chain(candidate_entries.iter())
        .chain(fourth_entries.iter())
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
        .cloned()
        .collect();
    let tear_upgrade = resolve_tear_to_completed_item(&all_for_tear, position, damage_type);

    let normalize_entries = |entries: &mut [ItemStatEntry]| {
        for entry in entries.iter_mut() {
            entry.ids = resolve_tear_item_ids(&entry.ids, tear_upgrade);
        }
    };
    normalize_entries(&mut core_entries);
    normalize_entries(&mut candidate_entries);
    normalize_entries(&mut fourth_entries);
    normalize_entries(&mut fifth_entries);
    normalize_entries(&mut sixth_entries);

    let cand_ids: Vec<i32> = candidate_entries
        .iter()
        .flat_map(|e| e.ids.clone())
        .collect();
    let core_build_options = extract_top_core_build_options(&core_entries, &cand_ids, damage_type);
    let primary_core = if let Some(first_core) = core_build_options.first() {
        first_core.item_ids.clone()
    } else {
        vec![]
    };
    let final_build_options = build_3_final_build_options(
        &primary_core,
        &fourth_entries,
        &fifth_entries,
        &sixth_entries,
        &cand_ids,
        damage_type,
    );

    let item_blocks = build_5_item_blocks_with_depth_lists(
        starter_entries,
        boot_entries,
        candidate_entries,
        fourth_entries,
        fifth_entries,
        sixth_entries,
        damage_type,
        position,
    );

    let spell_choices = spell_choice_ids
        .into_iter()
        .map(|id| SpellMetadata {
            id,
            name: String::new(),
            icon_url: String::new(),
        })
        .collect();

    let mut top_rune_pages = extract_top_rune_pages(analysis);
    if top_rune_pages.is_empty() {
        top_rune_pages.push(RunePageOption {
            primary_style_id,
            sub_style_id,
            selected_perk_ids: selected_perk_ids.clone(),
            play: 1,
            win: 1,
            pick_rate: 1.0,
            ..Default::default()
        });
    } else {
        let first = &top_rune_pages[0];
        primary_style_id = first.primary_style_id;
        sub_style_id = first.sub_style_id;
        selected_perk_ids = first.selected_perk_ids.clone();
    }

    Ok(ChampionBuild {
        champion_id,
        champion_name: champion_name.to_string(),
        win_rate,
        pick_rate,
        primary_style_id,
        sub_style_id,
        selected_perk_ids,
        spell1_id,
        spell2_id,
        item_blocks,
        spell_choices,
        rune_pages: top_rune_pages,
        core_build_options,
        final_build_options,
        ..Default::default()
    })
}

/// Sends a request to the OP.GG MCP endpoint to execute `lol_get_champion_analysis`.
pub async fn fetch_opgg_champion_analysis(
    client: &reqwest::Client,
    endpoint_url: &str,
    champion_name: &str,
    queue_name: &str,
) -> Result<serde_json::Value, String> {
    let formatted_champ = champion_key_to_opgg_name(champion_name);
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "lol_get_champion_analysis",
            "arguments": {
                "game_mode": queue_name,
                "champion": formatted_champ,
                "position": "mid",
                "desired_output_fields": [
                    "data.damage_type",
                    "data.summary.average_stats.{win_rate,pick_rate,play}",
                    "data.runes[].{primary_page_id,primary_rune_ids[],secondary_page_id,secondary_rune_ids[],stat_mod_ids[],play,win}",
                    "data.summoner_spells.{ids[],play,win}",
                    "data.starter_items.{ids[],play,win,pick_rate}",
                    "data.core_items.{ids[],play,win,pick_rate}",
                    "data.boots[].{ids[],play,win,pick_rate}",
                    "data.last_items[].{ids[],play,win,pick_rate}",
                    "data.fourth_items[].{ids[],play,win,pick_rate}",
                    "data.fifth_items[].{ids[],play,win,pick_rate}",
                    "data.sixth_items[].{ids[],play,win,pick_rate}"
                ]
            }
        }
    });

    let res = client
        .post(endpoint_url)
        .json(&payload)
        .send()
        .await
        .map_err(|err| format!("HTTP request to OP.GG failed: {}", err))?;

    if !res.status().is_success() {
        return Err(format!("OP.GG responded with HTTP error: {}", res.status()));
    }

    let json_body: serde_json::Value = res
        .json()
        .await
        .map_err(|err| format!("Failed to parse JSON response from OP.GG: {}", err))?;

    Ok(json_body)
}

/// High-level function that fetches the recommended build for a champion with automatic resilient fallback.
pub async fn get_recommended_build_with_endpoint(
    client: &reqwest::Client,
    endpoint_url: &str,
    champion_id: i32,
    queue_id: i32,
    ddragon_cache: &DataDragonCache,
) -> ChampionBuild {
    let champ_name = ddragon_cache
        .champions
        .get(&champion_id)
        .map(|c| c.key_name.as_str())
        .unwrap_or("Unknown");

    let queue_param = opgg_queue_param_from_id(queue_id);

    let mut build =
        match fetch_opgg_champion_analysis(client, endpoint_url, champ_name, queue_param).await {
            Ok(json_res) => match parse_opgg_analysis_payload(&json_res, champion_id, champ_name) {
                Ok(build) => build,
                Err(_) => build_fallback_champion_build(champion_id, champ_name, queue_id),
            },
            Err(_) => build_fallback_champion_build(champion_id, champ_name, queue_id),
        };

    enrich_build_with_ddragon(&mut build, ddragon_cache);
    build
}

/// Fetches available patch versions from the OP.GG REST API.
pub async fn fetch_opgg_versions(client: &reqwest::Client, region: &str) -> Vec<String> {
    let url = format!(
        "{}/{}/champions/ranked/versions",
        OPGG_REST_API_BASE, region
    );
    if let Ok(res) = client.get(&url).send().await
        && let Ok(json) = res.json::<serde_json::Value>().await
        && let Some(arr) = json.get("data").and_then(|d| d.as_array())
    {
        let vers: Vec<String> = arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        if !vers.is_empty() {
            return vers;
        }
    }
    vec!["16.22".to_string(), "16.21".to_string()]
}

/// Fetches champion build analysis from the OP.GG REST API with full filter parameters.
pub async fn fetch_opgg_rest_champion_analysis(
    client: &reqwest::Client,
    region: &str,
    mode: &str,
    champion_id: i32,
    position: &str,
    tier: &str,
    version: Option<&str>,
) -> Result<serde_json::Value, String> {
    let mut url = format!(
        "{}/{}/champions/{}/{}/{}?tier={}",
        OPGG_REST_API_BASE, region, mode, champion_id, position, tier
    );
    if let Some(v) = version
        && !v.trim().is_empty()
    {
        url.push_str(&format!("&version={}", v.trim()));
    }

    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("REST request to OP.GG failed: {}", e))?;

    if !res.status().is_success() {
        return Err(format!(
            "OP.GG REST responded with HTTP error: {}",
            res.status()
        ));
    }

    let json_body: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse JSON response from OP.GG REST: {}", e))?;

    Ok(json_body)
}

/// Fetches separate `fourth_items`, `fifth_items`, and `sixth_items` lists from OP.GG MCP.
pub async fn fetch_opgg_mcp_depth_items(
    client: &reqwest::Client,
    endpoint_url: &str,
    champion_name: &str,
    game_mode: &str,
    position: &str,
    tier: &str,
) -> Result<(Vec<ItemStatEntry>, Vec<ItemStatEntry>, Vec<ItemStatEntry>), String> {
    let formatted_champ = champion_key_to_opgg_name(champion_name);
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "lol_get_champion_analysis",
            "arguments": {
                "game_mode": game_mode,
                "champion": formatted_champ,
                "position": position,
                "tier": tier,
                "desired_output_fields": [
                    "data.fourth_items[].{ids[],play,win,pick_rate}",
                    "data.fifth_items[].{ids[],play,win,pick_rate}",
                    "data.sixth_items[].{ids[],play,win,pick_rate}"
                ]
            }
        }
    });

    let res = client
        .post(endpoint_url)
        .json(&payload)
        .send()
        .await
        .map_err(|err| format!("MCP depth items request failed: {}", err))?;

    if !res.status().is_success() {
        return Err(format!("MCP depth items HTTP error: {}", res.status()));
    }

    let json_body: serde_json::Value = res
        .json()
        .await
        .map_err(|err| format!("Failed to parse MCP depth items JSON: {}", err))?;

    let text = json_body
        .pointer("/result/content/0/text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing text in MCP depth items response".to_string())?;

    let grouped = extract_mcp_grouped_item_lists(text);
    let mut fourth = Vec::new();
    let mut fifth = Vec::new();
    let mut sixth = Vec::new();
    for (name, items) in grouped {
        match name.as_str() {
            "fourth_items" => fourth = items,
            "fifth_items" => fifth = items,
            "sixth_items" => sixth = items,
            _ => {}
        }
    }
    Ok((fourth, fifth, sixth))
}

/// Fetches recommended build for a champion filtered by position, tier, region, and patch.
pub async fn get_recommended_build_with_filters(
    client: &reqwest::Client,
    champion_id: i32,
    queue_id: i32,
    position: Option<&str>,
    tier: Option<&str>,
    region: Option<&str>,
    patch: Option<&str>,
) -> Result<ChampionBuild, String> {
    let ddragon_cache = get_or_init_ddragon_cache(client).await?;
    let champ_name = ddragon_cache
        .champions
        .get(&champion_id)
        .map(|c| c.key_name.as_str())
        .unwrap_or("Unknown");

    let is_aram = queue_id == 450;
    let mode = if is_aram { "aram" } else { "ranked" };
    let target_region = region.unwrap_or("global");
    let target_tier = tier.unwrap_or("emerald_plus");

    // 1. Fetch available versions for region
    let available_versions = fetch_opgg_versions(client, target_region).await;

    // 2. Determine effective patch (2-day volume protection rule)
    let (mut effective_patch, mut patch_fallback_applied) = if let Some(p) = patch {
        if !p.trim().is_empty() {
            (p.to_string(), false)
        } else {
            (
                available_versions.first().cloned().unwrap_or_default(),
                false,
            )
        }
    } else {
        (
            available_versions.first().cloned().unwrap_or_default(),
            false,
        )
    };

    let target_position = if is_aram {
        "mid".to_string()
    } else if let Some(p) = position
        && !p.trim().is_empty()
        && p != "auto"
    {
        p.to_lowercase()
    } else {
        match resolve_champion_most_played_lane(
            client,
            champion_id,
            target_region,
            target_tier,
            Some(&effective_patch),
        )
        .await
        {
            Ok(lane) => lane,
            Err(_) => champion_default_lane(champ_name).to_string(),
        }
    };

    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // If user did not manually specify a patch, inspect the latest patch's created_at
    if (patch.is_none() || patch.unwrap_or("").is_empty())
        && let Ok(probe_json) = fetch_opgg_rest_champion_analysis(
            client,
            target_region,
            mode,
            champion_id,
            &target_position,
            target_tier,
            Some(&effective_patch),
        )
        .await
    {
        let created_at_iso = probe_json
            .pointer("/data/trends/win/0/created_at")
            .and_then(|v| v.as_str());
        let prev_patch_created_at_iso = probe_json
            .pointer("/data/trends/win/1/created_at")
            .and_then(|v| v.as_str());
        let play_count = probe_json
            .pointer("/data/summary/average_stats/play")
            .or_else(|| probe_json.pointer("/data/average_stats/play"))
            .and_then(|v| v.as_u64());

        let (chosen_patch, fallback) = select_effective_patch_with_volume(
            &available_versions,
            created_at_iso,
            prev_patch_created_at_iso,
            play_count,
            now_unix,
            None,
        );
        effective_patch = chosen_patch;
        patch_fallback_applied = fallback;
    }

    // 3. Fetch build with effective patch (and enrich with MCP fourth/fifth/sixth item lists)
    let mcp_depth_fut = fetch_opgg_mcp_depth_items(
        client,
        OPGG_MCP_ENDPOINT,
        champ_name,
        mode,
        &target_position,
        target_tier,
    );
    let rest_fut = fetch_opgg_rest_champion_analysis(
        client,
        target_region,
        mode,
        champion_id,
        &target_position,
        target_tier,
        Some(&effective_patch),
    );
    let (rest_res, mcp_depth_res) = tokio::join!(rest_fut, mcp_depth_fut);

    let mut build = match rest_res {
        Ok(mut json_res) => {
            if let Some(data_obj) = json_res.get_mut("data").and_then(|d| d.as_object_mut()) {
                data_obj.insert("position".to_string(), serde_json::json!(target_position));
            }
            if let Ok((fourth, fifth, sixth)) = mcp_depth_res
                && let Some(data_obj) = json_res.get_mut("data").and_then(|d| d.as_object_mut())
            {
                let to_json_arr = |entries: Vec<ItemStatEntry>| -> serde_json::Value {
                    serde_json::Value::Array(
                        entries
                            .into_iter()
                            .map(|e| {
                                serde_json::json!({
                                    "ids": e.ids,
                                    "play": e.play,
                                    "win": e.win,
                                    "pick_rate": e.pick_rate
                                })
                            })
                            .collect(),
                    )
                };
                if !fourth.is_empty() {
                    data_obj.insert("fourth_items".to_string(), to_json_arr(fourth));
                }
                if !fifth.is_empty() {
                    data_obj.insert("fifth_items".to_string(), to_json_arr(fifth));
                }
                if !sixth.is_empty() {
                    data_obj.insert("sixth_items".to_string(), to_json_arr(sixth));
                }
            }
            match parse_opgg_analysis_payload(&json_res, champion_id, champ_name) {
                Ok(b) => b,
                Err(_) => {
                    get_recommended_build_with_endpoint(
                        client,
                        OPGG_MCP_ENDPOINT,
                        champion_id,
                        queue_id,
                        ddragon_cache,
                    )
                    .await
                }
            }
        }
        Err(_) => {
            get_recommended_build_with_endpoint(
                client,
                OPGG_MCP_ENDPOINT,
                champion_id,
                queue_id,
                ddragon_cache,
            )
            .await
        }
    };

    build.active_position = target_position;
    build.active_tier = target_tier.to_string();
    build.active_region = target_region.to_string();
    build.active_patch = effective_patch;
    build.available_patches = available_versions;
    build.patch_fallback_applied = patch_fallback_applied;

    enrich_build_with_ddragon(&mut build, ddragon_cache);
    Ok(build)
}

/// Production entry point to obtain recommended build for champion.
pub async fn get_recommended_build(
    client: &reqwest::Client,
    champion_id: i32,
    queue_id: i32,
) -> Result<ChampionBuild, String> {
    get_recommended_build_with_filters(client, champion_id, queue_id, None, None, None, None).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_opgg_queue_param_and_default_spells_adaptation() {
        assert_eq!(opgg_queue_param_from_id(450), "aram");
        assert_eq!(default_spells_for_queue(450), (4, 32)); // Flash + Snowball

        assert_eq!(opgg_queue_param_from_id(420), "ranked");
        assert_eq!(default_spells_for_queue(420), (4, 14)); // Flash + Ignite

        assert_eq!(opgg_queue_param_from_id(440), "ranked");
        assert_eq!(opgg_queue_param_from_id(400), "ranked");
        assert_eq!(opgg_queue_param_from_id(-1), "ranked");
        assert_eq!(default_spells_for_queue(-1), (4, 14));
    }

    #[test]
    fn test_fallback_build_has_valid_9_perks_and_queue_spells() {
        let aram_fallback = build_fallback_champion_build(103, "Ahri", 450);
        assert_eq!(aram_fallback.selected_perk_ids.len(), 9);
        assert_eq!(aram_fallback.spell1_id, 4);
        assert_eq!(aram_fallback.spell2_id, 32);
        assert_eq!(aram_fallback.item_blocks.len(), 5);
        assert_eq!(aram_fallback.item_blocks[0].title, "Itens Iniciais");
        assert_eq!(aram_fallback.item_blocks[1].title, "Primeiros 3 Itens");
        assert_eq!(aram_fallback.item_blocks[2].title, "3 Botas Mais Feitas");
        assert_eq!(aram_fallback.item_blocks[3].title, "Build Final");
        assert_eq!(
            aram_fallback.item_blocks[4].title,
            "Itens Situacionais / Úteis"
        );

        let ranked_fallback = build_fallback_champion_build(103, "Ahri", 420);
        assert_eq!(ranked_fallback.selected_perk_ids.len(), 9);
        assert_eq!(ranked_fallback.spell1_id, 4);
        assert_eq!(ranked_fallback.spell2_id, 14);

        let practice_fallback = build_fallback_champion_build(103, "Ahri", -1);
        assert_eq!(practice_fallback.selected_perk_ids.len(), 9);
        assert_eq!(practice_fallback.spell1_id, 4);
        assert_eq!(practice_fallback.spell2_id, 14);
        assert_eq!(practice_fallback.item_blocks[0].item_ids, vec![1055, 2003]);
    }

    #[test]
    fn test_enrich_build_with_ddragon_populates_urls_spells_and_shards() {
        let mut ddragon_cache = DataDragonCache {
            version: "14.23.1".to_string(),
            champions: HashMap::new(),
            runes: HashMap::new(),
        };

        ddragon_cache.champions.insert(
            103,
            ChampionMetadata {
                champion_id: 103,
                key_name: "Ahri".to_string(),
                display_name: "Ahri".to_string(),
                icon_url: "https://ddragon.leagueoflegends.com/cdn/14.23.1/img/champion/Ahri.png"
                    .to_string(),
            },
        );

        ddragon_cache.runes.insert(
            8100,
            RuneMetadata {
                id: 8100,
                key: "Domination".to_string(),
                name: "Dominação".to_string(),
                icon_url: "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/7200_Domination.png".to_string(),
            },
        );

        let mut build = build_fallback_champion_build(103, "Ahri", 420);
        enrich_build_with_ddragon(&mut build, &ddragon_cache);

        assert_eq!(
            build.champion_icon_url,
            "https://ddragon.leagueoflegends.com/cdn/14.23.1/img/champion/Ahri.png"
        );
        assert_eq!(build.spells.len(), 2);
        assert_eq!(build.spells[0].name, "Flash");
        assert!(build.spells[0].icon_url.contains("SummonerFlash.png"));
        assert_eq!(build.perks.len(), 9);
        assert_eq!(build.item_blocks.len(), 5);
        assert!(!build.item_blocks[0].item_icon_urls.is_empty());
        assert!(build.item_blocks[0].item_icon_urls[0].contains("/img/item/"));
    }

    #[test]
    fn test_parse_latest_ddragon_version() {
        let versions = serde_json::json!(["14.23.1", "14.22.1", "14.21.1"]);
        assert_eq!(parse_latest_ddragon_version(&versions).unwrap(), "14.23.1");

        let empty = serde_json::json!([]);
        assert!(parse_latest_ddragon_version(&empty).is_err());
    }

    #[test]
    fn test_parse_ddragon_runes_extracts_trees_and_perks() {
        let runes_fixture = serde_json::json!([
            {
                "id": 8100,
                "key": "Domination",
                "icon": "perk-images/Styles/7200_Domination.png",
                "name": "Domination",
                "slots": [
                    {
                        "runes": [
                            {
                                "id": 8112,
                                "key": "Electrocute",
                                "icon": "perk-images/Styles/Domination/Electrocute/Electrocute.png",
                                "name": "Electrocute"
                            }
                        ]
                    }
                ]
            }
        ]);

        let result = parse_ddragon_runes(&runes_fixture);
        assert!(result.is_ok());

        let map = result.unwrap();
        assert_eq!(map.len(), 2);

        // Check primary tree
        let dom = map.get(&8100).expect("Domination tree should exist");
        assert_eq!(dom.name, "Domination");
        assert!(dom.icon_url.contains("7200_Domination.png"));

        // Check keystone
        let elec = map.get(&8112).expect("Electrocute should exist");
        assert_eq!(elec.name, "Electrocute");
        assert!(elec.icon_url.contains("Electrocute.png"));
    }

    #[test]
    fn test_parse_ddragon_champion_by_id_finds_ahri() {
        let ddragon_fixture = serde_json::json!({
            "type": "champion",
            "format": "standAloneComplex",
            "version": "14.23.1",
            "data": {
                "Aatrox": {
                    "version": "14.23.1",
                    "id": "Aatrox",
                    "key": "266",
                    "name": "Aatrox",
                    "image": { "full": "Aatrox.png" }
                },
                "Ahri": {
                    "version": "14.23.1",
                    "id": "Ahri",
                    "key": "103",
                    "name": "Ahri",
                    "image": { "full": "Ahri.png" }
                }
            }
        });

        let result = parse_ddragon_champion_by_id(&ddragon_fixture, "14.23.1", 103);
        assert!(result.is_ok());

        let meta = result.unwrap();
        assert_eq!(meta.champion_id, 103);
        assert_eq!(meta.key_name, "Ahri");
        assert_eq!(meta.display_name, "Ahri");
        assert_eq!(
            meta.icon_url,
            "https://ddragon.leagueoflegends.com/cdn/14.23.1/img/champion/Ahri.png"
        );
    }

    #[tokio::test]
    async fn test_load_ddragon_cache_with_wiremock() {
        let mock_server = MockServer::start().await;

        let versions_json = serde_json::json!(["14.23.1"]);
        let champion_json = serde_json::json!({
            "data": {
                "Ahri": {
                    "id": "Ahri",
                    "key": "103",
                    "name": "Ahri",
                    "image": { "full": "Ahri.png" }
                }
            }
        });
        let runes_json = serde_json::json!([
            {
                "id": 8100,
                "key": "Domination",
                "icon": "Dom.png",
                "name": "Domination",
                "slots": []
            }
        ]);

        Mock::given(method("GET"))
            .and(path("/api/versions.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(versions_json))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/cdn/14.23.1/data/pt_BR/champion.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(champion_json))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/cdn/14.23.1/data/pt_BR/runesReforged.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(runes_json))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let cache = load_ddragon_cache_from_base_url(&client, &mock_server.uri())
            .await
            .expect("Loading ddragon cache should succeed");

        assert_eq!(cache.version, "14.23.1");
        assert!(cache.champions.contains_key(&103));
        assert_eq!(cache.champions.get(&103).unwrap().display_name, "Ahri");
        assert!(cache.runes.contains_key(&8100));
    }

    #[test]
    fn test_parse_opgg_analysis_extracts_9_runes_spells_and_items() {
        let opgg_fixture = serde_json::json!({
            "data": {
                "win_rate": 51.85,
                "pick_rate": 8.42,
                "runes": {
                    "primary_style_id": 8100,
                    "sub_style_id": 8200,
                    "primary_perk_ids": [8112, 8126, 8138, 8135],
                    "sub_perk_ids": [8226, 8210],
                    "stat_shards": [5008, 5008, 5001]
                },
                "spells": [4, 14],
                "items": {
                    "starters": [1056, 2003],
                    "core": [3285, 3020, 3089],
                    "situational": [3157, 3135],
                    "boots": [3020, 3111, 3047]
                }
            }
        });

        let result = parse_opgg_analysis_payload(&opgg_fixture, 103, "Ahri");
        assert!(result.is_ok());

        let build = result.unwrap();
        assert_eq!(build.champion_id, 103);
        assert_eq!(build.champion_name, "Ahri");
        assert_eq!(build.primary_style_id, 8100);
        assert_eq!(build.sub_style_id, 8200);
        assert_eq!(build.selected_perk_ids.len(), 9);
        assert_eq!(
            build.selected_perk_ids,
            vec![8112, 8126, 8138, 8135, 8226, 8210, 5008, 5008, 5001]
        );
        assert_eq!(build.spell1_id, 4);
        assert_eq!(build.spell2_id, 14);
        assert_eq!(build.item_blocks.len(), 5);
        assert_eq!(build.item_blocks[0].title, "Itens Iniciais");
        assert_eq!(build.item_blocks[1].title, "Primeiros 3 Itens");
        // 3020 is a boot, so it was filtered out and filled with situational item 3157, ensuring 3 core non-boot items!
        assert_eq!(build.item_blocks[1].item_ids, vec![3285, 3089, 3157]);
        assert_eq!(build.item_blocks[2].title, "3 Botas Mais Feitas");
        assert_eq!(build.item_blocks[2].item_ids, vec![3020, 3111, 3047]);
        assert_eq!(build.item_blocks[3].title, "Build Final");
        assert_eq!(build.item_blocks[4].title, "Itens Situacionais / Úteis");
        // Verify situational items are disjoint from core and final build!
        for &item in &build.item_blocks[4].item_ids {
            assert!(!build.item_blocks[1].item_ids.contains(&item));
            assert!(!build.item_blocks[3].item_ids.contains(&item));
            assert!(!is_boot_item(item));
        }
    }

    #[tokio::test]
    async fn test_fetch_opgg_champion_analysis_with_wiremock() {
        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "data": {
                    "win_rate": 52.4,
                    "pick_rate": 11.2,
                    "runes": {
                        "primary_style_id": 8000,
                        "sub_style_id": 8100,
                        "primary_perk_ids": [8005, 9111, 9104, 8014],
                        "sub_perk_ids": [8139, 8135],
                        "stat_shards": [5008, 5008, 5003]
                    },
                    "spells": [4, 7],
                    "items": {
                        "starters": [1055, 2003],
                        "core": [3031, 3006, 3094]
                    }
                }
            }
        });

        Mock::given(method("POST"))
            .and(path("/mcp"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let mcp_url = format!("{}/mcp", mock_server.uri());

        let raw_res = fetch_opgg_champion_analysis(&client, &mcp_url, "Jinx", "ranked")
            .await
            .expect("Mock request should succeed");

        let build = parse_opgg_analysis_payload(&raw_res, 222, "Jinx")
            .expect("Parsing mock response should succeed");

        assert_eq!(build.champion_id, 222);
        assert_eq!(build.champion_name, "Jinx");
        assert_eq!(build.primary_style_id, 8000);
        assert_eq!(build.sub_style_id, 8100);
        assert_eq!(build.selected_perk_ids.len(), 9);
        assert_eq!(build.spell1_id, 4);
        assert_eq!(build.spell2_id, 7);
    }

    #[tokio::test]
    async fn test_get_recommended_build_uses_fallback_when_opgg_returns_500() {
        let mock_server = MockServer::start().await;

        // Simulate 500 Internal Server Error from OP.GG
        Mock::given(method("POST"))
            .and(path("/mcp"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let mcp_url = format!("{}/mcp", mock_server.uri());

        let mut champions_map = HashMap::new();
        champions_map.insert(
            103,
            ChampionMetadata {
                champion_id: 103,
                key_name: "Ahri".to_string(),
                display_name: "Ahri".to_string(),
                icon_url: "https://example.com/ahri.png".to_string(),
            },
        );

        let cache = DataDragonCache {
            version: "14.23.1".to_string(),
            champions: champions_map,
            runes: HashMap::new(),
        };

        // Even though OP.GG fails with 500, this function must not fail, returning a resilient fallback!
        let build = get_recommended_build_with_endpoint(&client, &mcp_url, 103, 450, &cache).await;

        assert_eq!(build.champion_id, 103);
        assert_eq!(build.champion_name, "Ahri");
        assert_eq!(build.selected_perk_ids.len(), 9);
        assert_eq!(build.spell1_id, 4);
        assert_eq!(build.spell2_id, 32); // ARAM snowball
    }

    #[test]
    fn test_parse_opgg_mcp_text_real_ahri_payload() {
        let ahri_mcp_json = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": "class LolGetChampionAnalysis: data\nclass Data: summary,core_items,boots,starter_items,fourth_items,fifth_items,sixth_items,summoner_spells,runes\nclass Summary: average_stats\nclass AverageStats: win_rate,pick_rate\nclass CoreItems: ids\nclass Runes: primary_page_id,primary_rune_ids,secondary_page_id,secondary_rune_ids,stat_mod_ids\n\nLolGetChampionAnalysis(Data(Summary(AverageStats(0.51,0.12)),CoreItems([3118,4645,3157]),CoreItems([3020]),CoreItems([1056,2003,2003]),[CoreItems([3089]),CoreItems([3157]),CoreItems([3135])],[CoreItems([3089]),CoreItems([3135]),CoreItems([3157])],[CoreItems([4629]),CoreItems([3135]),CoreItems([4646])],CoreItems([4,14]),Runes(8100,[8112,8139,8140,8106],8200,[8210,8226],[5005,5008,5001])))\""
                    }
                ]
            }
        });

        let build = parse_opgg_analysis_payload(&ahri_mcp_json, 103, "Ahri")
            .expect("Parsing real Ahri MCP payload should succeed");

        assert_eq!(build.champion_id, 103);
        assert_eq!(build.champion_name, "Ahri");
        // win_rate: 0.51 converted to 51.0%, pick_rate: 0.12 converted to 12.0%
        assert_eq!(build.win_rate, 51.0);
        assert_eq!(build.pick_rate, 12.0);

        // Runes: Domination 8100 + Sorcery 8200
        assert_eq!(build.primary_style_id, 8100);
        assert_eq!(build.sub_style_id, 8200);
        assert_eq!(
            build.selected_perk_ids,
            vec![8112, 8139, 8140, 8106, 8210, 8226, 5005, 5008, 5001]
        );

        // Spells: Flash (4) + Ignite (14)
        assert_eq!(build.spell1_id, 4);
        assert_eq!(build.spell2_id, 14);

        // Core 3 items: Malignance (3118), Shadowflame (4645), Zhonya's (3157) - NO BOOTS!
        assert_eq!(build.item_blocks[1].title, "Primeiros 3 Itens");
        assert_eq!(build.item_blocks[1].item_ids, vec![3118, 4645, 3157]);
        for item in &build.item_blocks[1].item_ids {
            assert!(
                !is_boot_item(*item),
                "Boot item {} must not be in Core 3 items",
                item
            );
        }

        assert_eq!(build.item_blocks.len(), 5);
        // Position 3: Boots block: Sorcerer's Shoes (3020)
        assert_eq!(build.item_blocks[2].title, "3 Botas Mais Feitas");
        assert!(build.item_blocks[2].item_ids.contains(&3020));

        // Position 4: Build Final
        assert_eq!(build.item_blocks[3].title, "Build Final");
        for &core_item in &build.item_blocks[1].item_ids {
            assert!(build.item_blocks[3].item_ids.contains(&core_item));
        }

        // Position 5: Itens Situacionais / Úteis
        assert_eq!(build.item_blocks[4].title, "Itens Situacionais / Úteis");
        assert!(!build.item_blocks[4].item_ids.is_empty());
        for &item in &build.item_blocks[4].item_ids {
            assert!(
                !build.item_blocks[1].item_ids.contains(&item),
                "Situational item {} cannot be in Core 3",
                item
            );
            assert!(
                !build.item_blocks[3].item_ids.contains(&item),
                "Situational item {} cannot be in Final Build",
                item
            );
            assert!(
                !is_boot_item(item),
                "Situational item {} cannot be a boot",
                item
            );
        }
    }

    #[test]
    fn test_build_ordering_strictly_by_play_count() {
        // Item 3118 (Malignance): 10,000 games, 5,100 wins (51.0% win rate)
        // Item 3089 (Rabadon): 1,000 games, 650 wins (65.0% win rate)
        // Item 4645 (Shadowflame): 8,000 games, 4,200 wins (52.5% win rate)
        // Item 3157 (Zhonya): 6,000 games, 3,300 wins (55.0% win rate)
        let starter = Some(ItemStatEntry {
            ids: vec![1056, 2003],
            play: 10000,
            win: 5000,
            pick_rate: 0.9,
        });
        let boots = vec![ItemStatEntry {
            ids: vec![3020],
            play: 8000,
            win: 4000,
            pick_rate: 0.8,
        }];
        let candidates = vec![
            ItemStatEntry {
                ids: vec![3118],
                play: 10000,
                win: 5100,
                pick_rate: 0.7,
            },
            ItemStatEntry {
                ids: vec![3089],
                play: 1000,
                win: 650,
                pick_rate: 0.2,
            },
            ItemStatEntry {
                ids: vec![4645],
                play: 8000,
                win: 4200,
                pick_rate: 0.5,
            },
            ItemStatEntry {
                ids: vec![3157],
                play: 6000,
                win: 3300,
                pick_rate: 0.4,
            },
        ];

        // Ordering MUST strictly follow play count descending (absolute matches played):
        let popular_blocks = build_5_item_blocks(starter, boots, candidates, "AP");
        // Ordered by play count: 3118 (10k), 4645 (8k), 3157 (6k)
        assert_eq!(popular_blocks[1].item_ids, vec![3118, 4645, 3157]);
    }

    #[test]
    fn test_boots_ranking_prioritizes_most_played_boots() {
        // Boot 3020 (Sorcerer's): 50,000 games, 50% win rate
        // Boot 3158 (Lucidity): 5,000 games, 60% win rate
        // Boot 3111 (Mercs): 10,000 games, 52% win rate
        let starter = Some(ItemStatEntry {
            ids: vec![1056, 2003],
            play: 50000,
            win: 25000,
            pick_rate: 0.9,
        });
        let boots = vec![
            ItemStatEntry {
                ids: vec![3158],
                play: 5000,
                win: 3000,
                pick_rate: 0.1,
            },
            ItemStatEntry {
                ids: vec![3020],
                play: 50000,
                win: 25000,
                pick_rate: 0.7,
            },
            ItemStatEntry {
                ids: vec![3111],
                play: 10000,
                win: 5200,
                pick_rate: 0.2,
            },
        ];
        let candidates = vec![ItemStatEntry {
            ids: vec![3118, 4645, 3089],
            play: 20000,
            win: 11000,
            pick_rate: 0.5,
        }];

        let blocks = build_5_item_blocks(starter, boots, candidates, "AP");

        // Boots MUST be sorted by play count descending: 3020 (50k) -> 3111 (10k) -> 3158 (5k)
        assert_eq!(blocks[2].title, "3 Botas Mais Feitas");
        assert_eq!(blocks[2].item_ids, vec![3020, 3111, 3158]);
    }

    #[test]
    fn test_situational_items_filtered_and_sorted_by_play() {
        let starter = Some(ItemStatEntry {
            ids: vec![1056, 2003],
            play: 50000,
            win: 25000,
            pick_rate: 0.9,
        });
        let boots = vec![ItemStatEntry {
            ids: vec![3020],
            play: 40000,
            win: 20000,
            pick_rate: 0.8,
        }];
        let candidates = vec![
            ItemStatEntry {
                ids: vec![3118, 4645, 3157],
                play: 20000,
                win: 10000,
                pick_rate: 0.4,
            }, // Core
            ItemStatEntry {
                ids: vec![3089],
                play: 8000,
                win: 4500,
                pick_rate: 0.3,
            }, // Final build 4th
            ItemStatEntry {
                ids: vec![3135],
                play: 5000,
                win: 2500,
                pick_rate: 0.2,
            }, // Final build 5th
            ItemStatEntry {
                ids: vec![3102],
                play: 4000,
                win: 2100,
                pick_rate: 0.18,
            }, // Final build 6th
            ItemStatEntry {
                ids: vec![4646],
                play: 1500,
                win: 700,
                pick_rate: 0.1,
            }, // Situational (1500 plays)
            ItemStatEntry {
                ids: vec![4629],
                play: 3200,
                win: 1600,
                pick_rate: 0.15,
            }, // Situational (3200 plays)
            ItemStatEntry {
                ids: vec![3165],
                play: 700,
                win: 350,
                pick_rate: 0.05,
            }, // Situational (700 plays)
        ];

        let blocks = build_5_item_blocks(starter, boots, candidates, "AP");

        // Clean core (first 3): 3118, 4645, 3157
        assert_eq!(blocks[1].item_ids, vec![3118, 4645, 3157]);
        // Final build (6 items): 3118, 4645, 3157, 3089, 3135, 3102
        assert_eq!(blocks[3].item_ids, vec![3118, 4645, 3157, 3089, 3135, 3102]);

        // Situational items:
        // Must NOT contain any item from core or final build:
        for &id in &blocks[4].item_ids {
            assert!(!blocks[1].item_ids.contains(&id));
            assert!(!blocks[3].item_ids.contains(&id));
            assert!(!is_boot_item(id));
        }

        // Must be ordered strictly by play count descending:
        // 4629 (3200 plays) should appear BEFORE 4646 (1500 plays) which is BEFORE 3165 (700 plays)!
        let pos_4629 = blocks[4].item_ids.iter().position(|&x| x == 4629).unwrap();
        let pos_4646 = blocks[4].item_ids.iter().position(|&x| x == 4646).unwrap();
        let pos_3165 = blocks[4].item_ids.iter().position(|&x| x == 3165).unwrap();
        assert!(pos_4629 < pos_4646);
        assert!(pos_4646 < pos_3165);
    }
    #[test]
    fn test_parse_opgg_mcp_text_with_play_in_average_stats() {
        let text_with_play = "class LolGetChampionAnalysis: data\nclass Data: summary\nclass Summary: average_stats\nclass AverageStats: play,win_rate,pick_rate\n\nLolGetChampionAnalysis(Data(Summary(AverageStats(80342,0.512,0.118)),CoreItems([3118,4645,3157]),CoreItems([3020]),CoreItems([1056,2003]),CoreItems([4,14]),Runes(8100,[8112,8139,8140,8106],8200,[8210,8226],[5005,5008,5001])))";
        let build = parse_opgg_mcp_text(text_with_play, 103, "Ahri")
            .expect("Parsing payload with play field in AverageStats should succeed");
        assert_eq!(build.win_rate, 51.2);
        assert_eq!(build.pick_rate, 11.8);
        assert_eq!(build.item_blocks[1].item_ids, vec![3118, 4645, 3157]);
        // Build final first 3 items must be identical to clean_core in exact sequential order
        assert_eq!(&build.item_blocks[3].item_ids[..3], &[3118, 4645, 3157]);
    }
    #[test]
    fn test_parse_iso8601_to_unix() {
        let ts = parse_iso8601_to_unix("2026-11-07T22:59:29+09:00");
        assert!(ts.is_some());
        let unix_secs = ts.unwrap();
        assert!(unix_secs > 1_700_000_000);
    }

    #[test]
    fn test_select_effective_patch_under_48h_falls_back() {
        let versions = vec!["16.22".to_string(), "16.21".to_string()];
        let created_at = "2026-11-07T22:59:29+09:00";
        let created_unix = parse_iso8601_to_unix(created_at).unwrap();
        let now_unix = created_unix + 36000; // 10h later

        let (patch, fallback) = select_effective_patch(&versions, Some(created_at), now_unix, None);
        assert_eq!(patch, "16.21");
        assert!(fallback);
    }

    #[test]
    fn test_select_effective_patch_over_48h_keeps_latest() {
        let versions = vec!["16.22".to_string(), "16.21".to_string()];
        let created_at = "2026-11-05T12:00:00Z";
        let created_unix = parse_iso8601_to_unix(created_at).unwrap();
        let now_unix = created_unix + (3 * 86400); // 3 days later

        let (patch, fallback) = select_effective_patch(&versions, Some(created_at), now_unix, None);
        assert_eq!(patch, "16.22");
        assert!(!fallback);
    }

    #[test]
    fn test_select_effective_patch_respects_user_override() {
        let versions = vec!["16.22".to_string(), "16.21".to_string()];
        let created_at = "2026-11-07T22:59:29+09:00";
        let created_unix = parse_iso8601_to_unix(created_at).unwrap();
        let now_unix = created_unix + 1000;

        let (patch, fallback) =
            select_effective_patch(&versions, Some(created_at), now_unix, Some("16.22"));
        assert_eq!(patch, "16.22");
        assert!(!fallback);
    }

    #[test]
    fn test_parse_opgg_champion_positions_ranking_and_select_most_played_lane() {
        let json = serde_json::json!({
            "data": [
                {
                    "id": 157, // Yasuo
                    "positions": [
                        { "name": "MID", "stats": { "play": 57307, "role_rate": 0.60 } },
                        { "name": "TOP", "stats": { "play": 22898, "role_rate": 0.24 } },
                        { "name": "ADC", "stats": { "play": 14609, "role_rate": 0.15 } }
                    ]
                },
                {
                    "id": 79, // Gragas
                    "positions": [
                        { "name": "MID", "stats": { "play": 3491, "role_rate": 0.12 } },
                        { "name": "TOP", "stats": { "play": 16785, "role_rate": 0.60 } },
                        { "name": "JUNGLE", "stats": { "play": 5731, "role_rate": 0.20 } }
                    ]
                },
                {
                    "id": 147, // Seraphine
                    "positions": [
                        { "name": "SUPPORT", "stats": { "play": 44180, "role_rate": 0.91 } }
                    ]
                }
            ]
        });

        let map = parse_opgg_champion_positions_ranking(&json);
        assert_eq!(map.len(), 3);

        // Yasuo should pick Mid (57k > 22k > 14k)
        let yasuo_pos = map.get(&157).unwrap();
        assert_eq!(select_most_played_lane(yasuo_pos), Some("mid"));

        // Gragas should pick Top (16k > 5k > 3k)
        let gragas_pos = map.get(&79).unwrap();
        assert_eq!(select_most_played_lane(gragas_pos), Some("top"));

        // Seraphine should pick Support
        let sera_pos = map.get(&147).unwrap();
        assert_eq!(select_most_played_lane(sera_pos), Some("support"));
    }

    #[tokio::test]
    async fn test_resolve_champion_most_played_lane_cached() {
        let champ_id = 99999;
        let positions = vec![
            OpggPositionInfo {
                name: "TOP".to_string(),
                play: 100,
                role_rate: 0.1,
            },
            OpggPositionInfo {
                name: "ADC".to_string(),
                play: 9500,
                role_rate: 0.9,
            },
        ];

        let mut inner_map = HashMap::new();
        inner_map.insert(champ_id, positions);

        let cache_key = "global:emerald_plus:test_v1".to_string();
        if let Ok(mut guard) = CHAMPION_POSITIONS_CACHE.write() {
            guard.insert(cache_key, inner_map);
        }

        let dummy_client = reqwest::Client::new();
        let resolved = resolve_champion_most_played_lane(
            &dummy_client,
            champ_id,
            "global",
            "emerald_plus",
            Some("test_v1"),
        )
        .await;

        assert_eq!(resolved, Ok("adc".to_string()));
    }

    #[test]
    fn test_select_effective_patch_with_volume_keeps_latest_when_play_count_is_high() {
        let versions = vec!["16.22".to_string(), "16.21".to_string()];
        // Even if snapshot created_at was today (under 48h):
        let created_at = "2026-11-07T22:59:29+09:00";
        let created_unix = parse_iso8601_to_unix(created_at).unwrap();
        let now_unix = created_unix + 10000;

        // When play count is >= 1000, 16.22 is kept with fallback = false
        let (patch, fallback) = select_effective_patch_with_volume(
            &versions,
            Some(created_at),
            None,
            Some(84510),
            now_unix,
            None,
        );
        assert_eq!(patch, "16.22");
        assert!(!fallback);
    }

    #[test]
    fn test_extract_most_used_summoner_spells_from_opgg_rest_structure() {
        let json = serde_json::json!({
            "data": {
                "win_rate": 49.5,
                "pick_rate": 12.1,
                "runes": {
                    "primary_style_id": 8000,
                    "sub_style_id": 8300,
                    "primary_perk_ids": [8008, 9111, 9104, 8017],
                    "sub_perk_ids": [8304, 8345],
                    "stat_shards": [5005, 5008, 5011]
                },
                "summoner_spells": [
                    {
                        "ids": [4, 21], // Flash + Barrier (most played: 62087 games)
                        "win": 30822,
                        "play": 62087,
                        "pick_rate": 0.7006
                    },
                    {
                        "ids": [3, 4], // Exhaust + Flash (second: 17529 games)
                        "win": 8894,
                        "play": 17529,
                        "pick_rate": 0.1978
                    },
                    {
                        "ids": [4, 14], // Flash + Ignite (minority: 500 games)
                        "win": 250,
                        "play": 500,
                        "pick_rate": 0.005
                    }
                ],
                "items": {
                    "starters": [1055, 2003],
                    "core": [3031, 3094, 3036]
                }
            }
        });

        let build = parse_opgg_analysis_payload(&json, 804, "Yunara")
            .expect("Parsing Yunara with OP.GG summoner_spells array should succeed");

        // The most played combination is Flash (4) + Barrier (21), NOT Ignite (14)!
        assert_eq!(build.spell1_id, 4);
        assert_eq!(build.spell2_id, 21);
    }

    #[test]
    fn test_build_final_and_situational_exclude_components_and_cap_at_6() {
        let starter = Some(ItemStatEntry {
            ids: vec![1055, 2003],
            play: 10000,
            win: 5000,
            pick_rate: 0.9,
        });
        let boots = vec![ItemStatEntry {
            ids: vec![3006],
            play: 10000,
            win: 5000,
            pick_rate: 0.8,
        }];
        let candidates = vec![
            ItemStatEntry {
                ids: vec![6672, 3085, 3031],
                play: 16000,
                win: 8000,
                pick_rate: 0.5,
            }, // Core
            ItemStatEntry {
                ids: vec![3032],
                play: 12000,
                win: 6000,
                pick_rate: 0.4,
            }, // Yun Tal (4th)
            ItemStatEntry {
                ids: vec![1038],
                play: 11000,
                win: 5500,
                pick_rate: 0.35,
            }, // B.F. Sword (component - MUST BE SKIPPED!)
            ItemStatEntry {
                ids: vec![3036],
                play: 10000,
                win: 5000,
                pick_rate: 0.3,
            }, // LDR (5th)
            ItemStatEntry {
                ids: vec![3026],
                play: 9000,
                win: 4500,
                pick_rate: 0.28,
            }, // GA (6th)
            ItemStatEntry {
                ids: vec![2523],
                play: 8000,
                win: 4000,
                pick_rate: 0.25,
            }, // Situational 1
            ItemStatEntry {
                ids: vec![2512],
                play: 7000,
                win: 3500,
                pick_rate: 0.2,
            }, // Situational 2
            ItemStatEntry {
                ids: vec![3033],
                play: 6000,
                win: 3000,
                pick_rate: 0.18,
            }, // Situational 3
            ItemStatEntry {
                ids: vec![6673],
                play: 4000,
                win: 2000,
                pick_rate: 0.12,
            }, // Situational 4
            ItemStatEntry {
                ids: vec![3072],
                play: 3000,
                win: 1500,
                pick_rate: 0.1,
            }, // Situational 5
            ItemStatEntry {
                ids: vec![3139],
                play: 2000,
                win: 1000,
                pick_rate: 0.08,
            }, // Situational 6
            ItemStatEntry {
                ids: vec![3046],
                play: 1000,
                win: 500,
                pick_rate: 0.04,
            }, // Situational 7 (should be dropped by cap)
        ];

        let blocks = build_5_item_blocks(starter, boots, candidates, "AD");

        // Core items
        assert_eq!(blocks[1].item_ids, vec![6672, 3085, 3031]);

        // Build Final: 6 items, must NEVER contain component 1038 (B.F. Sword)!
        // Should contain core (6672, 3085, 3031) followed by 3032 (4th), 3036 (5th), and 3026 (6th)
        assert_eq!(blocks[3].item_ids, vec![6672, 3085, 3031, 3032, 3036, 3026]);

        // Situational items: max 6 items, must not contain 1038 or any core/final items
        assert_eq!(blocks[4].item_ids.len(), 6);
        assert!(!blocks[4].item_ids.contains(&1038));
        for &id in &blocks[4].item_ids {
            assert!(!blocks[1].item_ids.contains(&id));
            assert!(!blocks[3].item_ids.contains(&id));
        }
    }

    #[test]
    fn test_core_builds_and_separate_fourth_fifth_sixth_lists_yunara() {
        // Mirrors the exact OP.GG Yunara tables from the user's screenshot:
        // Core builds:
        //   1. [6672, 3085, 3031] -> 19,057 Games
        //   2. [3032, 3085, 3031] -> 15,006 Games
        //   3. [6672, 3085, 2523] -> 5,163 Games
        // Fourth Item:
        //   1. [3036] (LDR) -> 18,151 Games
        //   2. [3031] (IE) -> 8,958 Games
        //   3. [3033] (Mortal Reminder) -> 5,651 Games
        // Fifth Item:
        //   1. [3026] (Guardian Angel) -> 4,832 Games
        //   2. [3072] (Bloodthirster) -> 2,585 Games
        //   3. [6673] (Shieldbow) -> 2,549 Games
        // Sixth Item:
        //   1. [3026] (Guardian Angel) -> 2,035 Games (already picked as 5th item!)
        //   2. [3072] (Bloodthirster) -> 1,036 Games (must be picked as 6th item!)
        //   3. [3139] (Mercurial Scimitar) -> 638 Games
        let json = serde_json::json!({
            "data": {
                "damage_type": "AD",
                "win_rate": 50.0,
                "pick_rate": 11.9,
                "runes": {
                    "primary_style_id": 8000,
                    "sub_style_id": 8300,
                    "primary_perk_ids": [8008, 9111, 9104, 8017],
                    "sub_perk_ids": [8304, 8345],
                    "stat_shards": [5005, 5008, 5011]
                },
                "summoner_spells": [
                    { "ids": [4, 21], "play": 62087, "win": 30822 }
                ],
                "starter_items": [
                    { "ids": [1055, 2003], "play": 76942, "win": 38123 }
                ],
                "boots": [
                    { "ids": [3006], "play": 42413, "win": 22681 }
                ],
                "core_items": [
                    { "ids": [3032, 3085, 3031], "play": 15006, "win": 8883 },
                    { "ids": [6672, 3085, 3031], "play": 19057, "win": 10673 },
                    { "ids": [6672, 3085, 2523], "play": 5163, "win": 2880 }
                ],
                "fourth_items": [
                    { "ids": [3036], "play": 18151, "win": 11137 },
                    { "ids": [3031], "play": 8958, "win": 5236 },
                    { "ids": [3033], "play": 5651, "win": 3192 }
                ],
                "fifth_items": [
                    { "ids": [3026], "play": 4832, "win": 3108 },
                    { "ids": [3072], "play": 2585, "win": 1612 },
                    { "ids": [6673], "play": 2549, "win": 1570 }
                ],
                "sixth_items": [
                    { "ids": [3026], "play": 2035, "win": 1333 },
                    { "ids": [3072], "play": 1036, "win": 630 },
                    { "ids": [3139], "play": 638, "win": 401 }
                ],
                "last_items": [
                    { "ids": [3085], "play": 75928, "win": 39359 },
                    { "ids": [3031], "play": 59024, "win": 33741 },
                    { "ids": [6672], "play": 50399, "win": 24649 },
                    { "ids": [3032], "play": 34739, "win": 17553 },
                    { "ids": [3036], "play": 30559, "win": 17570 }
                ]
            }
        });

        let build = parse_opgg_analysis_payload(&json, 804, "Yunara").expect(
            "Parsing Yunara build with core_items and 4th/5th/6th item lists should succeed",
        );

        // Block 2 (Primeiros 3 Itens): must pick [6672, 3085, 3031] because 19,057 > 15,006
        assert_eq!(build.item_blocks[1].item_ids, vec![6672, 3085, 3031]);

        // Block 4 (Build Final): 3 Core ([6672, 3085, 3031]) + 4th (3036) + 5th (3026) + 6th (3072 since 3026 is already in 5th)
        assert_eq!(
            build.item_blocks[3].item_ids,
            vec![6672, 3085, 3031, 3036, 3026, 3072]
        );
    }
    #[test]
    fn test_boots_ranking_yunara_with_synchronized_souls() {
        // Mirrors OP.GG boots table for Yunara:
        // 1. 3008 (Synchronized Souls) -> 41,774 Games (53.06%)
        // 2. 3006 (Berserker's Greaves) -> 33,119 Games (42.07%)
        // 3. 3047 (Plated Steelcaps) -> 572 Games (0.73%)
        // 4. 3009 (Boots of Swiftness) -> 257 Games (0.33%)
        // 5. 3111 (Mercury's Treads) -> 252 Games (0.32%)
        let json = serde_json::json!({
            "data": {
                "damage_type": "AD",
                "win_rate": 50.0,
                "pick_rate": 11.9,
                "runes": {
                    "primary_style_id": 8000,
                    "sub_style_id": 8300,
                    "primary_perk_ids": [8008, 9111, 9104, 8017],
                    "sub_perk_ids": [8304, 8345],
                    "stat_shards": [5005, 5008, 5011]
                },
                "summoner_spells": [
                    { "ids": [4, 21], "play": 62087, "win": 30822 }
                ],
                "starter_items": [
                    { "ids": [1055, 2003], "play": 76942, "win": 38123 }
                ],
                "boots": [
                    { "ids": [3008], "play": 41774, "win": 22345, "pick_rate": 0.5306 },
                    { "ids": [3006], "play": 33119, "win": 15735, "pick_rate": 0.4207 },
                    { "ids": [3047], "play": 572, "win": 286, "pick_rate": 0.0073 },
                    { "ids": [3009], "play": 257, "win": 128, "pick_rate": 0.0033 },
                    { "ids": [3111], "play": 252, "win": 126, "pick_rate": 0.0032 }
                ],
                "core_items": [
                    { "ids": [6672, 3085, 3031], "play": 19057, "win": 10673 }
                ],
                "fourth_items": [
                    { "ids": [3036], "play": 18151, "win": 11137 }
                ],
                "fifth_items": [
                    { "ids": [3026], "play": 4832, "win": 3108 }
                ],
                "sixth_items": [
                    { "ids": [3072], "play": 1036, "win": 630 }
                ]
            }
        });

        let build = parse_opgg_analysis_payload(&json, 804, "Yunara")
            .expect("Parsing Yunara build with boots list should succeed");

        // Verify Block 3 (3 Botas Mais Feitas):
        assert_eq!(build.item_blocks[2].title, "3 Botas Mais Feitas");
        // Must be in strict order of play count descending: 3008 (41,774) -> 3006 (33,119) -> 3047 (572)
        assert_eq!(build.item_blocks[2].item_ids, vec![3008, 3006, 3047]);

        // Verify boots never leak into Core (Block 1), Final Build (Block 3), or Situational (Block 4)
        for &boot_id in &[3008, 3006, 3047, 3009, 3111] {
            assert!(
                !build.item_blocks[1].item_ids.contains(&boot_id),
                "Boot {boot_id} leaked into core"
            );
            assert!(
                !build.item_blocks[3].item_ids.contains(&boot_id),
                "Boot {boot_id} leaked into final build"
            );
            assert!(
                !build.item_blocks[4].item_ids.contains(&boot_id),
                "Boot {boot_id} leaked into situational"
            );
        }

        // Verify is_boot_item recognizes 3008, 3010, and 3013
        assert!(is_boot_item(3008));
        assert!(is_boot_item(3010));
        assert!(is_boot_item(3013));
    }
    #[test]
    fn test_starter_items_by_position_adc_top_mid_jungle_support() {
        // Standard ADC (Yunara): Doran's Blade (1055, 76k) > Doran's Shield (1054, 12k) + Potion (2003)
        let adc_starters = vec![
            ItemStatEntry {
                ids: vec![1055, 2003],
                play: 76942,
                win: 38123,
                pick_rate: 0.8,
            },
            ItemStatEntry {
                ids: vec![1054, 2003],
                play: 12000,
                win: 6000,
                pick_rate: 0.15,
            },
        ];
        let adc_blocks = build_5_item_blocks_with_depth_lists(
            adc_starters,
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            "AD",
            "adc",
        );
        assert_eq!(adc_blocks[0].title, "Itens Iniciais");
        assert_eq!(adc_blocks[0].item_ids, vec![1055, 2003]);

        // Standard Top (Tank): Doran's Shield (1054, 40k) > Doran's Blade (1055, 5k) + Potion (2003)
        let top_starters = vec![
            ItemStatEntry {
                ids: vec![1054, 2003],
                play: 40000,
                win: 20000,
                pick_rate: 0.7,
            },
            ItemStatEntry {
                ids: vec![1055, 2003],
                play: 5000,
                win: 2500,
                pick_rate: 0.1,
            },
        ];
        let top_blocks = build_5_item_blocks_with_depth_lists(
            top_starters,
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            "AD",
            "top",
        );
        assert_eq!(top_blocks[0].item_ids, vec![1054, 2003]);

        // Standard Mid (Mage): Doran's Ring (1056, 30k) > Dark Seal (1082, 5k) + Potion (2003)
        let mid_starters = vec![
            ItemStatEntry {
                ids: vec![1056, 2003],
                play: 30000,
                win: 15000,
                pick_rate: 0.8,
            },
            ItemStatEntry {
                ids: vec![1082, 2003],
                play: 5000,
                win: 2500,
                pick_rate: 0.1,
            },
        ];
        let mid_blocks = build_5_item_blocks_with_depth_lists(
            mid_starters,
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            "AP",
            "mid",
        );
        assert_eq!(mid_blocks[0].item_ids, vec![1056, 2003]);

        // Jungle: Mosstomper (1102, 50k) > Gustwalker (1103, 20k) > Scorchclaw (1101, 10k) + Potion (2003)
        let jng_starters = vec![
            ItemStatEntry {
                ids: vec![1102, 2003],
                play: 50000,
                win: 25000,
                pick_rate: 0.6,
            },
            ItemStatEntry {
                ids: vec![1103, 2003],
                play: 20000,
                win: 10000,
                pick_rate: 0.25,
            },
            ItemStatEntry {
                ids: vec![1101, 2003],
                play: 10000,
                win: 5000,
                pick_rate: 0.15,
            },
        ];
        let jng_blocks = build_5_item_blocks_with_depth_lists(
            jng_starters,
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            "AD",
            "jungle",
        );
        assert_eq!(jng_blocks[0].item_ids, vec![1102, 2003]);

        // Support: Quest item is pre-granted/bought, must ONLY contain Potion!
        let supp_starters = vec![ItemStatEntry {
            ids: vec![3865, 2003, 2003],
            play: 60000,
            win: 30000,
            pick_rate: 0.95,
        }];
        let supp_blocks = build_5_item_blocks_with_depth_lists(
            supp_starters,
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            "AP",
            "support",
        );
        assert_eq!(supp_blocks[0].item_ids, vec![2003]);
    }
    #[test]
    fn test_future_proof_dynamic_classification_and_opgg_context_trust() {
        // Simulate a new patch with completely un-hardcoded future item IDs:
        // Item 9901: brand new boot introduced by Riot (tags: ["Boots"])
        // Item 9902: brand new component item (has "into" recipe)
        // Item 9903: brand new completed legendary item (cost >= 1600, no "into")
        let ddragon_items_json = serde_json::json!({
            "data": {
                "9901": {
                    "name": "Astral Treads",
                    "tags": ["Boots"],
                    "gold": { "total": 1100 }
                },
                "9902": {
                    "name": "Arcane Filament",
                    "into": ["9903"],
                    "tags": ["Damage"],
                    "gold": { "total": 850 }
                },
                "9903": {
                    "name": "Celestial Singularity",
                    "tags": ["Damage"],
                    "gold": { "total": 3300 }
                }
            }
        });

        let cache = parse_ddragon_item_classifications(&ddragon_items_json);
        set_dynamic_item_classifications(cache);

        // Verify dynamic classifier recognized future items without any hardcoded IDs:
        assert!(is_boot_item_dynamic(9901));
        assert!(is_component_item_dynamic(9902));
        assert!(!is_component_item_dynamic(9903));

        // Test building blocks with these items in OP.GG payload:
        let boots = vec![ItemStatEntry {
            ids: vec![9901],
            play: 50000,
            win: 25000,
            pick_rate: 0.9,
        }];
        let candidate_entries = vec![
            ItemStatEntry {
                ids: vec![9903, 3089, 3157],
                play: 25000,
                win: 13000,
                pick_rate: 0.5,
            },
            ItemStatEntry {
                ids: vec![9902],
                play: 20000,
                win: 10000,
                pick_rate: 0.4,
            }, // Component - must be skipped!
            ItemStatEntry {
                ids: vec![3135],
                play: 15000,
                win: 8000,
                pick_rate: 0.3,
            },
            ItemStatEntry {
                ids: vec![3102],
                play: 12000,
                win: 6000,
                pick_rate: 0.25,
            },
            ItemStatEntry {
                ids: vec![4629],
                play: 10000,
                win: 5000,
                pick_rate: 0.2,
            },
            ItemStatEntry {
                ids: vec![4646],
                play: 8000,
                win: 4000,
                pick_rate: 0.15,
            },
        ];

        let blocks = build_5_item_blocks(
            Some(ItemStatEntry {
                ids: vec![1056, 2003],
                play: 50000,
                win: 25000,
                pick_rate: 0.9,
            }),
            boots,
            candidate_entries,
            "AP",
        );

        // 1. Boot block must contain the new boot 9901
        assert_eq!(blocks[2].title, "3 Botas Mais Feitas");
        assert!(blocks[2].item_ids.contains(&9901));

        // 2. Component 9902 must NEVER appear in Core, Final Build, or Situational blocks
        assert!(!blocks[1].item_ids.contains(&9902));
        assert!(!blocks[3].item_ids.contains(&9902));
        assert!(!blocks[4].item_ids.contains(&9902));

        // 3. New Legendary 9903 must be correctly integrated in Core and Final Build
        assert!(blocks[1].item_ids.contains(&9903));
        assert!(blocks[3].item_ids.contains(&9903));
    }
    #[test]
    fn test_extract_summoner_spell_choices_yunara_and_jungle() {
        // Yunara: Flash (4) + Barrier (21) (61,208) vs Exhaust (3) + Flash (4) (17,302)
        let yunara_json = serde_json::json!({
            "summoner_spells": [
                { "ids": [4, 21], "play": 61208, "win": 30365 },
                { "ids": [3, 4], "play": 17302, "win": 8780 }
            ]
        });
        let (top_spells, choices) = extract_summoner_spell_choices(&yunara_json, false);
        assert_eq!(top_spells, (4, 21));
        assert_eq!(choices, vec![21, 3]); // Option 1: Barrier (21), Option 2: Exhaust (3)

        // Jungle (Hecarim): Ghost (6) + Smite (11) (45,000) vs Flash (4) + Smite (11) (12,000)
        let hecarim_json = serde_json::json!({
            "summoner_spells": [
                { "ids": [6, 11], "play": 45000, "win": 23000 },
                { "ids": [4, 11], "play": 12000, "win": 6000 }
            ]
        });
        let (jng_spells, jng_choices) = extract_summoner_spell_choices(&hecarim_json, true);
        assert_eq!(jng_spells, (6, 11));
        assert_eq!(jng_choices, vec![6, 4]); // Option 1: Ghost (6), Option 2: Flash (4)

        // Jungle (Shaco): Ignite (14) + Smite (11) (38,000) vs Flash (4) + Smite (11) (5,000)
        let shaco_json = serde_json::json!({
            "summoner_spells": [
                { "ids": [14, 11], "play": 38000, "win": 20000 },
                { "ids": [4, 11], "play": 5000, "win": 2500 }
            ]
        });
        let (_, shaco_choices) = extract_summoner_spell_choices(&shaco_json, true);
        assert_eq!(shaco_choices, vec![14, 4]); // Option 1: Ignite (14), Option 2: Flash (4)
    }
    #[test]
    fn test_extract_top_rune_pages_multiple_sorted_by_play() {
        let json = serde_json::json!({
            "runes": [
                {
                    "primary_page_id": 8100,
                    "secondary_page_id": 8200,
                    "primary_rune_ids": [8112, 8126, 8138, 8135],
                    "secondary_rune_ids": [8226, 8210],
                    "stat_mod_ids": [5008, 5008, 5001],
                    "play": 12000,
                    "win": 6200
                },
                {
                    "primary_page_id": 8000,
                    "secondary_page_id": 8100,
                    "primary_rune_ids": [8005, 9111, 9104, 8014],
                    "secondary_rune_ids": [8139, 8135],
                    "stat_mod_ids": [5008, 5008, 5003],
                    "play": 45000,
                    "win": 23500
                },
                {
                    "primary_page_id": 8200,
                    "secondary_page_id": 8300,
                    "primary_rune_ids": [8214, 8226, 8210, 8236],
                    "secondary_rune_ids": [8304, 8345],
                    "stat_mod_ids": [5008, 5008, 5001],
                    "play": 8000,
                    "win": 4100
                },
                {
                    "primary_page_id": 8400,
                    "secondary_page_id": 8200,
                    "primary_rune_ids": [8437, 8401, 8473, 8484],
                    "secondary_rune_ids": [8226, 8210],
                    "stat_mod_ids": [5008, 5002, 5001],
                    "play": 2000,
                    "win": 950
                }
            ]
        });

        let pages = extract_top_rune_pages(&json);
        assert_eq!(pages.len(), 3);
        // 1st: 8000 (45,000 games)
        assert_eq!(pages[0].primary_style_id, 8000);
        assert_eq!(pages[0].play, 45000);
        // 2nd: 8100 (12,000 games)
        assert_eq!(pages[1].primary_style_id, 8100);
        assert_eq!(pages[1].play, 12000);
        // 3rd: 8200 (8,000 games)
        assert_eq!(pages[2].primary_style_id, 8200);
        assert_eq!(pages[2].play, 8000);
    }

    #[test]
    fn test_extract_top_core_and_final_build_options() {
        let core_entries = vec![
            ItemStatEntry {
                ids: vec![3118, 4645, 3089],
                play: 15000,
                win: 8000,
                pick_rate: 60.0,
            },
            ItemStatEntry {
                ids: vec![3118, 3157, 3089],
                play: 5000,
                win: 2600,
                pick_rate: 20.0,
            },
            ItemStatEntry {
                ids: vec![3118, 4645, 3135],
                play: 2000,
                win: 1050,
                pick_rate: 8.0,
            },
            ItemStatEntry {
                ids: vec![3118, 3165, 3089],
                play: 500,
                win: 250,
                pick_rate: 2.0,
            },
        ];
        let candidates = vec![3118, 4645, 3089, 3157, 3135, 3165, 4629, 3102];
        let core_opts = extract_top_core_build_options(&core_entries, &candidates, "AP");
        assert_eq!(core_opts.len(), 3);
        assert_eq!(core_opts[0].item_ids, vec![3118, 4645, 3089]);
        assert_eq!(core_opts[0].play, 15000);
        assert_eq!(core_opts[1].item_ids, vec![3118, 3157, 3089]);
        assert_eq!(core_opts[1].play, 5000);
        assert_eq!(core_opts[2].item_ids, vec![3118, 4645, 3135]);
        assert_eq!(core_opts[2].play, 2000);

        let fourth = vec![
            ItemStatEntry {
                ids: vec![3157],
                play: 10000,
                win: 5500,
                pick_rate: 40.0,
            },
            ItemStatEntry {
                ids: vec![3135],
                play: 3000,
                win: 1600,
                pick_rate: 15.0,
            },
        ];
        let fifth = vec![ItemStatEntry {
            ids: vec![3135],
            play: 8000,
            win: 4200,
            pick_rate: 35.0,
        }];
        let sixth = vec![
            ItemStatEntry {
                ids: vec![4629],
                play: 6000,
                win: 3100,
                pick_rate: 30.0,
            },
            ItemStatEntry {
                ids: vec![3102],
                play: 2000,
                win: 1000,
                pick_rate: 10.0,
            },
        ];

        let final_opts = build_3_final_build_options(
            &core_opts[0].item_ids,
            &fourth,
            &fifth,
            &sixth,
            &candidates,
            "AP",
        );
        assert_eq!(final_opts.len(), 3);
        for opt in &final_opts {
            assert_eq!(opt.item_ids.len(), 6);
            assert_eq!(&opt.item_ids[0..3], &[3118, 4645, 3089]);
        }
    }
    #[test]
    fn test_extract_all_mcp_runes_multiple_pages() {
        let text = "LolGetChampionAnalysis(Data(Summary(AverageStats(0.51,0.12)),CoreItems([3118,4645,3157]),CoreItems([3020]),CoreItems([1056,2003]),Runes(8100,[8112,8139,8140,8106],8200,[8210,8226],[5005,5008,5001]),Runes(8000,[8005,9111,9104,8014],8100,[8139,8135],[5008,5008,5001])))";
        let pages = extract_all_mcp_runes(text);
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].primary_style_id, 8100);
        assert_eq!(pages[1].primary_style_id, 8000);
    }

    #[test]
    fn test_normalize_tear_item_dynamic_and_static_fallbacks() {
        assert_eq!(normalize_tear_item_dynamic(3042), 3004); // Muramana -> Manamune
        assert_eq!(normalize_tear_item_dynamic(3040), 3003); // Seraph's Embrace -> Archangel's Staff
        assert_eq!(normalize_tear_item_dynamic(3121), 3119); // Fimbulwinter -> Winter's Approach
        assert_eq!(normalize_tear_item_dynamic(3089), 3089); // Untransformed Rabadon stays 3089
        assert_eq!(
            normalize_item_ids_dedup(&[3042, 3031, 3004]),
            vec![3004, 3031] // 3042 becomes 3004 and deduplicates with subsequent 3004
        );
    }

    #[test]
    fn test_parse_ddragon_item_classifications_handles_special_recipe_evolutions() {
        let ddragon_items_json = serde_json::json!({
            "data": {
                "9904": { "name": "Future Base Item", "gold": { "total": 2900 } },
                "9942": { "name": "Future Evolved Item", "inStore": false, "specialRecipe": 9904 }
            }
        });
        let cache = parse_ddragon_item_classifications(&ddragon_items_json);
        assert_eq!(cache.evolved_fallbacks.get(&9942), Some(&9904));
        set_dynamic_item_classifications(cache);
        assert_eq!(normalize_tear_item_dynamic(9942), 9904);
    }

    #[test]
    fn test_opgg_payload_normalizes_tear_items_in_blocks_and_options() {
        let analysis_json = serde_json::json!({
            "summary": { "position": "adc" },
            "core_items": [
                { "ids": [3078, 3042, 3036], "play": 15000, "win": 8000, "pick_rate": 45.0 }
            ],
            "boots": [
                { "ids": [3158], "play": 20000, "win": 11000, "pick_rate": 60.0 }
            ],
            "starters": [
                { "ids": [1055, 2003], "play": 25000, "win": 13000, "pick_rate": 80.0 }
            ],
            "fourth_items": [
                { "ids": [3072], "play": 8000, "win": 4500, "pick_rate": 30.0 }
            ],
            "fifth_items": [
                { "ids": [3026], "play": 6000, "win": 3200, "pick_rate": 25.0 }
            ],
            "sixth_items": [
                { "ids": [3139], "play": 4000, "win": 2100, "pick_rate": 20.0 }
            ],
            "last_items": [
                { "ids": [3040], "play": 3000, "win": 1600, "pick_rate": 15.0 }
            ],
            "runes": [
                {
                    "primary_page_id": 8000,
                    "primary_rune_ids": [8005, 9111, 9104, 8014],
                    "secondary_page_id": 8200,
                    "secondary_rune_ids": [8210, 8226],
                    "stat_mod_ids": [5005, 5008, 5001],
                    "play": 20000,
                    "win": 10500
                }
            ],
            "spells": [
                { "ids": [4, 7], "play": 20000, "win": 10500, "pick_rate": 80.0 }
            ]
        });

        let build = parse_opgg_analysis_payload(&analysis_json, 81, "Ezreal")
            .expect("Build parsing should succeed");

        // Core block must contain 3004 (Manamune), NOT 3042 (Muramana)
        assert_eq!(build.item_blocks[1].item_ids, vec![3078, 3004, 3036]);
        assert!(!build.item_blocks[1].item_ids.contains(&3042));

        // Final build must contain 3004, NOT 3042
        assert!(build.item_blocks[3].item_ids.contains(&3004));
        assert!(!build.item_blocks[3].item_ids.contains(&3042));

        // Core options must have 3004, NOT 3042
        assert_eq!(build.core_build_options[0].item_ids, vec![3078, 3004, 3036]);

        // Situational items with 3040 must normalize to 3003
        assert!(!build.item_blocks[4].item_ids.contains(&3040));
    }

    #[test]
    fn test_tear_item_merges_play_count_with_base_item() {
        let starter_entry = Some(ItemStatEntry {
            ids: vec![1055, 2003],
            play: 50000,
            win: 25000,
            pick_rate: 0.9,
        });
        let boots = vec![ItemStatEntry {
            ids: vec![3006],
            play: 50000,
            win: 25000,
            pick_rate: 0.9,
        }];
        let candidate_entries = vec![
            ItemStatEntry {
                ids: vec![3078, 3036, 3072],
                play: 25000,
                win: 13000,
                pick_rate: 0.5,
            },
            // Both 3004 and 3042 present:
            ItemStatEntry {
                ids: vec![3004],
                play: 5000,
                win: 2500,
                pick_rate: 0.1,
            },
            ItemStatEntry {
                ids: vec![3042],
                play: 15000,
                win: 8000,
                pick_rate: 0.3,
            },
            ItemStatEntry {
                ids: vec![3026],
                play: 4000,
                win: 2000,
                pick_rate: 0.08,
            },
        ];

        let blocks = build_5_item_blocks(starter_entry, boots, candidate_entries, "AD");
        // Check that 3004 appears in Final Build (promoted due to combined play 5k + 15k = 20k),
        // without duplicates and without 3042.
        let final_ids = &blocks[3].item_ids;
        assert!(!final_ids.contains(&3042));
        let count_3004 = final_ids.iter().filter(|&&id| id == 3004).count();
        assert_eq!(count_3004, 1);
        // And 3004 was placed before 3026 because 3004 combined play (20k) > 3026 play (4k)
        let pos_3004 = final_ids.iter().position(|&x| x == 3004).unwrap();
        let pos_3026 = final_ids.iter().position(|&x| x == 3026).unwrap();
        assert!(pos_3004 < pos_3026);
    }

    #[test]
    fn test_diadem_of_songs_normalizes_to_whispering_circlet() {
        assert_eq!(normalize_tear_item_dynamic(2530), 2526);
        assert_eq!(
            resolve_tear_item_ids(&[2530, 3001, 3070], 2526),
            vec![2526, 3001]
        );
    }

    #[test]
    fn test_seraphine_tear_in_core_promotes_to_whispering_circlet() {
        // Seraphine payload: core items has [6617, 3001, 3070] (Helia, Moonstone, Tear 3070)
        // And later entries has 2530 (Diadem of Songs) / 2526 (Whispering Circlet)
        let analysis_json = serde_json::json!({
            "summary": { "position": "support" },
            "core_items": [
                { "ids": [6617, 3001, 3070], "play": 18000, "win": 9500, "pick_rate": 55.0 }
            ],
            "boots": [
                { "ids": [3158], "play": 25000, "win": 13500, "pick_rate": 75.0 }
            ],
            "starters": [
                { "ids": [2003], "play": 28000, "win": 15000, "pick_rate": 90.0 }
            ],
            "fourth_items": [
                { "ids": [2530], "play": 12000, "win": 6800, "pick_rate": 45.0 }
            ],
            "fifth_items": [
                { "ids": [3107], "play": 8000, "win": 4300, "pick_rate": 30.0 }
            ],
            "sixth_items": [
                { "ids": [3504], "play": 5000, "win": 2800, "pick_rate": 20.0 }
            ],
            "runes": [
                {
                    "primary_page_id": 8200,
                    "primary_rune_ids": [8214, 8226, 8210, 8237],
                    "secondary_page_id": 8300,
                    "secondary_rune_ids": [8304, 8345],
                    "stat_mod_ids": [5008, 5008, 5001],
                    "play": 22000,
                    "win": 12000
                }
            ],
            "spells": [
                { "ids": [4, 7], "play": 25000, "win": 13500, "pick_rate": 85.0 }
            ]
        });

        let build = parse_opgg_analysis_payload(&analysis_json, 147, "Seraphine")
            .expect("Build parsing should succeed");

        // Core block must NOT contain 3070 (Tear); it must contain 2526 (Whispering Circlet)!
        assert_eq!(build.item_blocks[1].item_ids, vec![6617, 3001, 2526]);
        assert!(!build.item_blocks[1].item_ids.contains(&3070));
        assert!(!build.item_blocks[1].item_ids.contains(&2530));

        // Core options must also contain 2526, not 3070
        assert_eq!(build.core_build_options[0].item_ids, vec![6617, 3001, 2526]);

        // Final build must contain 2526 exactly once, without 3070 or 2530
        let final_ids = &build.item_blocks[3].item_ids;
        assert_eq!(final_ids.iter().filter(|&&id| id == 2526).count(), 1);
        assert!(!final_ids.contains(&3070));
        assert!(!final_ids.contains(&2530));
    }

    #[test]
    fn test_resolve_tear_to_completed_item_by_archetype() {
        let empty_entries: Vec<ItemStatEntry> = Vec::new();
        assert_eq!(
            resolve_tear_to_completed_item(&empty_entries, "support", "AP"),
            2526
        ); // Whispering Circlet
        assert_eq!(
            resolve_tear_to_completed_item(&empty_entries, "mid", "AP"),
            3003
        ); // Archangel's Staff
        assert_eq!(
            resolve_tear_to_completed_item(&empty_entries, "adc", "AD"),
            3004
        ); // Manamune
        assert_eq!(
            resolve_tear_to_completed_item(&empty_entries, "top", "TANK"),
            3119
        ); // Winter's Approach
    }
}

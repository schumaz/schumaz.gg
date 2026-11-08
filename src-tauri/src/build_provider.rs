//! Module 3: Data Fetching and Win Rate Provider (OP.GG + DataDragon)
//! Handles requests to DataDragon and OP.GG champion analysis (runes, builds, spells, win rates).

use crate::lcu_injector::BuildPriorityPreference;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::OnceCell;

pub const OPGG_MCP_ENDPOINT: &str = "https://mcp-api.op.gg/mcp";
pub const OPGG_REST_API_BASE: &str = "https://lol-api-champion.op.gg/api";
pub const DDRAGON_BASE_URL: &str = "https://ddragon.leagueoflegends.com";

static DDRAGON_CACHE: OnceCell<DataDragonCache> = OnceCell::const_new();

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

/// In-memory cache holding champion and rune metadata from DataDragon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataDragonCache {
    pub version: String,
    pub champions: HashMap<i32, ChampionMetadata>,
    pub runes: HashMap<i32, RuneMetadata>,
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
    let mut url = format!("{}/{}/champions/ranked?tier={}", OPGG_REST_API_BASE, region, tier);
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
        return Err(format!("OP.GG champions/ranked returned status: {}", res.status()));
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

    let total_secs_utc = total_days * 86400 + (hour as i64) * 3600 + (min as i64) * 60 + (sec as i64) - offset_secs;
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
        1054 | 1055 | 1056 | 1082 | 1083 | 2003 | 2031 | 2033 | 2055 | 2138 | 2139 | 2140 | 3865 | 3866 | 3867 | 3869 | 3870 | 3871 | 3876 | 3877
    )
}

pub fn is_boot_item(item_id: i32) -> bool {
    matches!(
        item_id,
        1001 // Boots
        | 3006 // Berserker's Greaves
        | 3009 // Boots of Swiftness
        | 3020 // Sorcerer's Shoes
        | 3047 // Plated Steelcaps
        | 3111 // Mercury's Treads
        | 3117 // Boots of Mobility
        | 3158 // Ionian Boots of Lucidity
        | 2422 // Slightly Magical Footwear
        | 3170 | 3171 | 3172 | 3173 | 3174 | 3175 // Tier 3 / Wild Rift / Arena boots
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
            "perk-images/StatMods/StatModsCDRIcon.png",
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
    let champions_url = format!("{}/cdn/{}/data/en_US/champion.json", base_url, version);
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
    let runes_url = format!("{}/cdn/{}/data/en_US/runesReforged.json", base_url, version);
    let runes_res: serde_json::Value = client
        .get(&runes_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch runesReforged.json: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse runesReforged.json: {}", e))?;

    let runes = parse_ddragon_runes(&runes_res)?;

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

/// Extracts all `CoreItems(...)` entries from OP.GG MCP text.
pub fn extract_all_core_items_from_text(text: &str) -> Vec<ItemStatEntry> {
    let mut entries = Vec::new();
    let mut search_slice = text;
    while let Some(ci_idx) = search_slice.find("CoreItems(") {
        let after_ci = &search_slice[ci_idx + 10..];
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

/// Builds the 5 item blocks conforming to the user's requirements:
/// 1. Itens Iniciais (Starter + Potion)
/// 2. Primeiros 3 Itens (Strictly non-boot, ordered according to BuildPriorityPreference)
/// 3. 3 Botas Mais Feitas (Always strictly ordered by most absolute games played: play descending)
/// 4. Build Final (clean_core + top 2 late-game items, ordered according to BuildPriorityPreference)
/// 5. Itens Situacionais / Úteis (Non-boot items not in core or final build, strictly ordered by play descending)
pub fn build_5_item_blocks(
    starter_entry: Option<ItemStatEntry>,
    boot_entries: Vec<ItemStatEntry>,
    candidate_entries: Vec<ItemStatEntry>,
    damage_type: &str,
    priority: BuildPriorityPreference,
) -> Vec<ItemBlock> {
    // 1. Itens Iniciais
    let mut starter_ids: Vec<i32> = starter_entry.map(|e| e.ids).unwrap_or_else(|| {
        if damage_type == "AP" {
            vec![1056, 2003]
        } else {
            vec![1055, 2003]
        }
    });
    if starter_ids.is_empty() {
        starter_ids = if damage_type == "AP" {
            vec![1056, 2003]
        } else {
            vec![1055, 2003]
        };
    } else if !starter_ids.contains(&2003)
        && !starter_ids.contains(&2031)
        && !starter_ids.contains(&2033)
    {
        starter_ids.push(2003);
    }

    // 2. 3 Botas Mais Feitas: Always ranked by absolute games played (play count descending)
    let mut boot_map: HashMap<i32, (i64, usize)> = HashMap::new();
    let mut boot_seen_counter = 0;
    for entry in &boot_entries {
        for &id in &entry.ids {
            if is_boot_item(id) {
                let slot = boot_map.entry(id).or_insert_with(|| {
                    let idx = boot_seen_counter;
                    boot_seen_counter += 1;
                    (0, idx)
                });
                slot.0 += entry.play.max(1);
            }
        }
    }
    for entry in &candidate_entries {
        for &id in &entry.ids {
            if is_boot_item(id) {
                let slot = boot_map.entry(id).or_insert_with(|| {
                    let idx = boot_seen_counter;
                    boot_seen_counter += 1;
                    (0, idx)
                });
                slot.0 += entry.play.max(1);
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

    // 3. Aggregate all non-boot, non-starter candidate items with play & win counts
    #[derive(Debug, Clone)]
    struct ItemAggregate {
        id: i32,
        play: i64,
        win: i64,
        first_seen_index: usize,
    }

    let mut item_map: HashMap<i32, (i64, i64, usize)> = HashMap::new();
    let mut item_seen_counter = 0;
    for entry in &candidate_entries {
        for &id in &entry.ids {
            if !is_boot_item(id) && !starter_ids.contains(&id) {
                let entry_play = entry.play.max(1);
                let entry_win = entry.win.max(0);
                let slot = item_map.entry(id).or_insert_with(|| {
                    let idx = item_seen_counter;
                    item_seen_counter += 1;
                    (0, 0, idx)
                });
                slot.0 += entry_play;
                slot.1 += entry_win;
            }
        }
    }

    let mut all_candidates: Vec<ItemAggregate> = item_map
        .into_iter()
        .map(|(id, (play, win, first_seen_index))| ItemAggregate {
            id,
            play,
            win,
            first_seen_index,
        })
        .collect();

    // Sort according to BuildPriorityPreference:
    match priority {
        BuildPriorityPreference::MostPopular => {
            all_candidates.sort_by(|a, b| {
                b.play
                    .cmp(&a.play)
                    .then_with(|| a.first_seen_index.cmp(&b.first_seen_index))
            });
        }
        BuildPriorityPreference::HighestWinRate => {
            all_candidates.sort_by(|a, b| {
                let wr_a = if a.play > 0 {
                    a.win as f64 / a.play as f64
                } else {
                    0.0
                };
                let wr_b = if b.play > 0 {
                    b.win as f64 / b.play as f64
                } else {
                    0.0
                };
                wr_b.partial_cmp(&wr_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| b.play.cmp(&a.play))
                    .then_with(|| a.first_seen_index.cmp(&b.first_seen_index))
            });
        }
    }

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
                .filter(|&&id| !is_boot_item(id) && !starter_ids.contains(&id))
                .count();
            non_boot_count >= 2
        })
        .collect();

    if !core_combos.is_empty() {
        match priority {
            BuildPriorityPreference::MostPopular => {
                core_combos.sort_by_key(|a| std::cmp::Reverse(a.play));
            }
            BuildPriorityPreference::HighestWinRate => {
                core_combos.sort_by(|a, b| {
                    b.win_rate()
                        .partial_cmp(&a.win_rate())
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| b.play.cmp(&a.play))
                });
            }
        }
        let best_combo = core_combos[0];
        for &id in &best_combo.ids {
            if clean_core.len() < 3
                && !is_boot_item(id)
                && !starter_ids.contains(&id)
                && !clean_core.contains(&id)
            {
                clean_core.push(id);
            }
        }
    }

    // Fill up to 3 core items from the top sorted candidates if needed
    for cand in &all_candidates {
        if clean_core.len() < 3 && !clean_core.contains(&cand.id) {
            clean_core.push(cand.id);
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

    // Block 4: Build Final (5 items)
    // The first 3 items MUST be identical to clean_core in the EXACT SAME sequential order,
    // followed by the top 2 late-game items.
    let mut final_build: Vec<i32> = clean_core.clone();
    for cand in &all_candidates {
        if final_build.len() < 5
            && !final_build.contains(&cand.id)
            && !is_component_item(cand.id)
            && !is_starter_item(cand.id)
            && !is_boot_item(cand.id)
        {
            final_build.push(cand.id);
        }
    }
    let default_final = if damage_type == "AP" {
        vec![3157, 3135, 4629]
    } else {
        vec![3072, 3026, 3139]
    };
    for d in default_final {
        if final_build.len() < 5 && !final_build.contains(&d) {
            final_build.push(d);
        }
    }

    // Block 5: Itens Situacionais / Úteis
    // Strictly non-boot items that are NOT in clean_core and NOT in final_build,
    // sorted strictly in descending order of absolute games played (play count)!
    let mut situational_candidates: Vec<&ItemAggregate> = all_candidates
        .iter()
        .filter(|cand| {
            !clean_core.contains(&cand.id)
                && !final_build.contains(&cand.id)
                && !is_component_item(cand.id)
                && !is_starter_item(cand.id)
                && !is_boot_item(cand.id)
        })
        .collect();
    situational_candidates.sort_by(|a, b| {
        b.play
            .cmp(&a.play)
            .then_with(|| a.first_seen_index.cmp(&b.first_seen_index))
    });

    let mut situational_items: Vec<i32> =
        situational_candidates.into_iter().map(|c| c.id).take(6).collect();

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
    parse_opgg_mcp_text_with_priority(
        text,
        champion_id,
        champion_name,
        BuildPriorityPreference::MostPopular,
    )
}

/// Parses text format returned in OP.GG MCP with user-configured BuildPriorityPreference.
pub fn parse_opgg_mcp_text_with_priority(
    text: &str,
    champion_id: i32,
    champion_name: &str,
    priority: BuildPriorityPreference,
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
                    win_rate = if w <= 1.0 { (w * 1000.0).round() / 10.0 } else { w };
                    pick_rate = if p <= 1.0 { (p * 1000.0).round() / 10.0 } else { p };
                } else if float_candidates.len() == 1 {
                    let w = float_candidates[0];
                    win_rate = if w <= 1.0 { (w * 1000.0).round() / 10.0 } else { w };
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
    let primary_style_id: i32 = after_runes[..comma1]
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
    let sub_style_id: i32 = after_primary_trim[..comma2]
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

    let starter_entry = item_candidate_entries.get(2).cloned();
    let boot_entries = item_candidate_entries.get(1).cloned().into_iter().collect();

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
    if item_candidate_entries.len() > 3 {
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

    let item_blocks = build_5_item_blocks(
        starter_entry,
        boot_entries,
        candidate_entries,
        damage_type,
        priority,
    );

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
        ..Default::default()
    })
}

/// Pure parser function to transform an OP.GG champion analysis response into `ChampionBuild`.
pub fn parse_opgg_analysis_payload(
    opgg_json: &serde_json::Value,
    champion_id: i32,
    champion_name: &str,
) -> Result<ChampionBuild, String> {
    parse_opgg_analysis_payload_with_priority(
        opgg_json,
        champion_id,
        champion_name,
        BuildPriorityPreference::MostPopular,
    )
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
                let win_rate = if play > 0 { win as f64 / play as f64 } else { 0.0 };
                if s1 > 0 && s2 > 0 {
                    entries.push((s1, s2, play, win_rate));
                }
            } else if let Some(ids) = item.as_array() && ids.len() >= 2 {
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

    if let Some(arr) = analysis.pointer("/summoner_spells/ids").and_then(|v| v.as_array())
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

/// Pure parser function to transform an OP.GG champion analysis response into `ChampionBuild` with user priority preference.
pub fn parse_opgg_analysis_payload_with_priority(
    opgg_json: &serde_json::Value,
    champion_id: i32,
    champion_name: &str,
    priority: BuildPriorityPreference,
) -> Result<ChampionBuild, String> {
    // If the response is wrapped in MCP content text (e.g. result.content[0].text), use text parser!
    if let Some(text) = opgg_json
        .pointer("/result/content/0/text")
        .and_then(|v| v.as_str())
    {
        return parse_opgg_mcp_text_with_priority(text, champion_id, champion_name, priority);
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
    let primary_style_id = analysis
        .pointer("/runes/primary_style_id")
        .or_else(|| analysis.pointer("/runes/primary_page_id"))
        .or_else(|| analysis.pointer("/runes/0/primary_page_id"))
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "Missing primary_style_id in OP.GG runes".to_string())?
        as i32;

    let sub_style_id = analysis
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

    // Extract summoner spells: selects the most played combination from OP.GG
    let (spell1_id, spell2_id) = extract_most_used_summoner_spells(analysis);

    let damage_type = analysis
        .get("damage_type")
        .and_then(|v| v.as_str())
        .unwrap_or("AP");

    // Extract item entries from JSON
    let starter_entry = parse_json_item_entries(
        analysis
            .pointer("/items/starters")
            .or_else(|| analysis.pointer("/starter_items")),
    )
    .into_iter()
    .next();

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
    let mut core_ids = Vec::new();
    for entry in &mut core_entries {
        if entry.play == 1 {
            entry.play = 10000;
        }
        core_ids.extend_from_slice(&entry.ids);
    }
    candidate_entries.extend(core_entries);

    let mut other_entries = Vec::new();
    other_entries.extend(parse_json_item_entries(
        analysis
            .pointer("/items/situational")
            .or_else(|| analysis.pointer("/last_items")),
    ));
    other_entries.extend(parse_json_item_entries(
        analysis
            .pointer("/items/final_build")
            .or_else(|| analysis.pointer("/fourth_items")),
    ));
    other_entries.extend(parse_json_item_entries(analysis.pointer("/fifth_items")));
    other_entries.extend(parse_json_item_entries(analysis.pointer("/sixth_items")));

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

    let item_blocks = build_5_item_blocks(
        starter_entry,
        boot_entries,
        candidate_entries,
        damage_type,
        priority,
    );

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
                    "data.runes.{primary_page_id,primary_rune_ids[],secondary_page_id,secondary_rune_ids[],stat_mod_ids[],play,win}",
                    "data.summoner_spells.{ids[],play,win}",
                    "data.starter_items.{ids[],play,win,pick_rate}",
                    "data.core_items.{ids[],play,win,pick_rate}",
                    "data.boots.{ids[],play,win,pick_rate}",
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
    let priority = crate::lcu_injector::get_user_preferences().build_priority;

    let mut build =
        match fetch_opgg_champion_analysis(client, endpoint_url, champ_name, queue_param).await {
            Ok(json_res) => {
                match parse_opgg_analysis_payload_with_priority(
                    &json_res,
                    champion_id,
                    champ_name,
                    priority,
                ) {
                    Ok(build) => build,
                    Err(_) => build_fallback_champion_build(champion_id, champ_name, queue_id),
                }
            }
            Err(_) => build_fallback_champion_build(champion_id, champ_name, queue_id),
        };

    enrich_build_with_ddragon(&mut build, ddragon_cache);
    build
}

/// Fetches available patch versions from the OP.GG REST API.
pub async fn fetch_opgg_versions(client: &reqwest::Client, region: &str) -> Vec<String> {
    let url = format!("{}/{}/champions/ranked/versions", OPGG_REST_API_BASE, region);
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
        return Err(format!("OP.GG REST responded with HTTP error: {}", res.status()));
    }

    let json_body: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse JSON response from OP.GG REST: {}", e))?;

    Ok(json_body)
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
    let priority = crate::lcu_injector::get_user_preferences().build_priority;

    // 1. Fetch available versions for region
    let available_versions = fetch_opgg_versions(client, target_region).await;

    // 2. Determine effective patch (2-day volume protection rule)
    let (mut effective_patch, mut patch_fallback_applied) = if let Some(p) = patch {
        if !p.trim().is_empty() {
            (p.to_string(), false)
        } else {
            (available_versions.first().cloned().unwrap_or_default(), false)
        }
    } else {
        (available_versions.first().cloned().unwrap_or_default(), false)
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

    // 3. Fetch build with effective patch
    let mut build = match fetch_opgg_rest_champion_analysis(
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
        Ok(json_res) => {
            match parse_opgg_analysis_payload_with_priority(
                &json_res,
                champion_id,
                champ_name,
                priority,
            ) {
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
            .and(path("/cdn/14.23.1/data/en_US/champion.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(champion_json))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/cdn/14.23.1/data/en_US/runesReforged.json"))
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
    fn test_build_priority_preference_changes_item_ordering() {
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

        // 1. With MostPopular (most games played):
        let popular_blocks = build_5_item_blocks(
            starter.clone(),
            boots.clone(),
            candidates.clone(),
            "AP",
            BuildPriorityPreference::MostPopular,
        );
        // Ordered by play count: 3118 (10k), 4645 (8k), 3157 (6k)
        assert_eq!(popular_blocks[1].item_ids, vec![3118, 4645, 3157]);

        // 2. With HighestWinRate:
        let winrate_blocks = build_5_item_blocks(
            starter,
            boots,
            candidates,
            "AP",
            BuildPriorityPreference::HighestWinRate,
        );
        // Ordered by win rate: 3089 (65%), 3157 (55%), 4645 (52.5%)
        assert_eq!(winrate_blocks[1].item_ids, vec![3089, 3157, 4645]);
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

        let blocks = build_5_item_blocks(
            starter,
            boots,
            candidates,
            "AP",
            BuildPriorityPreference::HighestWinRate, // Even if HighestWinRate is selected!
        );

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

        let blocks = build_5_item_blocks(
            starter,
            boots,
            candidates,
            "AP",
            BuildPriorityPreference::MostPopular,
        );

        // Clean core (first 3): 3118, 4645, 3157
        assert_eq!(blocks[1].item_ids, vec![3118, 4645, 3157]);
        // Final build (5 items): 3118, 4645, 3157, 3089, 3135
        assert_eq!(blocks[3].item_ids, vec![3118, 4645, 3157, 3089, 3135]);

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

        let (patch, fallback) =
            select_effective_patch(&versions, Some(created_at), now_unix, None);
        assert_eq!(patch, "16.21");
        assert!(fallback);
    }

    #[test]
    fn test_select_effective_patch_over_48h_keeps_latest() {
        let versions = vec!["16.22".to_string(), "16.21".to_string()];
        let created_at = "2026-11-05T12:00:00Z";
        let created_unix = parse_iso8601_to_unix(created_at).unwrap();
        let now_unix = created_unix + (3 * 86400); // 3 days later

        let (patch, fallback) =
            select_effective_patch(&versions, Some(created_at), now_unix, None);
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
            OpggPositionInfo { name: "TOP".to_string(), play: 100, role_rate: 0.1 },
            OpggPositionInfo { name: "ADC".to_string(), play: 9500, role_rate: 0.9 },
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
        ).await;

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
            ItemStatEntry { ids: vec![6672, 3085, 3031], play: 16000, win: 8000, pick_rate: 0.5 }, // Core
            ItemStatEntry { ids: vec![3032], play: 12000, win: 6000, pick_rate: 0.4 }, // Yun Tal (4th)
            ItemStatEntry { ids: vec![1038], play: 11000, win: 5500, pick_rate: 0.35 }, // B.F. Sword (component - MUST BE SKIPPED!)
            ItemStatEntry { ids: vec![3036], play: 10000, win: 5000, pick_rate: 0.3 }, // LDR (5th)
            ItemStatEntry { ids: vec![2523], play: 8000, win: 4000, pick_rate: 0.25 }, // Situational 1
            ItemStatEntry { ids: vec![2512], play: 7000, win: 3500, pick_rate: 0.2 }, // Situational 2
            ItemStatEntry { ids: vec![3033], play: 6000, win: 3000, pick_rate: 0.18 }, // Situational 3
            ItemStatEntry { ids: vec![3026], play: 5000, win: 2500, pick_rate: 0.15 }, // Situational 4
            ItemStatEntry { ids: vec![6673], play: 4000, win: 2000, pick_rate: 0.12 }, // Situational 5
            ItemStatEntry { ids: vec![3072], play: 3000, win: 1500, pick_rate: 0.1 }, // Situational 6
            ItemStatEntry { ids: vec![3139], play: 2000, win: 1000, pick_rate: 0.08 }, // Situational 7 (should be dropped by cap)
        ];

        let blocks = build_5_item_blocks(
            starter,
            boots,
            candidates,
            "AD",
            BuildPriorityPreference::MostPopular,
        );

        // Core items
        assert_eq!(blocks[1].item_ids, vec![6672, 3085, 3031]);

        // Build Final: must NEVER contain component 1038 (B.F. Sword)!
        // Should contain core (6672, 3085, 3031) followed by 3032 and 3036
        assert_eq!(blocks[3].item_ids, vec![6672, 3085, 3031, 3032, 3036]);

        // Situational items: max 6 items, must not contain 1038 or any core/final items
        assert_eq!(blocks[4].item_ids.len(), 6);
        assert!(!blocks[4].item_ids.contains(&1038));
        for &id in &blocks[4].item_ids {
            assert!(!blocks[1].item_ids.contains(&id));
            assert!(!blocks[3].item_ids.contains(&id));
        }
    }
}

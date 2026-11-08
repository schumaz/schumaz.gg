//! Module 3: Data Fetching and Win Rate Provider (OP.GG + DataDragon)
//! Handles requests to DataDragon and OP.GG champion analysis (runes, builds, spells, win rates).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::OnceCell;

pub const OPGG_MCP_ENDPOINT: &str = "https://mcp-api.op.gg/mcp";
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuneMetadata {
    pub id: i32,
    pub key: String,
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ItemBlock {
    pub title: String,
    pub item_ids: Vec<i32>,
}

/// Complete build recommendations fetched and parsed from OP.GG.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
            },
            ItemBlock {
                title: "Primeiros 3 Itens".to_string(),
                item_ids: vec![3031, 3006, 3094],
            },
            ItemBlock {
                title: "Build Final".to_string(),
                item_ids: vec![3031, 3006, 3094, 3036, 3026],
            },
            ItemBlock {
                title: "3 Botas Mais Feitas".to_string(),
                item_ids: vec![3006, 3047, 3111],
            },
        ],
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

/// Parses rune styles and individual perks from `runesReforged.json` into an O(1) lookup map.
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

/// Pure parser function to transform an OP.GG champion analysis response into `ChampionBuild`.
pub fn parse_opgg_analysis_payload(
    opgg_json: &serde_json::Value,
    champion_id: i32,
    champion_name: &str,
) -> Result<ChampionBuild, String> {
    let analysis = opgg_json
        .get("data")
        .or_else(|| opgg_json.pointer("/result/data"))
        .unwrap_or(opgg_json);

    let win_rate = analysis
        .get("win_rate")
        .or_else(|| analysis.pointer("/stats/win_rate"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let pick_rate = analysis
        .get("pick_rate")
        .or_else(|| analysis.pointer("/stats/pick_rate"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    // Extract rune styles and perks
    let primary_style_id = analysis
        .pointer("/runes/primary_style_id")
        .or_else(|| analysis.pointer("/runes/primary_page_id"))
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "Missing primary_style_id in OP.GG runes".to_string())?
        as i32;

    let sub_style_id = analysis
        .pointer("/runes/sub_style_id")
        .or_else(|| analysis.pointer("/runes/secondary_page_id"))
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "Missing sub_style_id in OP.GG runes".to_string())?
        as i32;

    let mut selected_perk_ids = Vec::with_capacity(9);

    if let Some(primary_perks) = analysis
        .pointer("/runes/primary_perk_ids")
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

    // Extract summoner spells
    let spell1_id = analysis
        .pointer("/spells/0")
        .or_else(|| analysis.pointer("/summoner_spells/0"))
        .and_then(|v| v.as_i64())
        .unwrap_or(4) as i32; // Default 4 = Flash

    let spell2_id = analysis
        .pointer("/spells/1")
        .or_else(|| analysis.pointer("/summoner_spells/1"))
        .and_then(|v| v.as_i64())
        .unwrap_or(14) as i32; // Default 14 = Ignite

    // Extract item blocks:
    // 1. Itens Iniciais (Starter + Potion)
    // 2. Primeiros 3 Itens (Core Build)
    // 3. Build Final (Full/Late-game build)
    // 4. 3 Botas Mais Feitas (Top 3 boots)
    let mut item_blocks = Vec::new();

    // 1. Starters
    let mut starter_ids: Vec<i32> = analysis
        .pointer("/items/starters")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_i64().map(|id| id as i32))
                .collect()
        })
        .unwrap_or_default();

    if starter_ids.is_empty() {
        starter_ids = vec![1056, 2003]; // Default Doran's Ring + Potion
    } else if !starter_ids.contains(&2003)
        && !starter_ids.contains(&2031)
        && !starter_ids.contains(&2033)
    {
        starter_ids.push(2003); // Ensure Health Potion is present
    }
    item_blocks.push(ItemBlock {
        title: "Itens Iniciais".to_string(),
        item_ids: starter_ids,
    });

    // 2. Primeiros 3 Itens (Core)
    let core_ids: Vec<i32> = analysis
        .pointer("/items/core")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_i64().map(|id| id as i32))
                .take(3)
                .collect()
        })
        .unwrap_or_else(|| vec![3285, 3020, 3089]);
    item_blocks.push(ItemBlock {
        title: "Primeiros 3 Itens".to_string(),
        item_ids: core_ids.clone(),
    });

    // 3. Build Final
    let mut final_build_ids: Vec<i32> = analysis
        .pointer("/items/final_build")
        .or_else(|| analysis.pointer("/items/full_build"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_i64().map(|id| id as i32))
                .collect()
        })
        .unwrap_or_default();

    if final_build_ids.is_empty() {
        final_build_ids = core_ids.clone();
        if let Some(situational) = analysis
            .pointer("/items/situational")
            .and_then(|v| v.as_array())
        {
            for id_val in situational
                .iter()
                .filter_map(|v| v.as_i64().map(|id| id as i32))
            {
                if !final_build_ids.contains(&id_val) && final_build_ids.len() < 6 {
                    final_build_ids.push(id_val);
                }
            }
        }
    }
    item_blocks.push(ItemBlock {
        title: "Build Final".to_string(),
        item_ids: final_build_ids,
    });

    // 4. 3 Botas Mais Feitas
    let mut boots_ids: Vec<i32> = analysis
        .pointer("/items/boots")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_i64().map(|id| id as i32))
                .take(3)
                .collect()
        })
        .unwrap_or_default();

    if boots_ids.is_empty() {
        boots_ids = vec![3020, 3111, 3047]; // Sorcerer's Shoes, Mercury's, Plated Steelcaps
    }
    item_blocks.push(ItemBlock {
        title: "3 Botas Mais Feitas".to_string(),
        item_ids: boots_ids,
    });

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
    })
}

/// Sends a request to the OP.GG MCP endpoint to execute `lol_get_champion_analysis`.
pub async fn fetch_opgg_champion_analysis(
    client: &reqwest::Client,
    endpoint_url: &str,
    champion_name: &str,
    queue_name: &str,
) -> Result<serde_json::Value, String> {
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "lol_get_champion_analysis",
            "arguments": {
                "champion_name": champion_name,
                "queue": queue_name,
                "desired_output_fields": "data.{stats,runes,spells,items}"
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

    match fetch_opgg_champion_analysis(client, endpoint_url, champ_name, queue_param).await {
        Ok(json_res) => match parse_opgg_analysis_payload(&json_res, champion_id, champ_name) {
            Ok(build) => build,
            Err(_) => build_fallback_champion_build(champion_id, champ_name, queue_id),
        },
        Err(_) => build_fallback_champion_build(champion_id, champ_name, queue_id),
    }
}

/// Production entry point to obtain recommended build for champion.
pub async fn get_recommended_build(
    client: &reqwest::Client,
    champion_id: i32,
    queue_id: i32,
) -> Result<ChampionBuild, String> {
    let cache = get_or_init_ddragon_cache(client).await?;
    Ok(
        get_recommended_build_with_endpoint(
            client,
            OPGG_MCP_ENDPOINT,
            champion_id,
            queue_id,
            cache,
        )
        .await,
    )
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
    }

    #[test]
    fn test_fallback_build_has_valid_9_perks_and_queue_spells() {
        let aram_fallback = build_fallback_champion_build(103, "Ahri", 450);
        assert_eq!(aram_fallback.selected_perk_ids.len(), 9);
        assert_eq!(aram_fallback.spell1_id, 4);
        assert_eq!(aram_fallback.spell2_id, 32);
        assert_eq!(aram_fallback.item_blocks.len(), 4);
        assert_eq!(aram_fallback.item_blocks[0].title, "Itens Iniciais");
        assert_eq!(aram_fallback.item_blocks[1].title, "Primeiros 3 Itens");
        assert_eq!(aram_fallback.item_blocks[2].title, "Build Final");
        assert_eq!(aram_fallback.item_blocks[3].title, "3 Botas Mais Feitas");

        let ranked_fallback = build_fallback_champion_build(103, "Ahri", 420);
        assert_eq!(ranked_fallback.selected_perk_ids.len(), 9);
        assert_eq!(ranked_fallback.spell1_id, 4);
        assert_eq!(ranked_fallback.spell2_id, 14);
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
        assert_eq!(build.item_blocks.len(), 4);
        assert_eq!(build.item_blocks[0].title, "Itens Iniciais");
        assert_eq!(build.item_blocks[1].title, "Primeiros 3 Itens");
        assert_eq!(build.item_blocks[1].item_ids, vec![3285, 3020, 3089]);
        assert_eq!(build.item_blocks[2].title, "Build Final");
        assert_eq!(build.item_blocks[3].title, "3 Botas Mais Feitas");
        assert_eq!(build.item_blocks[3].item_ids, vec![3020, 3111, 3047]);
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
}

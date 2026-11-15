use super::super::items::{TEAR_COMPLETED_ITEMS, is_boot_item};
use super::super::models::{
    ChampionMetadata, ItemClassificationCache, RuneMetadata, SpellMetadata,
};
use std::collections::{HashMap, HashSet};

/// Parses DataDragon `{locale}/summoner.json` into a lookup map keyed by numeric spell ID.
pub fn parse_ddragon_summoners(
    summoners_json: &serde_json::Value,
    version: &str,
) -> Result<HashMap<i32, SpellMetadata>, String> {
    let data_map = summoners_json
        .get("data")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "Missing 'data' object in DataDragon summoner.json".to_string())?;

    let mut map = HashMap::with_capacity(data_map.len());
    for (fallback_id, spell_data) in data_map {
        let key_str = spell_data.get("key").and_then(|k| k.as_str()).unwrap_or("");
        if let Ok(spell_id) = key_str.parse::<i32>() {
            let name = spell_data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(fallback_id);
            let image_full = spell_data
                .pointer("/image/full")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{}.png", fallback_id));

            map.insert(
                spell_id,
                SpellMetadata {
                    id: spell_id,
                    name: name.to_string(),
                    icon_url: format!(
                        "https://ddragon.leagueoflegends.com/cdn/{}/img/spell/{}",
                        version, image_full
                    ),
                },
            );
        }
    }

    Ok(map)
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
    let mut boots = HashSet::new();
    let mut components = HashSet::new();
    let mut completed = HashSet::new();
    let mut evolved_fallbacks = HashMap::new();
    let mut item_tags = HashMap::new();

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
                let string_tags: Vec<String> = tags.iter().map(|s| s.to_string()).collect();
                item_tags.insert(id, string_tags);

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

                let is_tear_completed = TEAR_COMPLETED_ITEMS.contains(&id);

                if tags.contains(&"Boots") {
                    boots.insert(id);
                }
                if has_into && !tags.contains(&"Boots") && !is_tear_completed {
                    components.insert(id);
                }
                if (!has_into || is_tear_completed) && gold >= 1600 && !tags.contains(&"Boots") {
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
        item_tags,
    }
}

/// Parses DataDragon `item.json` to identify completed legendary/mythic items (gold >= 1600, no 'into' recipes).
pub fn parse_ddragon_completed_items(items_json: &serde_json::Value) -> HashSet<i32> {
    let mut completed = HashSet::new();
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

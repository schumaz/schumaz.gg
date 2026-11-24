use super::super::models::{ChampionMetadata, RuneMetadata, SpellMetadata};
pub use super::item_parser::{parse_ddragon_completed_items, parse_ddragon_item_classifications};
use super::tooltip_parser::sanitize_ddragon_html;
use std::collections::HashMap;

fn parse_spell_cooldown_secs(spell_data: &serde_json::Value) -> Option<i32> {
    let raw = spell_data
        .get("cooldown")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_f64())
        .or_else(|| {
            spell_data
                .get("cooldownBurn")
                .and_then(|v| v.as_str())
                .and_then(|s| s.trim().parse::<f64>().ok())
        })?;
    if raw >= 1.0 {
        Some(raw.round() as i32)
    } else {
        None
    }
}

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
            let modes = spell_data
                .get("modes")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|m| m.as_str().map(str::trim))
                        .filter(|m| !m.is_empty())
                        .map(|m| m.to_ascii_uppercase())
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default();
            let description = spell_data
                .get("description")
                .and_then(|v| v.as_str())
                .map(sanitize_ddragon_html)
                .unwrap_or_default();
            let cooldown_secs = parse_spell_cooldown_secs(spell_data);

            map.insert(
                spell_id,
                SpellMetadata {
                    id: spell_id,
                    name: name.to_string(),
                    icon_url: format!(
                        "https://ddragon.leagueoflegends.com/cdn/{}/img/spell/{}",
                        version, image_full
                    ),
                    modes,
                    description,
                    cooldown_secs,
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
                    short_desc: String::new(),
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
                            let short_desc = rune
                                .get("shortDesc")
                                .and_then(|v| v.as_str())
                                .map(sanitize_ddragon_html)
                                .unwrap_or_default();

                            map.insert(
                                rune_id,
                                RuneMetadata {
                                    id: rune_id,
                                    key,
                                    name,
                                    icon_url,
                                    short_desc,
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

use super::super::models::{ChampionProfile, RuneTreeCatalog};
use std::collections::HashMap;

/// Parses DataDragon `champion.json` / `championFull.json` combat profiles (`tags`, `info`, `partype`)
/// keyed by numeric champion ID.
pub fn parse_all_ddragon_champion_profiles(
    ddragon_json: &serde_json::Value,
) -> HashMap<i32, ChampionProfile> {
    let Some(data_map) = ddragon_json.get("data").and_then(|v| v.as_object()) else {
        return HashMap::new();
    };

    let mut profiles = HashMap::with_capacity(data_map.len());
    for champ_data in data_map.values() {
        let Some(champ_id) = champ_data
            .get("key")
            .and_then(|k| k.as_str())
            .and_then(|s| s.trim().parse::<i32>().ok())
        else {
            continue;
        };

        let tags = champ_data
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t.as_str().map(str::trim))
                    .filter(|t| !t.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();

        let attack = champ_data
            .pointer("/info/attack")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let defense = champ_data
            .pointer("/info/defense")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let magic = champ_data
            .pointer("/info/magic")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let partype = champ_data
            .get("partype")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .unwrap_or("")
            .to_string();

        profiles.insert(
            champ_id,
            ChampionProfile {
                tags,
                attack,
                defense,
                magic,
                partype,
            },
        );
    }

    profiles
}

/// Parses DataDragon `runesReforged.json` into ordered tree slot catalogs for dynamic fallback selection.
pub fn parse_ddragon_rune_trees(runes_json: &serde_json::Value) -> Vec<RuneTreeCatalog> {
    let Some(trees) = runes_json.as_array() else {
        return Vec::new();
    };

    let mut catalog = Vec::with_capacity(trees.len());
    for tree in trees {
        let Some(id) = tree
            .get("id")
            .and_then(|v| v.as_i64())
            .map(|n| n as i32)
            .filter(|&id| id > 0)
        else {
            continue;
        };

        let key = tree
            .get("key")
            .and_then(|v| v.as_str())
            .or_else(|| tree.get("name").and_then(|v| v.as_str()))
            .map(str::trim)
            .unwrap_or("")
            .to_string();

        let slots = tree
            .get("slots")
            .and_then(|v| v.as_array())
            .map(|slot_arr| {
                slot_arr
                    .iter()
                    .map(|slot| {
                        slot.get("runes")
                            .and_then(|v| v.as_array())
                            .map(|runes| {
                                runes
                                    .iter()
                                    .filter_map(|r| r.get("id").and_then(|v| v.as_i64()))
                                    .map(|n| n as i32)
                                    .filter(|&perk_id| perk_id > 0)
                                    .collect::<Vec<i32>>()
                            })
                            .unwrap_or_default()
                    })
                    .filter(|ids| !ids.is_empty())
                    .collect::<Vec<Vec<i32>>>()
            })
            .unwrap_or_default();

        catalog.push(RuneTreeCatalog { id, key, slots });
    }

    catalog
}

use super::items::*;
use super::models::*;
use crate::lcu_injector::FlashKeyPreference;
use std::collections::HashMap;

pub const DDRAGON_BASE_URL: &str = "https://ddragon.leagueoflegends.com";
pub static DDRAGON_CACHE: tokio::sync::OnceCell<DataDragonCache> =
    tokio::sync::OnceCell::const_new();

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

/// Resolves summoner spell metadata and official Riot CDN icon (official PT-BR fallback).
pub fn get_spell_metadata(version: &str, spell_id: i32) -> SpellMetadata {
    let (name, icon_name) = match spell_id {
        4 => ("Flash", "SummonerFlash"),
        14 => ("Incendiar", "SummonerDot"),
        12 => ("Teleporte", "SummonerTeleport"),
        11 => ("Golpear", "SummonerSmite"),
        7 => ("Curar", "SummonerHeal"),
        21 => ("Barreira", "SummonerBarrier"),
        3 => ("Exaustão", "SummonerExhaust"),
        6 => ("Fantasma", "SummonerHaste"),
        1 => ("Purificar", "SummonerBoost"),
        13 => ("Clareza", "SummonerMana"),
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

/// Resolves summoner spell metadata preferring DataDragon `pt_BR/summoner.json` when available,
/// falling back to the built-in official PT-BR catalog.
pub fn resolve_spell_metadata(cache: &DataDragonCache, spell_id: i32) -> SpellMetadata {
    if let Some(meta) = cache.spells.get(&spell_id) {
        return meta.clone();
    }
    get_spell_metadata(&cache.version, spell_id)
}

/// Parses DataDragon `pt_BR/summoner.json` into a lookup map keyed by numeric spell ID.
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

/// Applies Flash key preference (D vs F) and Jungle Smite rule to a ChampionBuild's spell slots
/// and enriches `spells` and `spell_choices` using DataDragon cache.
pub fn apply_build_spells_with_preference(
    build: &mut ChampionBuild,
    cache: &DataDragonCache,
    flash_key: FlashKeyPreference,
) {
    let is_jungle = build.active_position.eq_ignore_ascii_case("jungle");
    let (s1, s2) = crate::lcu_injector::apply_jungle_smite_and_flash_preference(
        build.spell1_id,
        build.spell2_id,
        flash_key,
        is_jungle,
    );
    build.spell1_id = s1;
    build.spell2_id = s2;

    build.spells = vec![
        resolve_spell_metadata(cache, build.spell1_id),
        resolve_spell_metadata(cache, build.spell2_id),
    ];

    if build.spell_choices.is_empty() {
        let non_primary = if is_jungle {
            if build.spell1_id == 11 {
                build.spell2_id
            } else {
                build.spell1_id
            }
        } else if build.spell1_id == 4 {
            build.spell2_id
        } else {
            build.spell1_id
        };
        let second_alt = if is_jungle {
            if non_primary == 4 { 6 } else { 4 }
        } else if non_primary == 21 {
            3
        } else if non_primary == 7 {
            21
        } else if non_primary == 32 {
            7
        } else if non_primary == 14 {
            12
        } else {
            14
        };
        build.spell_choices = vec![
            resolve_spell_metadata(cache, non_primary),
            resolve_spell_metadata(cache, second_alt),
        ];
    } else {
        build.spell_choices = build
            .spell_choices
            .iter()
            .map(|s| resolve_spell_metadata(cache, s.id))
            .collect();
    }
}

/// Enriches a ChampionBuild with visual assets from DataDragon (avatars, icons, spells, items)
/// using an explicit `FlashKeyPreference`.
pub fn enrich_build_with_ddragon_with_flash_key(
    build: &mut ChampionBuild,
    cache: &DataDragonCache,
    flash_key: FlashKeyPreference,
) {
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

    apply_build_spells_with_preference(build, cache, flash_key);

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

    for opt in &mut build.build_options {
        opt.core_item_icon_urls = opt
            .core_item_ids
            .iter()
            .map(|id| {
                format!(
                    "https://ddragon.leagueoflegends.com/cdn/{}/img/item/{}.png",
                    cache.version, id
                )
            })
            .collect();
        opt.final_item_icon_urls = opt
            .final_item_ids
            .iter()
            .map(|id| {
                format!(
                    "https://ddragon.leagueoflegends.com/cdn/{}/img/item/{}.png",
                    cache.version, id
                )
            })
            .collect();
        opt.situational_item_icon_urls = opt
            .situational_item_ids
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

/// Enriches a ChampionBuild with visual assets from DataDragon (avatars, icons, spells, items).
pub fn enrich_build_with_ddragon(build: &mut ChampionBuild, cache: &DataDragonCache) {
    let prefs = crate::lcu_injector::get_user_preferences();
    enrich_build_with_ddragon_with_flash_key(build, cache, prefs.flash_key);
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
    let mut item_tags = std::collections::HashMap::new();

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

    // 2. Fetch champions, runes, items, and summoner spells concurrently in pt_BR
    let champions_url = format!("{}/cdn/{}/data/pt_BR/champion.json", base_url, version);
    let runes_url = format!("{}/cdn/{}/data/pt_BR/runesReforged.json", base_url, version);
    let items_url = format!("{}/cdn/{}/data/pt_BR/item.json", base_url, version);
    let summoners_url = format!("{}/cdn/{}/data/pt_BR/summoner.json", base_url, version);

    let champions_fut = async {
        client
            .get(&champions_url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch champion.json: {}", e))?
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("Failed to parse champion.json: {}", e))
    };

    let runes_fut = async {
        client
            .get(&runes_url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch runesReforged.json: {}", e))?
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("Failed to parse runesReforged.json: {}", e))
    };

    let items_fut = async {
        if let Ok(res) = client.get(&items_url).send().await
            && res.status().is_success()
            && let Ok(items_res) = res.json::<serde_json::Value>().await
        {
            let classification = parse_ddragon_item_classifications(&items_res);
            set_dynamic_item_classifications(classification);
        }
    };

    let summoners_fut = async {
        if let Ok(res) = client.get(&summoners_url).send().await
            && res.status().is_success()
            && let Ok(summoners_res) = res.json::<serde_json::Value>().await
            && let Ok(parsed_spells) = parse_ddragon_summoners(&summoners_res, &version)
        {
            return parsed_spells;
        }
        HashMap::new()
    };

    let (champions_res, runes_res, (), spells) =
        tokio::join!(champions_fut, runes_fut, items_fut, summoners_fut);

    let champions = parse_all_ddragon_champions(&champions_res?, &version)?;
    let runes = parse_ddragon_runes(&runes_res?)?;

    Ok(DataDragonCache {
        version,
        champions,
        runes,
        spells,
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

#[cfg(test)]
#[path = "ddragon_tests.rs"]
mod tests;

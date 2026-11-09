use super::items::*;
use super::models::*;
use std::collections::HashMap;

pub const DDRAGON_BASE_URL: &str = "https://ddragon.leagueoflegends.com";
pub static DDRAGON_CACHE: tokio::sync::OnceCell<DataDragonCache> = tokio::sync::OnceCell::const_new();

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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_provider::*;
    use crate::build_provider::items::TEST_CLASSIFICATION_MUTEX;

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
    fn test_parse_ddragon_item_classifications_handles_special_recipe_evolutions() {
        let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
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
}

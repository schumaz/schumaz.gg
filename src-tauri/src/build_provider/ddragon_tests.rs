use super::*;
use crate::build_provider::items::TEST_CLASSIFICATION_MUTEX;
use crate::build_provider::*;

#[test]
fn test_enrich_build_with_ddragon_populates_urls_spells_and_shards() {
    let mut ddragon_cache = DataDragonCache {
        version: "14.23.1".to_string(),
        champions: HashMap::new(),
        runes: HashMap::new(),
        spells: HashMap::new(),
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
            icon_url:
                "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/7200_Domination.png"
                    .to_string(),
        },
    );

    let mut build = build_fallback_champion_build(103, "Ahri", 420);
    enrich_build_with_ddragon_with_flash_key(&mut build, &ddragon_cache, FlashKeyPreference::D);

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
fn test_get_spell_metadata_uses_official_pt_br_names() {
    let ver = "16.22.1";
    assert_eq!(get_spell_metadata(ver, 32).name, "Marca");
    assert_eq!(get_spell_metadata(ver, 13).name, "Clareza");
    assert_eq!(get_spell_metadata(ver, 12).name, "Teleporte");
    assert_eq!(get_spell_metadata(ver, 7).name, "Curar");
    assert_eq!(get_spell_metadata(ver, 21).name, "Barreira");
    assert_eq!(get_spell_metadata(ver, 1).name, "Purificar");
    assert_eq!(get_spell_metadata(ver, 3).name, "Exaustão");
    assert_eq!(get_spell_metadata(ver, 6).name, "Fantasma");
    assert_eq!(get_spell_metadata(ver, 14).name, "Incendiar");
    assert_eq!(get_spell_metadata(ver, 11).name, "Golpear");
    assert_eq!(get_spell_metadata(ver, 4).name, "Flash");
    assert!(
        get_spell_metadata(ver, 32)
            .icon_url
            .ends_with("/16.22.1/img/spell/SummonerSnowball.png")
    );
    assert!(
        get_spell_metadata(ver, 13)
            .icon_url
            .ends_with("/16.22.1/img/spell/SummonerMana.png")
    );
}

#[test]
fn test_parse_ddragon_summoners_loads_pt_br_names_and_icons() {
    let summoners_json = serde_json::json!({
        "type": "summoner",
        "version": "16.22.1",
        "data": {
            "SummonerFlash": { "id": "SummonerFlash", "name": "Flash", "key": "4", "image": { "full": "SummonerFlash.png" } },
            "SummonerHeal": { "id": "SummonerHeal", "name": "Curar", "key": "7", "image": { "full": "SummonerHeal.png" } },
            "SummonerBarrier": { "id": "SummonerBarrier", "name": "Barreira", "key": "21", "image": { "full": "SummonerBarrier.png" } },
            "SummonerTeleport": { "id": "SummonerTeleport", "name": "Teleporte", "key": "12", "image": { "full": "SummonerTeleport.png" } },
            "SummonerMana": { "id": "SummonerMana", "name": "Clareza", "key": "13", "image": { "full": "SummonerMana.png" } },
            "SummonerSnowball": { "id": "SummonerSnowball", "name": "Marca", "key": "32", "image": { "full": "SummonerSnowball.png" } }
        }
    });

    let map = parse_ddragon_summoners(&summoners_json, "16.22.1").unwrap();
    assert_eq!(map.get(&4).unwrap().name, "Flash");
    assert_eq!(map.get(&7).unwrap().name, "Curar");
    assert_eq!(map.get(&21).unwrap().name, "Barreira");
    assert_eq!(map.get(&12).unwrap().name, "Teleporte");
    assert_eq!(map.get(&13).unwrap().name, "Clareza");
    assert_eq!(map.get(&32).unwrap().name, "Marca");
}

#[test]
fn test_enrich_build_with_ddragon_with_flash_key_applies_preference_in_all_modes() {
    let cache = DataDragonCache {
        version: "16.22.1".to_string(),
        champions: HashMap::new(),
        runes: HashMap::new(),
        spells: HashMap::new(),
    };

    // 1. ARAM Tristana: Flash (4) + Curar (7) primary, Barreira (21) secondary
    let mut tristana_aram = ChampionBuild {
        champion_id: 18,
        champion_name: "Tristana".to_string(),
        spell1_id: 7, // numerically ordered by OP.GG or reversed
        spell2_id: 4,
        spell_choices: vec![
            SpellMetadata {
                id: 7,
                name: String::new(),
                icon_url: String::new(),
            },
            SpellMetadata {
                id: 21,
                name: String::new(),
                icon_url: String::new(),
            },
        ],
        active_position: String::new(), // ARAM has empty position
        ..Default::default()
    };

    enrich_build_with_ddragon_with_flash_key(&mut tristana_aram, &cache, FlashKeyPreference::D);
    assert_eq!(tristana_aram.spell1_id, 4);
    assert_eq!(tristana_aram.spell2_id, 7);
    assert_eq!(tristana_aram.spells[0].name, "Flash");
    assert_eq!(tristana_aram.spells[1].name, "Curar");
    assert_eq!(tristana_aram.spell_choices[0].name, "Curar");
    assert_eq!(tristana_aram.spell_choices[1].name, "Barreira");

    enrich_build_with_ddragon_with_flash_key(&mut tristana_aram, &cache, FlashKeyPreference::F);
    assert_eq!(tristana_aram.spell1_id, 7);
    assert_eq!(tristana_aram.spell2_id, 4);
    assert_eq!(tristana_aram.spells[0].name, "Curar");
    assert_eq!(tristana_aram.spells[1].name, "Flash");

    // 2. ARAM with Marca (32) + Flash (4) in numeric/reversed order [32, 4]
    let mut ahri_aram = ChampionBuild {
        champion_id: 103,
        champion_name: "Ahri".to_string(),
        spell1_id: 32,
        spell2_id: 4,
        active_position: String::new(),
        ..Default::default()
    };
    enrich_build_with_ddragon_with_flash_key(&mut ahri_aram, &cache, FlashKeyPreference::D);
    assert_eq!(ahri_aram.spell1_id, 4);
    assert_eq!(ahri_aram.spell2_id, 32);
    assert_eq!(ahri_aram.spells[0].name, "Flash");
    assert_eq!(ahri_aram.spells[1].name, "Marca");

    // 3. ARAM Mayhem with numerically ordered [1, 4] (Purificar + Flash) and [3, 4] (Exaustão + Flash)
    let mut cleanse_mayhem = ChampionBuild {
        champion_id: 222,
        champion_name: "Jinx".to_string(),
        spell1_id: 1,
        spell2_id: 4,
        supports_runes: false,
        active_position: String::new(),
        ..Default::default()
    };
    enrich_build_with_ddragon_with_flash_key(&mut cleanse_mayhem, &cache, FlashKeyPreference::D);
    assert_eq!(cleanse_mayhem.spell1_id, 4);
    assert_eq!(cleanse_mayhem.spell2_id, 1);
    assert_eq!(cleanse_mayhem.spells[0].name, "Flash");
    assert_eq!(cleanse_mayhem.spells[1].name, "Purificar");

    // 4. Summoner's Rift non-jungle ("mid") also respects FlashKeyPreference
    let mut mid_sr = ChampionBuild {
        champion_id: 103,
        champion_name: "Ahri".to_string(),
        spell1_id: 4,
        spell2_id: 12,
        active_position: "mid".to_string(),
        ..Default::default()
    };
    enrich_build_with_ddragon_with_flash_key(&mut mid_sr, &cache, FlashKeyPreference::F);
    assert_eq!(mid_sr.spell1_id, 12);
    assert_eq!(mid_sr.spell2_id, 4);
    assert_eq!(mid_sr.spells[0].name, "Teleporte");
    assert_eq!(mid_sr.spells[1].name, "Flash");
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

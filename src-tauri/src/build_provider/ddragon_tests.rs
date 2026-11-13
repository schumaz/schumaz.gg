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

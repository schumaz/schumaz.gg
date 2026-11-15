use super::*;
use crate::build_provider::items::TEST_CLASSIFICATION_MUTEX;
use crate::build_provider::*;

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

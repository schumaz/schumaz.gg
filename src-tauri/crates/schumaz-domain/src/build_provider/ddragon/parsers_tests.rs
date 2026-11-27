use super::*;
use crate::build_provider::items::TEST_CLASSIFICATION_MUTEX;
use crate::build_provider::*;

#[test]
fn test_parse_ddragon_summoners_loads_pt_br_names_icons_and_modes() {
    let summoners_json = serde_json::json!({
        "type": "summoner",
        "version": "16.22.1",
        "data": {
            "SummonerFlash": { "id": "SummonerFlash", "name": "Flash", "key": "4", "modes": ["CLASSIC", "ARAM", "KIWI"], "image": { "full": "SummonerFlash.png" } },
            "SummonerExhaust": { "id": "SummonerExhaust", "name": "Exaustão", "key": "3", "modes": ["CLASSIC", "ARAM"], "image": { "full": "SummonerExhaust.png" } },
            "SummonerHeal": { "id": "SummonerHeal", "name": "Curar", "key": "7", "modes": ["CLASSIC", "ARAM", "KIWI"], "image": { "full": "SummonerHeal.png" } },
            "SummonerBarrier": { "id": "SummonerBarrier", "name": "Barreira", "key": "21", "modes": ["CLASSIC", "ARAM", "KIWI"], "image": { "full": "SummonerBarrier.png" } },
            "SummonerTeleport": { "id": "SummonerTeleport", "name": "Teleporte", "key": "12", "modes": ["CLASSIC"], "image": { "full": "SummonerTeleport.png" } },
            "SummonerMana": { "id": "SummonerMana", "name": "Clareza", "key": "13", "modes": ["ARAM", "KIWI"], "image": { "full": "SummonerMana.png" } },
            "SummonerSnowball": { "id": "SummonerSnowball", "name": "Marca", "key": "32", "modes": ["ARAM", "KIWI"], "image": { "full": "SummonerSnowball.png" } }
        }
    });

    let map = parse_ddragon_summoners(&summoners_json, "16.22.1").unwrap();
    assert_eq!(map.get(&4).unwrap().name, "Flash");
    assert_eq!(map.get(&3).unwrap().name, "Exaustão");
    assert_eq!(map.get(&7).unwrap().name, "Curar");
    assert_eq!(map.get(&21).unwrap().name, "Barreira");
    assert_eq!(map.get(&12).unwrap().name, "Teleporte");
    assert_eq!(map.get(&13).unwrap().name, "Clareza");
    assert_eq!(map.get(&32).unwrap().name, "Marca");

    assert_eq!(map.get(&3).unwrap().modes, vec!["CLASSIC", "ARAM"]);
    assert!(!map.get(&3).unwrap().modes.iter().any(|m| m == "KIWI"));
    assert_eq!(map.get(&12).unwrap().modes, vec!["CLASSIC"]);
    assert_eq!(map.get(&32).unwrap().modes, vec!["ARAM", "KIWI"]);
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

#[test]
fn test_parse_ddragon_item_classifications_supplements_offensive_tags_from_stats() {
    let items_json = serde_json::json!({
        "data": {
            "6610": {
                "name": "Sundered Sky",
                "gold": { "total": 3100 },
                "tags": ["Health"],
                "stats": { "FlatPhysicalDamageMod": 40.0 }
            },
            "4646": {
                "name": "Stormsurge",
                "gold": { "total": 2900 },
                "tags": [],
                "stats": { "FlatMagicDamageMod": 90.0 }
            }
        }
    });
    let cache = parse_ddragon_item_classifications(&items_json);
    assert!(cache.item_tags[&6610].iter().any(|t| t == "Damage"));
    assert!(cache.item_tags[&4646].iter().any(|t| t == "SpellDamage"));
}

#[test]
fn test_parse_all_ddragon_champion_abilities_extracts_qwer_and_sanitizes_html() {
    let champion_full = serde_json::json!({
        "data": {
            "Ahri": {
                "id": "Ahri",
                "key": "103",
                "name": "Ahri",
                "image": { "full": "Ahri.png" },
                "spells": [
                    {
                        "name": "Orb of Deception",
                        "description": "Ahri sends out and pulls back her orb, dealing <magicDamage>magic damage</magicDamage>.",
                        "cooldownBurn": "7",
                        "maxrank": 5,
                        "image": { "full": "AhriQ.png" }
                    },
                    {
                        "name": "Fox-Fire",
                        "description": "Ahri gains Move Speed and releases three fox-fires.",
                        "cooldownBurn": "9/8/7/6/5",
                        "maxrank": 5,
                        "image": { "full": "AhriW.png" }
                    },
                    {
                        "name": "Charm",
                        "description": "Ahri blows a kiss that charms an enemy.",
                        "cooldownBurn": "12",
                        "maxrank": 5,
                        "image": { "full": "AhriE.png" }
                    },
                    {
                        "name": "Spirit Rush",
                        "description": "Ahri dashes forward and fires essence bolts.",
                        "cooldownBurn": "130/115/100",
                        "maxrank": 3,
                        "image": { "full": "AhriR.png" }
                    }
                ]
            }
        }
    });

    let map = parse_all_ddragon_champion_abilities(&champion_full, "16.22.1");
    let ahri = map.get(&103).expect("Ahri abilities should be parsed");
    assert_eq!(ahri.len(), 4);
    assert_eq!(ahri[0].key, "Q");
    assert_eq!(ahri[0].name, "Orb of Deception");
    assert_eq!(
        ahri[0].icon_url,
        "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/spell/AhriQ.png"
    );
    assert_eq!(
        ahri[0].description,
        "Ahri sends out and pulls back her orb, dealing magic damage."
    );
    assert_eq!(ahri[0].cooldown_burn, "7");
    assert_eq!(ahri[0].max_rank, 5);
    assert_eq!(ahri[1].cooldown_burn, "9/8/7/6/5");
    assert_eq!(ahri[3].key, "R");
    assert_eq!(ahri[3].max_rank, 3);
}

#[test]
fn test_parse_ddragon_catalog_profiles_rune_trees_and_map_item_sets() {
    let champions_json = serde_json::json!({
        "data": {
            "Ahri": {
                "key": "103",
                "tags": ["Mage", "Assassin"],
                "partype": "Mana",
                "info": { "attack": 3, "defense": 4, "magic": 8 }
            }
        }
    });
    let profiles = parse_all_ddragon_champion_profiles(&champions_json);
    let ahri = profiles.get(&103).expect("Ahri profile should be parsed");
    assert_eq!(ahri.tags, vec!["Mage", "Assassin"]);
    assert_eq!((ahri.attack, ahri.defense, ahri.magic), (3, 4, 8));
    assert_eq!(ahri.partype, "Mana");

    let runes_json = serde_json::json!([
        {
            "id": 8200,
            "key": "Sorcery",
            "slots": [
                { "runes": [{ "id": 8229 }, { "id": 8214 }] },
                { "runes": [{ "id": 8226 }] }
            ]
        }
    ]);
    let trees = parse_ddragon_rune_trees(&runes_json);
    assert_eq!(trees.len(), 1);
    assert_eq!((trees[0].id, trees[0].key.as_str()), (8200, "Sorcery"));
    assert_eq!(trees[0].slots, vec![vec![8229, 8214], vec![8226]]);

    let items_json = serde_json::json!({
        "data": {
            "1056": { "tags": ["SpellDamage", "Lane"], "gold": { "total": 400, "purchasable": true }, "maps": { "11": true, "12": false } },
            "3112": { "tags": ["SpellDamage"], "gold": { "total": 950, "purchasable": true }, "maps": { "11": false, "12": true } },
            "2003": { "tags": ["Consumable", "HealthRegen"], "gold": { "total": 50, "purchasable": true }, "maps": { "11": true, "12": true } },
            "3340": { "tags": ["Trinket"], "gold": { "total": 0, "purchasable": true }, "maps": { "11": true, "12": true } },
            "9001": { "inStore": false, "tags": ["SpellDamage"], "gold": { "total": 3000, "purchasable": true }, "maps": { "11": true, "12": true } },
            "9002": { "requiredChampion": "Viktor", "tags": ["SpellDamage"], "gold": { "total": 3000, "purchasable": true }, "maps": { "11": true, "12": true } }
        }
    });
    let cls = parse_ddragon_item_classifications(&items_json);
    assert!(cls.map11_items.contains(&1056) && !cls.map12_items.contains(&1056));
    assert!(cls.map12_items.contains(&3112) && !cls.map11_items.contains(&3112));
    assert!(cls.starters.contains(&1056) && cls.starters.contains(&3112));
    assert!(cls.consumables.contains(&2003));
    assert_eq!(cls.item_gold.get(&1056).copied(), Some(400));
    assert!(!cls.map11_items.contains(&3340) && !cls.map11_items.contains(&9001) && !cls.map11_items.contains(&9002));
}

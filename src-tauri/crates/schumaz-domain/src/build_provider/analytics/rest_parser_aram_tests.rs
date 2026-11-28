use super::*;
use super::super::rest_extractors::infer_rest_damage_type;
use crate::build_provider::items::{TEST_CLASSIFICATION_MUTEX, set_dynamic_item_classifications};
use crate::build_provider::models::{ItemClassificationCache, ItemStatEntry};

#[test]
fn test_real_opgg_aram_tristana_payload_blocks_options_and_spells() {
    // Real OP.GG ARAM REST payload for Tristana (patch 16.22, emerald_plus, no `damage_type`)
    // enriched with MCP `fourth_items` and `fifth_items` (MCP omits `sixth_items` in ARAM).
    let rest_json = serde_json::json!({
        "data": {
            "summary": {
                "id": 18,
                "average_stats": { "play": 3627, "win_rate": 0.542046, "pick_rate": 0.091103 },
                "positions": null,
                "roles": [
                    { "name": "MARKSMAN", "stats": { "win_rate": 0.536839, "role_rate": 0.981104, "play": 3271, "win": 1756 } }
                ]
            },
            "summoner_spells": [
                { "ids": [4, 7], "win": 599, "play": 1125, "pick_rate": 0.3358 },
                { "ids": [4, 21], "win": 404, "play": 695, "pick_rate": 0.2075 },
                { "ids": [4, 6], "win": 370, "play": 678, "pick_rate": 0.2024 },
                { "ids": [4, 32], "win": 172, "play": 330, "pick_rate": 0.0985 },
                { "ids": [1, 4], "win": 126, "play": 236, "pick_rate": 0.0704 }
            ],
            "starter_items": [
                { "ids": [1038, 2003, 2003], "play": 1397, "win": 745, "pick_rate": 0.415 },
                { "ids": [1038], "play": 398, "win": 222, "pick_rate": 0.1182 },
                { "ids": [1042, 3006], "play": 251, "win": 124, "pick_rate": 0.0746 },
                { "ids": [2031, 3051], "play": 121, "win": 69, "pick_rate": 0.0359 },
                { "ids": [1001, 2003, 2003, 3134], "play": 67, "win": 36, "pick_rate": 0.0199 },
                { "ids": [1042, 2003, 3006], "play": 66, "win": 34, "pick_rate": 0.0196 }
            ],
            "boots": [
                { "ids": [3006], "play": 2640, "win": 1415, "pick_rate": 0.8585 },
                { "ids": [3008], "play": 264, "win": 166, "pick_rate": 0.0859 },
                { "ids": [3111], "play": 124, "win": 62, "pick_rate": 0.0403 },
                { "ids": [3047], "play": 42, "win": 26, "pick_rate": 0.0137 },
                { "ids": [3009], "play": 4, "win": 3, "pick_rate": 0.0013 }
            ],
            "core_items": [
                { "ids": [3032, 6675, 3031], "play": 598, "win": 329, "pick_rate": 0.2025 },
                { "ids": [3032, 3031, 6675], "play": 292, "win": 179, "pick_rate": 0.0989 },
                { "ids": [3032, 3031, 3036], "play": 210, "win": 120, "pick_rate": 0.0711 },
                { "ids": [3032, 6675, 3036], "play": 167, "win": 90, "pick_rate": 0.0566 },
                { "ids": [3032, 6676, 3031], "play": 113, "win": 65, "pick_rate": 0.0383 },
                { "ids": [6676, 6675, 3031], "play": 95, "win": 59, "pick_rate": 0.0322 },
                { "ids": [3032, 6676, 3036], "play": 52, "win": 28, "pick_rate": 0.0176 },
                { "ids": [6676, 3031, 3036], "play": 46, "win": 26, "pick_rate": 0.0156 },
                { "ids": [6676, 3031, 6675], "play": 45, "win": 26, "pick_rate": 0.0152 },
                { "ids": [3032, 3031, 3033], "play": 41, "win": 25, "pick_rate": 0.0139 },
                { "ids": [3032, 6676, 6675], "play": 39, "win": 19, "pick_rate": 0.0132 },
                { "ids": [6672, 6675, 3031], "play": 37, "win": 26, "pick_rate": 0.0125 },
                { "ids": [3032, 6675, 3033], "play": 37, "win": 17, "pick_rate": 0.0125 },
                { "ids": [3032, 3031, 3072], "play": 33, "win": 21, "pick_rate": 0.0112 },
                { "ids": [6676, 6675, 3036], "play": 30, "win": 14, "pick_rate": 0.0102 }
            ],
            "fourth_items": [
                { "ids": [3036], "play": 593, "win": 346, "pick_rate": 0.29 },
                { "ids": [3072], "play": 360, "win": 200, "pick_rate": 0.17 },
                { "ids": [3031], "play": 283, "win": 159, "pick_rate": 0.14 }
            ],
            "fifth_items": [
                { "ids": [3072], "play": 255, "win": 160, "pick_rate": 0.29 },
                { "ids": [3153], "play": 138, "win": 87, "pick_rate": 0.16 },
                { "ids": [3036], "play": 95, "win": 59, "pick_rate": 0.11 }
            ],
            "last_items": [
                { "ids": [3031], "play": 2613, "win": 1438, "pick_rate": 0.7496 },
                { "ids": [3032], "play": 2448, "win": 1317, "pick_rate": 0.7022 },
                { "ids": [6675], "play": 2056, "win": 1124, "pick_rate": 0.5898 },
                { "ids": [3036], "play": 1439, "win": 822, "pick_rate": 0.4128 },
                { "ids": [6676], "play": 1007, "win": 551, "pick_rate": 0.2889 },
                { "ids": [3072], "play": 848, "win": 500, "pick_rate": 0.2433 },
                { "ids": [3153], "play": 528, "win": 322, "pick_rate": 0.1515 },
                { "ids": [6672], "play": 429, "win": 239, "pick_rate": 0.1231 },
                { "ids": [1038], "play": 408, "win": 224, "pick_rate": 0.117 },
                { "ids": [3094], "play": 274, "win": 136, "pick_rate": 0.0786 },
                { "ids": [3033], "play": 245, "win": 125, "pick_rate": 0.0703 },
                { "ids": [2523], "play": 132, "win": 72, "pick_rate": 0.0379 },
                { "ids": [3046], "play": 113, "win": 65, "pick_rate": 0.0324 },
                { "ids": [3085], "play": 110, "win": 63, "pick_rate": 0.0316 },
                { "ids": [6673], "play": 91, "win": 46, "pick_rate": 0.0261 },
                { "ids": [3139], "play": 88, "win": 52, "pick_rate": 0.0252 }
            ],
            "runes": [
                {
                    "id": 8008,
                    "primary_page_id": 8000,
                    "primary_rune_ids": [8008, 9111, 9104, 8014],
                    "secondary_page_id": 8100,
                    "secondary_rune_ids": [8135, 8139],
                    "stat_mod_ids": [5005, 5008, 5001],
                    "play": 1882,
                    "win": 1010,
                    "pick_rate": 0.5402
                },
                {
                    "id": 9923,
                    "primary_page_id": 8100,
                    "primary_rune_ids": [9923, 8139, 8140, 8135],
                    "secondary_page_id": 8000,
                    "secondary_rune_ids": [9104, 9111],
                    "stat_mod_ids": [5005, 5008, 5001],
                    "play": 359,
                    "win": 196,
                    "pick_rate": 0.103
                }
            ]
        }
    });

    let aram_build = parse_opgg_analysis_payload(&rest_json, 18, "Tristana")
        .expect("Real Tristana ARAM payload should parse");

    // Summoner spells: Flash + Heal (4, 7) and top pair options [(4, 7), (4, 21)]
    assert_eq!(aram_build.spell1_id, 4);
    assert_eq!(aram_build.spell2_id, 7);
    assert_eq!(
        aram_build
            .spell_options
            .iter()
            .map(|o| (o.spell1_id, o.spell2_id))
            .collect::<Vec<_>>(),
        vec![(4, 7), (4, 21)]
    );
    assert_eq!(aram_build.spell_options[0].pick_rate, 33.6);
    assert_eq!(aram_build.spell_options[1].pick_rate, 20.8);

    // 5 Item Blocks aligned with OP.GG:
    assert_eq!(aram_build.item_blocks.len(), 5);
    assert_eq!(aram_build.item_blocks[0].item_ids, vec![1038, 2003]);
    assert_eq!(aram_build.item_blocks[1].item_ids, vec![3032, 6675, 3031]);
    assert_eq!(aram_build.item_blocks[2].item_ids, vec![3006, 3008, 3111]);
    assert_eq!(
        aram_build.item_blocks[3].item_ids,
        vec![3032, 6675, 3031, 3036, 3072, 6676]
    );
    assert_eq!(
        aram_build.item_blocks[4].item_ids,
        vec![3153, 6672, 3033, 3094, 2523, 3046]
    );
    assert!(!aram_build.item_blocks[4].item_ids.contains(&1038));

    // Unified build options aligned with Core + Final (MCP + last_items) + Situational:
    assert_eq!(aram_build.build_options.len(), 3);
    assert_eq!(
        aram_build.build_options[0].core_item_ids,
        vec![3032, 6675, 3031]
    );
    assert_eq!(
        aram_build.build_options[0].final_item_ids,
        vec![3032, 6675, 3031, 3036, 3072, 6676]
    );
    assert_eq!(
        aram_build.build_options[0].situational_item_ids,
        vec![3153, 6672, 3033, 3094, 2523, 3046]
    );
    assert_eq!(
        aram_build.build_options[1].core_item_ids,
        vec![3032, 3031, 6675]
    );
    assert_eq!(
        aram_build.build_options[2].core_item_ids,
        vec![3032, 3031, 3036]
    );

    // Same real payload parsed with `supports_runes = false` (ARAM Mayhem)
    let mayhem_build = parse_opgg_analysis_payload_with_options(&rest_json, 18, "Tristana", false)
        .expect("Real Tristana ARAM Mayhem payload should parse");
    assert!(!mayhem_build.supports_runes);
    assert_eq!(mayhem_build.item_blocks, aram_build.item_blocks);
    assert_eq!(mayhem_build.build_options, aram_build.build_options);
}

#[test]
fn test_irelia_aram_mayhem_does_not_inflate_play_1_troll_ap_build() {
    // Regression for real cached Irelia 16.23 payload where `play == 1` AP troll build
    // `[6655, 3118, 4646]` used to be inflated to `play = 9992`, overtaking `[3153, 6610, 6333]` (play = 14).
    let irelia_json = serde_json::json!({
        "data": {
            "summary": {
                "id": 39,
                "average_stats": { "play": 79, "win_rate": 0.481, "pick_rate": 0.032 },
                "roles": [{ "name": "FIGHTER", "stats": { "play": 79, "win": 38 } }]
            },
            "summoner_spells": [{ "ids": [4, 32], "play": 55, "win": 27 }],
            "starter_items": [{ "ids": [3184, 2003], "play": 48, "win": 24 }],
            "boots": [{ "ids": [3111], "play": 36, "win": 18 }],
            "core_items": [
                { "ids": [3153, 6610, 6333], "play": 14, "win": 9, "pick_rate": 0.177 },
                { "ids": [3153, 3078, 6333], "play": 6, "win": 4, "pick_rate": 0.076 },
                { "ids": [6655, 3118, 4646], "play": 1, "win": 1, "pick_rate": 0.013 }
            ],
            "last_items": [
                { "ids": [3153], "play": 64, "win": 33 },
                { "ids": [6610], "play": 40, "win": 21 },
                { "ids": [6333], "play": 34, "win": 18 },
                { "ids": [3053], "play": 22, "win": 11 }
            ],
            "runes": []
        }
    });

    let build = parse_opgg_analysis_payload_with_options(&irelia_json, 39, "Irelia", false)
        .expect("Irelia ARAM Mayhem payload should parse");

    assert_eq!(build.item_blocks[1].item_ids, vec![3153, 6610, 6333]);
    assert_eq!(build.build_options.len(), 2);
    assert_eq!(build.build_options[0].core_item_ids, vec![3153, 6610, 6333]);
    assert_eq!(build.build_options[0].play, 14);
    assert_eq!(build.build_options[1].core_item_ids, vec![3153, 3078, 6333]);
    assert_eq!(build.build_options[1].play, 6);
    assert!(!build.item_blocks[3].item_ids.contains(&6655));
    assert!(!build.item_blocks[4].item_ids.contains(&6655));
}

#[test]
fn test_empty_core_and_last_items_returns_err_even_without_runes() {
    let empty_mayhem = serde_json::json!({
        "data": {
            "summary": {
                "id": 39,
                "average_stats": { "play": 0, "win_rate": 0.0, "pick_rate": 0.0 },
                "roles": [{ "name": "FIGHTER", "stats": { "play": 0, "win": 0 } }]
            },
            "core_items": [],
            "last_items": [],
            "runes": []
        }
    });

    let err = parse_opgg_analysis_payload_with_options(&empty_mayhem, 39, "Irelia", false)
        .expect_err("Empty core_items and last_items must error even when supports_runes == false");
    assert!(err.contains("core_items"));
}

#[test]
fn test_infer_rest_damage_type_uses_aggregated_tags_and_fighter_assassin_fallback() {
    let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
    let mut cache = ItemClassificationCache::default();
    cache
        .item_tags
        .insert(3153, vec!["Damage".to_string(), "AttackSpeed".to_string()]);
    cache
        .item_tags
        .insert(6655, vec!["SpellDamage".to_string(), "Mana".to_string()]);
    set_dynamic_item_classifications(cache);

    // 1. Aggregated `last_items` AD tags resolve "AD" even when `core_items` is empty
    let fighter_analysis = serde_json::json!({
        "summary": { "roles": [{ "name": "FIGHTER", "stats": { "play": 100 } }] }
    });
    let ad_last = vec![ItemStatEntry {
        ids: vec![3153],
        play: 50,
        win: 25,
        pick_rate: 0.5,
    }];
    assert_eq!(
        infer_rest_damage_type(&fighter_analysis, &[], &ad_last),
        "AD"
    );

    // 2. FIGHTER / ASSASSIN without SpellDamage tags default to "AD"
    let assassin_analysis = serde_json::json!({
        "summary": { "roles": [{ "name": "ASSASSIN", "stats": { "play": 100 } }] }
    });
    assert_eq!(infer_rest_damage_type(&fighter_analysis, &[], &[]), "AD");
    assert_eq!(infer_rest_damage_type(&assassin_analysis, &[], &[]), "AD");

    // 3. AP Assassin with SpellDamage items resolves "AP"
    let ap_last = vec![ItemStatEntry {
        ids: vec![6655],
        play: 50,
        win: 25,
        pick_rate: 0.5,
    }];
    assert_eq!(
        infer_rest_damage_type(&assassin_analysis, &[], &ap_last),
        "AP"
    );

    // Clean up dynamic cache for test isolation
    set_dynamic_item_classifications(ItemClassificationCache::default());
}

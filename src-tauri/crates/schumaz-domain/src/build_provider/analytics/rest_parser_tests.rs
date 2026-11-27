use super::*;
use crate::build_provider::items::is_boot_item;

#[test]
fn test_core_builds_and_separate_fourth_fifth_sixth_lists_yunara() {
    // Mirrors the exact OP.GG Yunara tables:
    // Core builds: [6672, 3085, 3031] -> 19,057 Games
    // Fourth/Fifth/Sixth: 3036 (18,151) -> 3026 (4,832) -> 3072 (1,036)
    let json = serde_json::json!({
        "data": {
            "damage_type": "AD",
            "win_rate": 50.0,
            "pick_rate": 11.9,
            "runes": {
                "primary_style_id": 8000,
                "sub_style_id": 8300,
                "primary_perk_ids": [8008, 9111, 9104, 8017],
                "sub_perk_ids": [8304, 8345],
                "stat_shards": [5005, 5008, 5011]
            },
            "summoner_spells": [
                { "ids": [4, 21], "play": 62087, "win": 30822 }
            ],
            "starter_items": [
                { "ids": [1055, 2003], "play": 76942, "win": 38123 }
            ],
            "boots": [
                { "ids": [3006], "play": 42413, "win": 22681 }
            ],
            "core_items": [
                { "ids": [3032, 3085, 3031], "play": 15006, "win": 8883 },
                { "ids": [6672, 3085, 3031], "play": 19057, "win": 10673 },
                { "ids": [6672, 3085, 2523], "play": 5163, "win": 2880 }
            ],
            "fourth_items": [
                { "ids": [3036], "play": 18151, "win": 11137 },
                { "ids": [3031], "play": 8958, "win": 5236 },
                { "ids": [3033], "play": 5651, "win": 3192 }
            ],
            "fifth_items": [
                { "ids": [3026], "play": 4832, "win": 3108 },
                { "ids": [3072], "play": 2585, "win": 1612 },
                { "ids": [6673], "play": 2549, "win": 1570 }
            ],
            "sixth_items": [
                { "ids": [3026], "play": 2035, "win": 1333 },
                { "ids": [3072], "play": 1036, "win": 630 },
                { "ids": [3139], "play": 638, "win": 401 }
            ],
            "last_items": [
                { "ids": [3085], "play": 75928, "win": 39359 },
                { "ids": [3031], "play": 59024, "win": 33741 },
                { "ids": [6672], "play": 50399, "win": 24649 },
                { "ids": [3032], "play": 34739, "win": 17553 },
                { "ids": [3036], "play": 30559, "win": 17570 }
            ]
        }
    });

    let build = parse_opgg_analysis_payload(&json, 804, "Yunara")
        .expect("Parsing Yunara build with core_items and 4th/5th/6th item lists should succeed");

    // Block 2 (Primeiros 3 Itens): must pick [6672, 3085, 3031] because 19,057 > 15,006
    assert_eq!(build.item_blocks[1].item_ids, vec![6672, 3085, 3031]);

    // Block 4 (Build Final): 3 Core ([6672, 3085, 3031]) + 4th (3036) + 5th (3026) + 6th (3072 since 3026 is already in 5th)
    assert_eq!(
        build.item_blocks[3].item_ids,
        vec![6672, 3085, 3031, 3036, 3026, 3072]
    );
}

#[test]
fn test_opgg_payload_normalizes_tear_items_in_blocks_and_options() {
    let analysis_json = serde_json::json!({
        "summary": { "position": "adc" },
        "core_items": [
            { "ids": [3078, 3042, 3036], "play": 15000, "win": 8000, "pick_rate": 45.0 }
        ],
        "boots": [
            { "ids": [3158], "play": 20000, "win": 11000, "pick_rate": 60.0 }
        ],
        "starters": [
            { "ids": [1055, 2003], "play": 25000, "win": 13000, "pick_rate": 80.0 }
        ],
        "fourth_items": [
            { "ids": [3072], "play": 8000, "win": 4500, "pick_rate": 30.0 }
        ],
        "fifth_items": [
            { "ids": [3026], "play": 6000, "win": 3200, "pick_rate": 25.0 }
        ],
        "sixth_items": [
            { "ids": [3139], "play": 4000, "win": 2100, "pick_rate": 20.0 }
        ],
        "last_items": [
            { "ids": [3040], "play": 3000, "win": 1600, "pick_rate": 15.0 }
        ],
        "runes": [
            {
                "primary_page_id": 8000,
                "primary_rune_ids": [8005, 9111, 9104, 8014],
                "secondary_page_id": 8200,
                "secondary_rune_ids": [8210, 8226],
                "stat_mod_ids": [5005, 5008, 5001],
                "play": 20000,
                "win": 10500
            }
        ],
        "spells": [
            { "ids": [4, 7], "play": 20000, "win": 10500, "pick_rate": 80.0 }
        ]
    });

    let build = parse_opgg_analysis_payload(&analysis_json, 81, "Ezreal")
        .expect("Build parsing should succeed");

    // Core block must contain 3004 (Manamune), NOT 3042 (Muramana)
    assert_eq!(build.item_blocks[1].item_ids, vec![3078, 3004, 3036]);
    assert!(!build.item_blocks[1].item_ids.contains(&3042));

    // Final build must contain 3004, NOT 3042
    assert!(build.item_blocks[3].item_ids.contains(&3004));
    assert!(!build.item_blocks[3].item_ids.contains(&3042));

    // Core options must have 3004, NOT 3042
    assert_eq!(build.build_options[0].core_item_ids, vec![3078, 3004, 3036]);

    // Situational items with 3040 must normalize to 3003
    assert!(!build.item_blocks[4].item_ids.contains(&3040));
}

#[test]
fn test_parse_opgg_analysis_extracts_9_runes_spells_and_items() {
    let opgg_fixture = serde_json::json!({
        "data": {
            "win_rate": 51.85,
            "pick_rate": 8.42,
            "runes": {
                "primary_style_id": 8100,
                "sub_style_id": 8200,
                "primary_perk_ids": [8112, 8126, 8138, 8135],
                "sub_perk_ids": [8226, 8210],
                "stat_shards": [5008, 5008, 5001]
            },
            "spells": [4, 14],
            "items": {
                "starters": [1056, 2003],
                "core": [3285, 3020, 3089],
                "situational": [3157, 3135],
                "boots": [3020, 3111, 3047]
            }
        }
    });

    let result = parse_opgg_analysis_payload(&opgg_fixture, 103, "Ahri");
    assert!(result.is_ok());

    let build = result.unwrap();
    assert_eq!(build.champion_id, 103);
    assert_eq!(build.champion_name, "Ahri");
    assert_eq!(build.tier, None);
    assert_eq!(build.primary_style_id, 8100);
    assert_eq!(build.sub_style_id, 8200);
    assert_eq!(build.selected_perk_ids.len(), 9);
    assert_eq!(
        build.selected_perk_ids,
        vec![8112, 8126, 8138, 8135, 8226, 8210, 5008, 5008, 5001]
    );
    assert_eq!(build.spell1_id, 4);
    assert_eq!(build.spell2_id, 14);
    assert_eq!(build.item_blocks.len(), 5);
    assert_eq!(build.item_blocks[0].title, "Itens Iniciais");
    assert_eq!(build.item_blocks[0].item_ids, vec![1056, 2003]);
    assert_eq!(build.item_blocks[1].title, "Primeiros 3 Itens");
    // 3020 is a boot, so it was filtered out and filled with situational item 3157, ensuring 3 core non-boot items!
    assert_eq!(build.item_blocks[1].item_ids, vec![3285, 3089, 3157]);
    assert_eq!(build.item_blocks[2].title, "3 Botas Mais Feitas");
    assert_eq!(build.item_blocks[2].item_ids, vec![3020, 3111, 3047]);
    assert_eq!(build.item_blocks[3].title, "Build Final");
    assert_eq!(build.item_blocks[4].title, "Itens Situacionais / Úteis");
    // Verify situational items are disjoint from core and final build!
    for &item in &build.item_blocks[4].item_ids {
        assert!(!build.item_blocks[1].item_ids.contains(&item));
        assert!(!build.item_blocks[3].item_ids.contains(&item));
        assert!(!is_boot_item(item));
    }
}

#[test]
fn test_parse_opgg_analysis_allows_missing_runes_when_supports_runes_is_false() {
    let mayhem_json = serde_json::json!({
        "data": {
            "damage_type": "AD",
            "summary": { "average_stats": { "win_rate": 0.52, "pick_rate": 0.08, "tier": 5 } },
            "runes": [],
            "summoner_spells": [
                { "ids": [4, 7], "play": 32000, "win": 17000 }
            ],
            "starter_items": [
                { "ids": [1038, 2003], "play": 27000, "win": 14400 }
            ],
            "boots": [
                { "ids": [3006], "play": 40000, "win": 21000 }
            ],
            "core_items": [
                { "ids": [3032, 6675, 3031], "play": 18000, "win": 9500 }
            ],
            "last_items": [
                { "ids": [3036], "play": 15000, "win": 8000 }
            ]
        }
    });

    // When supports_runes == true, missing runes should still return Err
    assert!(parse_opgg_analysis_payload(&mayhem_json, 18, "Tristana").is_err());

    // When supports_runes == false (ARAM Mayhem), missing runes must succeed!
    let build = parse_opgg_analysis_payload_with_options(&mayhem_json, 18, "Tristana", false)
        .expect("ARAM Mayhem payload without runes should parse successfully");
    assert!(!build.supports_runes);
    assert_eq!(build.tier, Some(4));
    assert_eq!(build.primary_style_id, 0);
    assert_eq!(build.sub_style_id, 0);
    assert!(build.selected_perk_ids.is_empty());
    assert!(build.rune_pages.is_empty());
    assert_eq!(build.spell1_id, 4);
    assert_eq!(build.spell2_id, 7);
    assert_eq!(build.active_position, "");

    // Also supports `supports_runes: false` directly inside the JSON payload
    let mut json_with_flag = mayhem_json.clone();
    json_with_flag["data"]["supports_runes"] = serde_json::json!(false);
    let build_from_flag = parse_opgg_analysis_payload(&json_with_flag, 18, "Tristana")
        .expect("JSON with supports_runes: false and no runes should succeed");
    assert!(!build_from_flag.supports_runes);
    assert!(build_from_flag.selected_perk_ids.is_empty());
}

#[test]
fn test_parse_opgg_analysis_extracts_position_tier_and_handles_off_meta_and_rip() {
    let base_payload = |position: &str, is_rip: bool| {
        serde_json::json!({
            "data": {
                "position": position,
                "summary": {
                    "is_rip": is_rip,
                    "average_stats": { "win_rate": 0.515, "pick_rate": 0.092, "tier": 1 },
                    "positions": [
                        { "name": "MID", "stats": { "tier_data": { "tier": 1, "rank": 2 } } },
                        { "name": "SUPPORT", "stats": { "tier_data": { "tier": 3, "rank": 18 } } },
                        { "name": "ADC", "stats": { "tier_data": { "tier": 6, "rank": 40 } } }
                    ]
                },
                "runes": {
                    "primary_style_id": 8200,
                    "sub_style_id": 8300,
                    "primary_perk_ids": [8229, 8226, 8210, 8237],
                    "sub_perk_ids": [8345, 8347],
                    "stat_shards": [5008, 5008, 5001]
                },
                "spells": [4, 14],
                "items": {
                    "starters": [1056, 2003],
                    "core": [6655, 4645, 3089],
                    "situational": [3157],
                    "boots": [3020]
                }
            }
        })
    };

    // Active position "mid" matches MID tier 1 -> canonical 0 (OP)
    let mid_build = parse_opgg_analysis_payload(&base_payload("mid", false), 99, "Lux").unwrap();
    assert_eq!(mid_build.tier, Some(0));

    // Active position "support" matches SUPPORT tier 3 -> canonical 2 (Tier 2)
    let sup_build =
        parse_opgg_analysis_payload(&base_payload("support", false), 99, "Lux").unwrap();
    assert_eq!(sup_build.tier, Some(2));

    // Raw tier > 5 clamps to canonical 4 (Tier 4; Tier 5 does not exist)
    let adc_build = parse_opgg_analysis_payload(&base_payload("adc", false), 99, "Lux").unwrap();
    assert_eq!(adc_build.tier, Some(4));

    // Off-meta position missing from summary.positions must return None (not leak average_stats.tier)
    let jg_build = parse_opgg_analysis_payload(&base_payload("jungle", false), 99, "Lux").unwrap();
    assert_eq!(jg_build.tier, None);

    // RIP champions must return None
    let rip_build = parse_opgg_analysis_payload(&base_payload("mid", true), 99, "Lux").unwrap();
    assert_eq!(rip_build.tier, None);
}

use super::*;
use crate::build_provider::*;

#[test]
fn test_build_ordering_strictly_by_play_count() {
    // Item 3118 (Malignance): 10,000 games, 5,100 wins (51.0% win rate)
    // Item 3089 (Rabadon): 1,000 games, 650 wins (65.0% win rate)
    // Item 4645 (Shadowflame): 8,000 games, 4,200 wins (52.5% win rate)
    // Item 3157 (Zhonya): 6,000 games, 3,300 wins (55.0% win rate)
    let starter = Some(ItemStatEntry {
        ids: vec![1056, 2003],
        play: 10000,
        win: 5000,
        pick_rate: 0.9,
    });
    let boots = vec![ItemStatEntry {
        ids: vec![3020],
        play: 8000,
        win: 4000,
        pick_rate: 0.8,
    }];
    let candidates = vec![
        ItemStatEntry {
            ids: vec![3118],
            play: 10000,
            win: 5100,
            pick_rate: 0.7,
        },
        ItemStatEntry {
            ids: vec![3089],
            play: 1000,
            win: 650,
            pick_rate: 0.2,
        },
        ItemStatEntry {
            ids: vec![4645],
            play: 8000,
            win: 4200,
            pick_rate: 0.5,
        },
        ItemStatEntry {
            ids: vec![3157],
            play: 6000,
            win: 3300,
            pick_rate: 0.4,
        },
    ];

    // Ordering MUST strictly follow play count descending (absolute matches played):
    let popular_blocks = build_5_item_blocks(starter, boots, candidates, "AP");
    // Ordered by play count: 3118 (10k), 4645 (8k), 3157 (6k)
    assert_eq!(popular_blocks[1].item_ids, vec![3118, 4645, 3157]);
}

#[test]
fn test_build_final_and_situational_exclude_components_and_cap_at_6() {
    let starter = Some(ItemStatEntry {
        ids: vec![1055, 2003],
        play: 10000,
        win: 5000,
        pick_rate: 0.9,
    });
    let boots = vec![ItemStatEntry {
        ids: vec![3006],
        play: 10000,
        win: 5000,
        pick_rate: 0.8,
    }];
    let candidates = vec![
        ItemStatEntry {
            ids: vec![6672, 3085, 3031],
            play: 16000,
            win: 8000,
            pick_rate: 0.5,
        }, // Core
        ItemStatEntry {
            ids: vec![3032],
            play: 12000,
            win: 6000,
            pick_rate: 0.4,
        }, // Yun Tal (4th)
        ItemStatEntry {
            ids: vec![1038],
            play: 11000,
            win: 5500,
            pick_rate: 0.35,
        }, // B.F. Sword (component - MUST BE SKIPPED!)
        ItemStatEntry {
            ids: vec![3036],
            play: 10000,
            win: 5000,
            pick_rate: 0.3,
        }, // LDR (5th)
        ItemStatEntry {
            ids: vec![3026],
            play: 9000,
            win: 4500,
            pick_rate: 0.28,
        }, // GA (6th)
        ItemStatEntry {
            ids: vec![2523],
            play: 8000,
            win: 4000,
            pick_rate: 0.25,
        }, // Situational 1
        ItemStatEntry {
            ids: vec![2512],
            play: 7000,
            win: 3500,
            pick_rate: 0.2,
        }, // Situational 2
        ItemStatEntry {
            ids: vec![3033],
            play: 6000,
            win: 3000,
            pick_rate: 0.18,
        }, // Situational 3
        ItemStatEntry {
            ids: vec![6673],
            play: 4000,
            win: 2000,
            pick_rate: 0.12,
        }, // Situational 4
        ItemStatEntry {
            ids: vec![3072],
            play: 3000,
            win: 1500,
            pick_rate: 0.1,
        }, // Situational 5
        ItemStatEntry {
            ids: vec![3139],
            play: 2000,
            win: 1000,
            pick_rate: 0.08,
        }, // Situational 6
        ItemStatEntry {
            ids: vec![3046],
            play: 1000,
            win: 500,
            pick_rate: 0.04,
        }, // Situational 7 (should be dropped by cap)
    ];

    let blocks = build_5_item_blocks(starter, boots, candidates, "AD");

    // Core items
    assert_eq!(blocks[1].item_ids, vec![6672, 3085, 3031]);

    // Build Final: 6 items, must NEVER contain component 1038 (B.F. Sword)!
    // Should contain core (6672, 3085, 3031) followed by 3032 (4th), 3036 (5th), and 3026 (6th)
    assert_eq!(blocks[3].item_ids, vec![6672, 3085, 3031, 3032, 3036, 3026]);

    // Situational items: max 6 items, must not contain 1038 or any core/final items
    assert_eq!(blocks[4].item_ids.len(), 6);
    assert!(!blocks[4].item_ids.contains(&1038));
    for &id in &blocks[4].item_ids {
        assert!(!blocks[1].item_ids.contains(&id));
        assert!(!blocks[3].item_ids.contains(&id));
    }
}

#[test]
fn test_situational_items_filtered_and_sorted_by_play() {
    let starter = Some(ItemStatEntry {
        ids: vec![1056, 2003],
        play: 50000,
        win: 25000,
        pick_rate: 0.9,
    });
    let boots = vec![ItemStatEntry {
        ids: vec![3020],
        play: 40000,
        win: 20000,
        pick_rate: 0.8,
    }];
    let candidates = vec![
        ItemStatEntry {
            ids: vec![3118, 4645, 3157],
            play: 20000,
            win: 10000,
            pick_rate: 0.4,
        }, // Core
        ItemStatEntry {
            ids: vec![3089],
            play: 8000,
            win: 4500,
            pick_rate: 0.3,
        }, // Final build 4th
        ItemStatEntry {
            ids: vec![3135],
            play: 5000,
            win: 2500,
            pick_rate: 0.2,
        }, // Final build 5th
        ItemStatEntry {
            ids: vec![3102],
            play: 4000,
            win: 2100,
            pick_rate: 0.18,
        }, // Final build 6th
        ItemStatEntry {
            ids: vec![4646],
            play: 1500,
            win: 700,
            pick_rate: 0.1,
        }, // Situational (1500 plays)
        ItemStatEntry {
            ids: vec![4629],
            play: 3200,
            win: 1600,
            pick_rate: 0.15,
        }, // Situational (3200 plays)
        ItemStatEntry {
            ids: vec![3165],
            play: 700,
            win: 350,
            pick_rate: 0.05,
        }, // Situational (700 plays)
    ];

    let blocks = build_5_item_blocks(starter, boots, candidates, "AP");

    // Clean core (first 3): 3118, 4645, 3157
    assert_eq!(blocks[1].item_ids, vec![3118, 4645, 3157]);
    // Final build (6 items): 3118, 4645, 3157, 3089, 3135, 3102
    assert_eq!(blocks[3].item_ids, vec![3118, 4645, 3157, 3089, 3135, 3102]);

    // Situational items:
    // Must NOT contain any item from core or final build:
    for &id in &blocks[4].item_ids {
        assert!(!blocks[1].item_ids.contains(&id));
        assert!(!blocks[3].item_ids.contains(&id));
        assert!(!is_boot_item(id));
    }

    // Must be ordered strictly by play count descending:
    // 4629 (3200 plays) should appear BEFORE 4646 (1500 plays) which is BEFORE 3165 (700 plays)!
    let pos_4629 = blocks[4].item_ids.iter().position(|&x| x == 4629).unwrap();
    let pos_4646 = blocks[4].item_ids.iter().position(|&x| x == 4646).unwrap();
    let pos_3165 = blocks[4].item_ids.iter().position(|&x| x == 3165).unwrap();
    assert!(pos_4629 < pos_4646);
    assert!(pos_4646 < pos_3165);
}

#[test]
fn test_starter_items_by_position_adc_top_mid_jungle_support() {
    // Standard ADC (Yunara): Doran's Blade (1055, 76k) > Doran's Shield (1054, 12k) + Potion (2003)
    let adc_starters = vec![
        ItemStatEntry {
            ids: vec![1055, 2003],
            play: 76942,
            win: 38123,
            pick_rate: 0.8,
        },
        ItemStatEntry {
            ids: vec![1054, 2003],
            play: 12000,
            win: 6000,
            pick_rate: 0.15,
        },
    ];
    let adc_blocks = build_5_item_blocks_with_depth_lists(
        adc_starters,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AD",
        "adc",
    );
    assert_eq!(adc_blocks[0].title, "Itens Iniciais");
    assert_eq!(adc_blocks[0].item_ids, vec![1055, 2003]);

    // Standard Top (Tank): Doran's Shield (1054, 40k) > Doran's Blade (1055, 5k) + Potion (2003)
    let top_starters = vec![
        ItemStatEntry {
            ids: vec![1054, 2003],
            play: 40000,
            win: 20000,
            pick_rate: 0.7,
        },
        ItemStatEntry {
            ids: vec![1055, 2003],
            play: 5000,
            win: 2500,
            pick_rate: 0.1,
        },
    ];
    let top_blocks = build_5_item_blocks_with_depth_lists(
        top_starters,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AD",
        "top",
    );
    assert_eq!(top_blocks[0].item_ids, vec![1054, 2003]);

    // Standard Mid (Mage): Doran's Ring (1056, 30k) > Dark Seal (1082, 5k) + Potion (2003)
    let mid_starters = vec![
        ItemStatEntry {
            ids: vec![1056, 2003],
            play: 30000,
            win: 15000,
            pick_rate: 0.8,
        },
        ItemStatEntry {
            ids: vec![1082, 2003],
            play: 5000,
            win: 2500,
            pick_rate: 0.1,
        },
    ];
    let mid_blocks = build_5_item_blocks_with_depth_lists(
        mid_starters,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AP",
        "mid",
    );
    assert_eq!(mid_blocks[0].item_ids, vec![1056, 2003]);

    // Jungle: Mosstomper (1102, 50k) > Gustwalker (1103, 20k) > Scorchclaw (1101, 10k) + Potion (2003)
    let jng_starters = vec![
        ItemStatEntry {
            ids: vec![1102, 2003],
            play: 50000,
            win: 25000,
            pick_rate: 0.6,
        },
        ItemStatEntry {
            ids: vec![1103, 2003],
            play: 20000,
            win: 10000,
            pick_rate: 0.25,
        },
        ItemStatEntry {
            ids: vec![1101, 2003],
            play: 10000,
            win: 5000,
            pick_rate: 0.15,
        },
    ];
    let jng_blocks = build_5_item_blocks_with_depth_lists(
        jng_starters,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AD",
        "jungle",
    );
    assert_eq!(jng_blocks[0].item_ids, vec![1102, 2003]);

    // Support: Quest item is pre-granted/bought, must ONLY contain Potion!
    let supp_starters = vec![ItemStatEntry {
        ids: vec![3865, 2003, 2003],
        play: 60000,
        win: 30000,
        pick_rate: 0.95,
    }];
    let supp_blocks = build_5_item_blocks_with_depth_lists(
        supp_starters,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AP",
        "support",
    );
    assert_eq!(supp_blocks[0].item_ids, vec![2003]);
}

#[test]
fn test_boots_ranking_prioritizes_most_played_boots() {
    // Boot 3020 (Sorcerer's): 50,000 games, 50% win rate
    // Boot 3158 (Lucidity): 5,000 games, 60% win rate
    // Boot 3111 (Mercs): 10,000 games, 52% win rate
    let starter = Some(ItemStatEntry {
        ids: vec![1056, 2003],
        play: 50000,
        win: 25000,
        pick_rate: 0.9,
    });
    let boots = vec![
        ItemStatEntry {
            ids: vec![3158],
            play: 5000,
            win: 3000,
            pick_rate: 0.1,
        },
        ItemStatEntry {
            ids: vec![3020],
            play: 50000,
            win: 25000,
            pick_rate: 0.7,
        },
        ItemStatEntry {
            ids: vec![3111],
            play: 10000,
            win: 5200,
            pick_rate: 0.2,
        },
    ];
    let candidates = vec![ItemStatEntry {
        ids: vec![3118, 4645, 3089],
        play: 20000,
        win: 11000,
        pick_rate: 0.5,
    }];

    let blocks = build_5_item_blocks(starter, boots, candidates, "AP");

    // Boots MUST be sorted by play count descending: 3020 (50k) -> 3111 (10k) -> 3158 (5k)
    assert_eq!(blocks[2].title, "3 Botas Mais Feitas");
    assert_eq!(blocks[2].item_ids, vec![3020, 3111, 3158]);
}

#[test]
fn test_boots_ranking_yunara_with_synchronized_souls() {
    // Mirrors OP.GG boots table for Yunara:
    // 1. 3008 (Synchronized Souls) -> 41,774 Games (53.06%)
    // 2. 3006 (Berserker's Greaves) -> 33,119 Games (42.07%)
    // 3. 3047 (Plated Steelcaps) -> 572 Games (0.73%)
    // 4. 3009 (Boots of Swiftness) -> 257 Games (0.33%)
    // 5. 3111 (Mercury's Treads) -> 252 Games (0.32%)
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
                { "ids": [3008], "play": 41774, "win": 22345, "pick_rate": 0.5306 },
                { "ids": [3006], "play": 33119, "win": 15735, "pick_rate": 0.4207 },
                { "ids": [3047], "play": 572, "win": 286, "pick_rate": 0.0073 },
                { "ids": [3009], "play": 257, "win": 128, "pick_rate": 0.0033 },
                { "ids": [3111], "play": 252, "win": 126, "pick_rate": 0.0032 }
            ],
            "core_items": [
                { "ids": [6672, 3085, 3031], "play": 19057, "win": 10673 }
            ],
            "fourth_items": [
                { "ids": [3036], "play": 18151, "win": 11137 }
            ],
            "fifth_items": [
                { "ids": [3026], "play": 4832, "win": 3108 }
            ],
            "sixth_items": [
                { "ids": [3072], "play": 1036, "win": 630 }
            ]
        }
    });

    let build = parse_opgg_analysis_payload(&json, 804, "Yunara")
        .expect("Parsing Yunara build with boots list should succeed");

    // Verify Block 3 (3 Botas Mais Feitas):
    assert_eq!(build.item_blocks[2].title, "3 Botas Mais Feitas");
    // Must be in strict order of play count descending: 3008 (41,774) -> 3006 (33,119) -> 3047 (572)
    assert_eq!(build.item_blocks[2].item_ids, vec![3008, 3006, 3047]);

    // Verify boots never leak into Core (Block 1), Final Build (Block 3), or Situational (Block 4)
    for &boot_id in &[3008, 3006, 3047, 3009, 3111] {
        assert!(
            !build.item_blocks[1].item_ids.contains(&boot_id),
            "Boot {boot_id} leaked into core"
        );
        assert!(
            !build.item_blocks[3].item_ids.contains(&boot_id),
            "Boot {boot_id} leaked into final build"
        );
        assert!(
            !build.item_blocks[4].item_ids.contains(&boot_id),
            "Boot {boot_id} leaked into situational"
        );
    }

    // Verify is_boot_item recognizes 3008, 3010, and 3013
    assert!(is_boot_item(3008));
    assert!(is_boot_item(3010));
    assert!(is_boot_item(3013));
}

#[test]
fn test_aram_starter_preserves_top_potion_and_allows_no_potion_combos() {
    // 1. Tristana ARAM: #1 [1038, 2003, 2003] (play 1397) vs secondary [1038] (398),
    //    [1042, 3006] (251), [2031, 3051] (121), [2051, 2003] (80).
    //    Must select [1038, 2003] without letting secondary 2031 overwrite potion or forcing 2051.
    let tristana_starters = vec![
        ItemStatEntry {
            ids: vec![2051, 2003],
            play: 80,
            win: 40,
            pick_rate: 0.02,
        },
        ItemStatEntry {
            ids: vec![1038, 2003, 2003],
            play: 1397,
            win: 745,
            pick_rate: 0.415,
        },
        ItemStatEntry {
            ids: vec![1038],
            play: 398,
            win: 222,
            pick_rate: 0.118,
        },
        ItemStatEntry {
            ids: vec![1042, 3006],
            play: 251,
            win: 124,
            pick_rate: 0.075,
        },
        ItemStatEntry {
            ids: vec![2031, 3051],
            play: 121,
            win: 69,
            pick_rate: 0.036,
        },
    ];
    let trist_blocks = build_5_item_blocks_with_depth_lists(
        tristana_starters,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AD",
        "",
    );
    assert_eq!(trist_blocks[0].item_ids, vec![1038, 2003]);

    // 2. Solo B.F. Sword [1038] without potion must NOT force 2003
    let solo_bf = build_5_item_blocks_with_depth_lists(
        vec![
            ItemStatEntry {
                ids: vec![1038],
                play: 900,
                win: 500,
                pick_rate: 0.5,
            },
            ItemStatEntry {
                ids: vec![1038, 2003],
                play: 200,
                win: 100,
                pick_rate: 0.1,
            },
        ],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AD",
        "",
    );
    assert_eq!(solo_bf[0].item_ids, vec![1038]);

    // 3. Two components without potion [1037, 1042] must preserve both without adding 2003
    let two_components = build_5_item_blocks_with_depth_lists(
        vec![ItemStatEntry {
            ids: vec![1037, 1042],
            play: 1200,
            win: 650,
            pick_rate: 0.4,
        }],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AD",
        "",
    );
    assert_eq!(two_components[0].item_ids, vec![1037, 1042]);

    // 4. ARAM exclusive starter [2051] / [3112] without consumables must NOT force 2003
    let horn_only = build_5_item_blocks_with_depth_lists(
        vec![ItemStatEntry {
            ids: vec![2051],
            play: 1500,
            win: 820,
            pick_rate: 0.5,
        }],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AD",
        "",
    );
    assert_eq!(horn_only[0].item_ids, vec![2051]);

    let orb_only = build_5_item_blocks_with_depth_lists(
        vec![ItemStatEntry {
            ids: vec![3112],
            play: 1500,
            win: 820,
            pick_rate: 0.5,
        }],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AP",
        "",
    );
    assert_eq!(orb_only[0].item_ids, vec![3112]);

    // 5. OP.GG numeric sorting puts consumables first: [2031, 3802] -> [3802, 2031]
    //    and [2003, 2003, 3057, 3070] -> [3057, 3070, 2003]
    let ahri_blocks = build_5_item_blocks_with_depth_lists(
        vec![ItemStatEntry {
            ids: vec![2031, 3802],
            play: 2000,
            win: 1100,
            pick_rate: 0.5,
        }],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AP",
        "",
    );
    assert_eq!(ahri_blocks[0].item_ids, vec![3802, 2031]);

    let ezreal_blocks = build_5_item_blocks_with_depth_lists(
        vec![ItemStatEntry {
            ids: vec![2003, 2003, 3057, 3070],
            play: 2000,
            win: 1100,
            pick_rate: 0.5,
        }],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AD",
        "",
    );
    assert_eq!(ezreal_blocks[0].item_ids, vec![3057, 3070, 2003]);
}

#[test]
fn test_starter_entries_with_boots_and_completed_items_are_not_filtered_from_other_blocks() {
    let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
    let ddragon_items_json = serde_json::json!({
        "data": {
            "1042": { "name": "Dagger", "into": ["3006"], "gold": { "total": 250 } },
            "3006": { "name": "Berserker's Greaves", "tags": ["Boots"], "into": ["3172"], "gold": { "total": 1100 } },
            "3008": { "name": "Gluttonous Greaves", "tags": ["Boots"], "gold": { "total": 1000 } },
            "3111": { "name": "Mercury's Treads", "tags": ["Boots"], "into": ["3173"], "gold": { "total": 1200 } },
            "3032": { "name": "Yun Tal Wildarrows", "gold": { "total": 3000 } },
            "6675": { "name": "Navori Flickerblade", "gold": { "total": 2650 } },
            "3031": { "name": "Infinity Edge", "gold": { "total": 3450 } },
            "3036": { "name": "Lord Dominik's Regards", "gold": { "total": 3100 } },
            "3072": { "name": "Bloodthirster", "gold": { "total": 3400 } },
            "6676": { "name": "The Collector", "gold": { "total": 3000 } },
            "3153": { "name": "Blade of the Ruined King", "gold": { "total": 3200 } }
        }
    });
    set_dynamic_item_classifications(parse_ddragon_item_classifications(&ddragon_items_json));

    let starters = vec![
        ItemStatEntry {
            ids: vec![1042, 3006],
            play: 1500,
            win: 800,
            pick_rate: 0.45,
        },
        ItemStatEntry {
            ids: vec![3032], // edge-case completed item present in starter_entries
            play: 100,
            win: 55,
            pick_rate: 0.03,
        },
    ];
    let boots = vec![
        ItemStatEntry {
            ids: vec![3006],
            play: 2640,
            win: 1415,
            pick_rate: 0.85,
        },
        ItemStatEntry {
            ids: vec![3008],
            play: 264,
            win: 166,
            pick_rate: 0.08,
        },
        ItemStatEntry {
            ids: vec![3111],
            play: 124,
            win: 62,
            pick_rate: 0.04,
        },
    ];
    let candidates = vec![
        ItemStatEntry {
            ids: vec![3032, 6675, 3031],
            play: 598,
            win: 329,
            pick_rate: 0.20,
        },
        ItemStatEntry {
            ids: vec![6676],
            play: 1007,
            win: 551,
            pick_rate: 0.28,
        },
        ItemStatEntry {
            ids: vec![3153],
            play: 528,
            win: 322,
            pick_rate: 0.15,
        },
    ];

    let blocks = build_5_item_blocks_with_depth_lists(
        starters,
        boots,
        candidates,
        vec![ItemStatEntry {
            ids: vec![3036],
            play: 593,
            win: 346,
            pick_rate: 0.29,
        }],
        vec![ItemStatEntry {
            ids: vec![3072],
            play: 255,
            win: 160,
            pick_rate: 0.29,
        }],
        vec![],
        "AD",
        "",
    );

    // [1042, 3006] is preserved as starter without forcing a potion
    assert_eq!(blocks[0].item_ids, vec![1042, 3006]);
    // 3032 was NOT filtered out as a starter from Core / Build Final
    assert_eq!(blocks[1].item_ids, vec![3032, 6675, 3031]);
    // 3006 was NOT filtered out as a starter from 3 Botas Mais Feitas
    assert_eq!(blocks[2].item_ids, vec![3006, 3008, 3111]);
    assert_eq!(blocks[3].item_ids, vec![3032, 6675, 3031, 3036, 3072, 6676]);
    assert!(blocks[4].item_ids.contains(&3153));
}

#[test]
fn test_jungle_and_support_starter_preserve_top_entry_potion() {
    // Support: #1 has no potion [3865], #2 has [3865, 2031] -> must NOT leak 2031 from #2
    let supp_no_pot_first = build_5_item_blocks_with_depth_lists(
        vec![
            ItemStatEntry {
                ids: vec![3865],
                play: 50000,
                win: 25000,
                pick_rate: 0.8,
            },
            ItemStatEntry {
                ids: vec![3865, 2031],
                play: 5000,
                win: 2500,
                pick_rate: 0.1,
            },
        ],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AP",
        "support",
    );
    assert_eq!(supp_no_pot_first[0].item_ids, vec![2003]);

    // Jungle: #1 [1102] without potion -> keeps only [1102] without forcing 2003 or leaking #2's 2031
    let jng_no_pot_first = build_5_item_blocks_with_depth_lists(
        vec![
            ItemStatEntry {
                ids: vec![1102],
                play: 50000,
                win: 25000,
                pick_rate: 0.8,
            },
            ItemStatEntry {
                ids: vec![1102, 2031],
                play: 5000,
                win: 2500,
                pick_rate: 0.1,
            },
        ],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AD",
        "jungle",
    );
    assert_eq!(jng_no_pot_first[0].item_ids, vec![1102]);
}

use super::*;
use crate::build_provider::*;

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

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

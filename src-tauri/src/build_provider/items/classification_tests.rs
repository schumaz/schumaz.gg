use super::*;
use crate::build_provider::*;

#[test]
fn test_normalize_tear_item_dynamic_and_static_fallbacks() {
    assert_eq!(normalize_tear_item_dynamic(3042), 3004); // Muramana -> Manamune
    assert_eq!(normalize_tear_item_dynamic(3040), 3003); // Seraph's Embrace -> Archangel's Staff
    assert_eq!(normalize_tear_item_dynamic(3121), 3119); // Fimbulwinter -> Winter's Approach
    assert_eq!(normalize_tear_item_dynamic(3089), 3089); // Untransformed Rabadon stays 3089
    assert_eq!(
        normalize_item_ids_dedup(&[3042, 3031, 3004]),
        vec![3004, 3031] // 3042 becomes 3004 and deduplicates with subsequent 3004
    );
}

#[test]
fn test_resolve_tear_to_completed_item_by_archetype() {
    let empty_entries: Vec<ItemStatEntry> = Vec::new();
    assert_eq!(
        resolve_tear_to_completed_item(&empty_entries, "support", "AP"),
        2526
    ); // Whispering Circlet
    assert_eq!(
        resolve_tear_to_completed_item(&empty_entries, "mid", "AP"),
        3003
    ); // Archangel's Staff
    assert_eq!(
        resolve_tear_to_completed_item(&empty_entries, "adc", "AD"),
        3004
    ); // Manamune
    assert_eq!(
        resolve_tear_to_completed_item(&empty_entries, "top", "TANK"),
        3119
    ); // Winter's Approach
}

#[test]
fn test_diadem_of_songs_normalizes_to_whispering_circlet() {
    assert_eq!(normalize_tear_item_dynamic(2530), 2526);
    assert_eq!(
        resolve_tear_item_ids(&[2530, 3001, 3070], 2526),
        vec![2526, 3001]
    );
}

#[test]
fn test_tear_item_merges_play_count_with_base_item() {
    let starter_entry = Some(ItemStatEntry {
        ids: vec![1055, 2003],
        play: 50000,
        win: 25000,
        pick_rate: 0.9,
    });
    let boots = vec![ItemStatEntry {
        ids: vec![3006],
        play: 50000,
        win: 25000,
        pick_rate: 0.9,
    }];
    let candidate_entries = vec![
        ItemStatEntry {
            ids: vec![3078, 3036, 3072],
            play: 25000,
            win: 13000,
            pick_rate: 0.5,
        },
        // Both 3004 and 3042 present:
        ItemStatEntry {
            ids: vec![3004],
            play: 5000,
            win: 2500,
            pick_rate: 0.1,
        },
        ItemStatEntry {
            ids: vec![3042],
            play: 15000,
            win: 8000,
            pick_rate: 0.3,
        },
        ItemStatEntry {
            ids: vec![3026],
            play: 4000,
            win: 2000,
            pick_rate: 0.08,
        },
    ];

    let blocks = build_5_item_blocks(starter_entry, boots, candidate_entries, "AD");
    // Check that 3004 appears in Final Build (promoted due to combined play 5k + 15k = 20k),
    // without duplicates and without 3042.
    let final_ids = &blocks[3].item_ids;
    assert!(!final_ids.contains(&3042));
    let count_3004 = final_ids.iter().filter(|&&id| id == 3004).count();
    assert_eq!(count_3004, 1);
    // And 3004 was placed before 3026 because 3004 combined play (20k) > 3026 play (4k)
    let pos_3004 = final_ids.iter().position(|&x| x == 3004).unwrap();
    let pos_3026 = final_ids.iter().position(|&x| x == 3026).unwrap();
    assert!(pos_3004 < pos_3026);
}

#[test]
fn test_seraphine_tear_in_core_promotes_to_whispering_circlet() {
    // Seraphine payload: core items has [6617, 3001, 3070] (Helia, Moonstone, Tear 3070)
    // And later entries has 2530 (Diadem of Songs) / 2526 (Whispering Circlet)
    let analysis_json = serde_json::json!({
        "summary": { "position": "support" },
        "core_items": [
            { "ids": [6617, 3001, 3070], "play": 18000, "win": 9500, "pick_rate": 55.0 }
        ],
        "boots": [
            { "ids": [3158], "play": 25000, "win": 13500, "pick_rate": 75.0 }
        ],
        "starters": [
            { "ids": [2003], "play": 28000, "win": 15000, "pick_rate": 90.0 }
        ],
        "fourth_items": [
            { "ids": [2530], "play": 12000, "win": 6800, "pick_rate": 45.0 }
        ],
        "fifth_items": [
            { "ids": [3107], "play": 8000, "win": 4300, "pick_rate": 30.0 }
        ],
        "sixth_items": [
            { "ids": [3504], "play": 5000, "win": 2800, "pick_rate": 20.0 }
        ],
        "runes": [
            {
                "primary_page_id": 8200,
                "primary_rune_ids": [8214, 8226, 8210, 8237],
                "secondary_page_id": 8300,
                "secondary_rune_ids": [8304, 8345],
                "stat_mod_ids": [5008, 5008, 5001],
                "play": 22000,
                "win": 12000
            }
        ],
        "spells": [
            { "ids": [4, 7], "play": 25000, "win": 13500, "pick_rate": 85.0 }
        ]
    });

    let build = parse_opgg_analysis_payload(&analysis_json, 147, "Seraphine")
        .expect("Build parsing should succeed");

    // Core block must NOT contain 3070 (Tear); it must contain 2526 (Whispering Circlet)!
    assert_eq!(build.item_blocks[1].item_ids, vec![6617, 3001, 2526]);
    assert!(!build.item_blocks[1].item_ids.contains(&3070));
    assert!(!build.item_blocks[1].item_ids.contains(&2530));

    // Core options must also contain 2526, not 3070
    assert_eq!(build.build_options[0].core_item_ids, vec![6617, 3001, 2526]);

    // Final build must contain 2526 exactly once, without 3070 or 2530
    let final_ids = &build.item_blocks[3].item_ids;
    assert_eq!(final_ids.iter().filter(|&&id| id == 2526).count(), 1);
    assert!(!final_ids.contains(&3070));
    assert!(!final_ids.contains(&2530));
}

#[test]
fn test_future_proof_dynamic_classification_and_opgg_context_trust() {
    let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
    // Simulate a new patch with completely un-hardcoded future item IDs:
    // Item 9901: brand new boot introduced by Riot (tags: ["Boots"])
    // Item 9902: brand new component item (has "into" recipe)
    // Item 9903: brand new completed legendary item (cost >= 1600, no "into")
    let ddragon_items_json = serde_json::json!({
        "data": {
            "9901": {
                "name": "Astral Treads",
                "tags": ["Boots"],
                "gold": { "total": 1100 }
            },
            "9902": {
                "name": "Arcane Filament",
                "into": ["9903"],
                "tags": ["Damage"],
                "gold": { "total": 850 }
            },
            "9903": {
                "name": "Celestial Singularity",
                "tags": ["Damage"],
                "gold": { "total": 3300 }
            }
        }
    });

    let cache = parse_ddragon_item_classifications(&ddragon_items_json);
    set_dynamic_item_classifications(cache);

    // Verify dynamic classifier recognized future items without any hardcoded IDs:
    assert!(is_boot_item_dynamic(9901));
    assert!(is_component_item_dynamic(9902));
    assert!(!is_component_item_dynamic(9903));

    // Test building blocks with these items in OP.GG payload:
    let boots = vec![ItemStatEntry {
        ids: vec![9901],
        play: 50000,
        win: 25000,
        pick_rate: 0.9,
    }];
    let candidate_entries = vec![
        ItemStatEntry {
            ids: vec![9903, 3089, 3157],
            play: 25000,
            win: 13000,
            pick_rate: 0.5,
        },
        ItemStatEntry {
            ids: vec![9902],
            play: 20000,
            win: 10000,
            pick_rate: 0.4,
        }, // Component - must be skipped!
        ItemStatEntry {
            ids: vec![3135],
            play: 15000,
            win: 8000,
            pick_rate: 0.3,
        },
        ItemStatEntry {
            ids: vec![3102],
            play: 12000,
            win: 6000,
            pick_rate: 0.25,
        },
        ItemStatEntry {
            ids: vec![4629],
            play: 10000,
            win: 5000,
            pick_rate: 0.2,
        },
        ItemStatEntry {
            ids: vec![4646],
            play: 8000,
            win: 4000,
            pick_rate: 0.15,
        },
    ];

    let blocks = build_5_item_blocks(
        Some(ItemStatEntry {
            ids: vec![1056, 2003],
            play: 50000,
            win: 25000,
            pick_rate: 0.9,
        }),
        boots,
        candidate_entries,
        "AP",
    );

    // 1. Boot block must contain the new boot 9901
    assert_eq!(blocks[2].title, "3 Botas Mais Feitas");
    assert!(blocks[2].item_ids.contains(&9901));

    // 2. Component 9902 must NEVER appear in Core, Final Build, or Situational blocks
    assert!(!blocks[1].item_ids.contains(&9902));
    assert!(!blocks[3].item_ids.contains(&9902));
    assert!(!blocks[4].item_ids.contains(&9902));

    // 3. New Legendary 9903 must be correctly integrated in Core and Final Build
    assert!(blocks[1].item_ids.contains(&9903));
    assert!(blocks[3].item_ids.contains(&9903));
}

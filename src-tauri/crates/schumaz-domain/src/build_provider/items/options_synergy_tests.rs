use super::super::classification::TEST_CLASSIFICATION_MUTEX;
use super::*;
use crate::build_provider::models::ItemClassificationCache;
use crate::build_provider::*;

#[test]
fn test_ezreal_mana_synergy_recognizes_frozen_heart() {
    let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
    let mut cache = ItemClassificationCache::default();
    cache.item_tags.insert(
        3078,
        vec![
            "Damage".to_string(),
            "AttackSpeed".to_string(),
            "Health".to_string(),
            "AbilityHaste".to_string(),
        ],
    ); // Triforce
    cache.item_tags.insert(
        3004,
        vec![
            "Damage".to_string(),
            "Mana".to_string(),
            "AbilityHaste".to_string(),
        ],
    ); // Manamune
    cache.item_tags.insert(
        3036,
        vec!["Damage".to_string(), "ArmorPenetration".to_string()],
    ); // LDR
    cache.item_tags.insert(
        3072,
        vec![
            "Damage".to_string(),
            "LifeSteal".to_string(),
            "CriticalStrike".to_string(),
        ],
    ); // BT
    cache.item_tags.insert(
        3110,
        vec![
            "Armor".to_string(),
            "Mana".to_string(),
            "AbilityHaste".to_string(),
        ],
    ); // Frozen Heart
    cache.item_tags.insert(
        3153,
        vec![
            "Damage".to_string(),
            "AttackSpeed".to_string(),
            "LifeSteal".to_string(),
        ],
    ); // BotRK
    cache
        .completed
        .extend(vec![3078, 3004, 3036, 3072, 3110, 3153]);
    set_dynamic_item_classifications(cache);

    let core = vec![3078, 3004, 3036];
    let fourth = vec![ItemStatEntry {
        ids: vec![3072],
        play: 2000,
        win: 1100,
        pick_rate: 0.4,
    }];
    let fifth = vec![
        ItemStatEntry {
            ids: vec![3110],
            play: 1500,
            win: 850,
            pick_rate: 0.3,
        }, // Frozen Heart
    ];
    let sixth = vec![ItemStatEntry {
        ids: vec![3153],
        play: 1000,
        win: 550,
        pick_rate: 0.2,
    }];

    let final_6 = build_cohesive_final_build_for_core(&core, &fourth, &fifth, &sixth, &[], "AD");

    assert_eq!(final_6, vec![3078, 3004, 3036, 3072, 3110, 3153]);
    assert!(
        final_6.contains(&3110),
        "Frozen Heart must be recognized via Mana synergy"
    );
}

#[test]
fn test_option_situational_excludes_core_final_boots_components() {
    let core = vec![3031, 3094, 3036];
    let final_items = vec![3031, 3094, 3036, 3026, 3072, 3139];
    let fourth = vec![ItemStatEntry {
        ids: vec![3026],
        play: 1000,
        win: 500,
        pick_rate: 0.1,
    }];
    let fifth = vec![ItemStatEntry {
        ids: vec![3072],
        play: 800,
        win: 400,
        pick_rate: 0.1,
    }];
    let sixth = vec![ItemStatEntry {
        ids: vec![3139],
        play: 600,
        win: 300,
        pick_rate: 0.1,
    }];
    let candidates = vec![
        ItemStatEntry {
            ids: vec![3006],
            play: 5000,
            win: 2500,
            pick_rate: 0.5,
        }, // Greaves (boot) -> exclude
        ItemStatEntry {
            ids: vec![1055],
            play: 4000,
            win: 2000,
            pick_rate: 0.4,
        }, // Doran's Blade (starter) -> exclude
        ItemStatEntry {
            ids: vec![1038],
            play: 3000,
            win: 1500,
            pick_rate: 0.3,
        }, // BF Sword (component) -> exclude
        ItemStatEntry {
            ids: vec![3046],
            play: 2500,
            win: 1200,
            pick_rate: 0.25,
        }, // Phantom Dancer -> VALID
        ItemStatEntry {
            ids: vec![3085],
            play: 2000,
            win: 1000,
            pick_rate: 0.2,
        }, // Runaan's Hurricane -> VALID
        ItemStatEntry {
            ids: vec![3078],
            play: 1500,
            win: 750,
            pick_rate: 0.15,
        }, // Trinity Force -> VALID
        ItemStatEntry {
            ids: vec![6676],
            play: 1200,
            win: 600,
            pick_rate: 0.12,
        }, // Collector -> VALID
        ItemStatEntry {
            ids: vec![3153],
            play: 1000,
            win: 500,
            pick_rate: 0.1,
        }, // Blade of Ruined King -> VALID
    ];

    let sit = build_situational_items_for_option(
        &core,
        &final_items,
        &fourth,
        &fifth,
        &sixth,
        &candidates,
        &[],
        &[],
        "AD",
    );

    // Core and final build must be completely excluded
    for id in &core {
        assert!(
            !sit.contains(id),
            "Core item {id} must not be in situational"
        );
    }
    for id in &final_items {
        assert!(
            !sit.contains(id),
            "Final item {id} must not be in situational"
        );
    }
    // Boots, starters, and components must be excluded
    assert!(!sit.contains(&3006), "Boot 3006 must not be in situational");
    assert!(
        !sit.contains(&1055),
        "Starter 1055 must not be in situational"
    );
    assert!(
        !sit.contains(&1038),
        "Component 1038 must not be in situational"
    );

    // Valid situational items must be present
    assert_eq!(sit, vec![3046, 3085, 3078, 6676, 3153]);
}

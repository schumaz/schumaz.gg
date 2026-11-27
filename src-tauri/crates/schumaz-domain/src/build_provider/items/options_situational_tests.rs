use super::super::classification::TEST_CLASSIFICATION_MUTEX;
use super::*;
use crate::build_provider::models::ItemClassificationCache;
use crate::build_provider::*;

#[test]
fn test_option_situational_filters_by_archetype() {
    let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
    let mut cache = ItemClassificationCache::default();
    cache.item_tags.insert(
        6697,
        vec!["Damage".to_string(), "ArmorPenetration".to_string()],
    );
    cache.item_tags.insert(
        6676,
        vec!["Damage".to_string(), "CriticalStrike".to_string()],
    );
    cache.item_tags.insert(
        3031,
        vec!["Damage".to_string(), "CriticalStrike".to_string()],
    );
    cache.item_tags.insert(
        3036,
        vec!["Damage".to_string(), "ArmorPenetration".to_string()],
    );
    cache
        .item_tags
        .insert(3026, vec!["Damage".to_string(), "Armor".to_string()]);
    cache.item_tags.insert(
        3142,
        vec!["Damage".to_string(), "ArmorPenetration".to_string()],
    );
    cache.item_tags.insert(
        3046,
        vec!["Damage".to_string(), "CriticalStrike".to_string()],
    );
    cache
        .item_tags
        .insert(3078, vec!["Damage".to_string(), "AttackSpeed".to_string()]);
    cache
        .item_tags
        .insert(3153, vec!["Damage".to_string(), "LifeSteal".to_string()]);
    cache
        .item_tags
        .insert(3089, vec!["SpellDamage".to_string()]);
    cache
        .item_tags
        .insert(3157, vec!["SpellDamage".to_string()]);
    cache.completed.extend(vec![
        6697, 6676, 3031, 3036, 3026, 3142, 3046, 3078, 3153, 3089, 3157,
    ]);
    set_dynamic_item_classifications(cache);

    let ad_core = vec![6697, 6676, 3031];
    let ad_final = vec![6697, 6676, 3031, 3036, 3026, 3142];
    let candidates = vec![
        ItemStatEntry {
            ids: vec![3089],
            play: 10000,
            win: 5000,
            pick_rate: 0.5,
        },
        ItemStatEntry {
            ids: vec![3157],
            play: 8000,
            win: 4000,
            pick_rate: 0.4,
        },
        ItemStatEntry {
            ids: vec![3046],
            play: 3000,
            win: 1500,
            pick_rate: 0.3,
        },
        ItemStatEntry {
            ids: vec![3078],
            play: 2000,
            win: 1000,
            pick_rate: 0.2,
        },
        ItemStatEntry {
            ids: vec![3153],
            play: 1000,
            win: 500,
            pick_rate: 0.1,
        },
    ];

    let sit = build_situational_items_for_option(
        &ad_core,
        &ad_final,
        &[],
        &[],
        &[],
        &candidates,
        &[],
        &[],
        "AD",
    );

    assert!(
        !sit.contains(&3089),
        "AP item Rabadon must be excluded from AD build"
    );
    assert!(
        !sit.contains(&3157),
        "AP item Zhonya must be excluded from AD build"
    );
    assert!(sit.contains(&3046));
    assert!(sit.contains(&3078));
    assert!(sit.contains(&3153));
}

#[test]
fn test_option_situational_hybrid_keeps_both() {
    let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
    let mut cache = ItemClassificationCache::default();
    cache.item_tags.insert(
        3115,
        vec!["SpellDamage".to_string(), "AttackSpeed".to_string()],
    );
    cache
        .item_tags
        .insert(3124, vec!["Damage".to_string(), "SpellDamage".to_string()]);
    cache
        .item_tags
        .insert(3089, vec!["SpellDamage".to_string()]);
    cache.item_tags.insert(
        3046,
        vec!["Damage".to_string(), "CriticalStrike".to_string()],
    );
    cache
        .item_tags
        .insert(3157, vec!["SpellDamage".to_string()]);
    cache
        .item_tags
        .insert(3072, vec!["Damage".to_string(), "LifeSteal".to_string()]);
    cache
        .completed
        .extend(vec![3115, 3124, 3089, 3046, 3157, 3072]);
    set_dynamic_item_classifications(cache);

    let hybrid_core = vec![3115, 3124, 3089];
    let hybrid_final = vec![3115, 3124, 3089, 3157];
    let candidates = vec![
        ItemStatEntry {
            ids: vec![3046],
            play: 2000,
            win: 1000,
            pick_rate: 0.3,
        },
        ItemStatEntry {
            ids: vec![3072],
            play: 1500,
            win: 750,
            pick_rate: 0.2,
        },
    ];

    let sit = build_situational_items_for_option(
        &hybrid_core,
        &hybrid_final,
        &[],
        &[],
        &[],
        &candidates,
        &[],
        &[],
        "HYBRID",
    );

    assert!(sit.contains(&3046), "Hybrid build should keep AD item PD");
    assert!(sit.contains(&3072), "Hybrid build should keep AD item BT");
}

#[test]
fn test_option_situational_min_5_sorted_by_play() {
    let core = vec![1, 2, 3];
    let final_items = vec![1, 2, 3, 4, 5, 6];
    let fourth = vec![
        ItemStatEntry {
            ids: vec![101],
            play: 100,
            win: 50,
            pick_rate: 0.1,
        },
        ItemStatEntry {
            ids: vec![102],
            play: 500,
            win: 250,
            pick_rate: 0.2,
        },
    ];
    let fifth = vec![ItemStatEntry {
        ids: vec![103],
        play: 300,
        win: 150,
        pick_rate: 0.15,
    }];
    let sixth = vec![ItemStatEntry {
        ids: vec![104],
        play: 800,
        win: 400,
        pick_rate: 0.3,
    }];
    let candidates = vec![
        ItemStatEntry {
            ids: vec![105],
            play: 200,
            win: 100,
            pick_rate: 0.1,
        },
        ItemStatEntry {
            ids: vec![106],
            play: 400,
            win: 200,
            pick_rate: 0.18,
        },
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

    // Should have 6 items sorted by play count descending:
    // 104 (800) > 102 (500) > 106 (400) > 103 (300) > 105 (200) > 101 (100)
    assert_eq!(sit, vec![104, 102, 106, 103, 105, 101]);
}

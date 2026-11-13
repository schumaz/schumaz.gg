use super::*;
use crate::build_provider::*;
use super::super::classification::TEST_CLASSIFICATION_MUTEX;
use crate::build_provider::models::ItemClassificationCache;

#[test]
fn test_shaco_ad_picks_ldr_ga_youmuu_sequentially_per_slot() {
    let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
    let mut cache = ItemClassificationCache::default();
    cache.item_tags.insert(6697, vec!["Damage".to_string(), "ArmorPenetration".to_string()]); // Cyclosword
    cache.item_tags.insert(6676, vec!["Damage".to_string(), "CriticalStrike".to_string(), "ArmorPenetration".to_string()]); // Collector
    cache.item_tags.insert(3031, vec!["Damage".to_string(), "CriticalStrike".to_string()]); // Infinity Edge
    cache.item_tags.insert(3036, vec!["Damage".to_string(), "ArmorPenetration".to_string()]); // Lord Dominik's Regards
    cache.item_tags.insert(3026, vec!["Damage".to_string(), "Armor".to_string()]); // Guardian Angel
    cache.item_tags.insert(3142, vec!["Damage".to_string(), "ArmorPenetration".to_string(), "NonbootsMovement".to_string()]); // Youmuu's Ghostblade
    cache.item_tags.insert(3179, vec!["Damage".to_string(), "ArmorPenetration".to_string(), "AbilityHaste".to_string()]); // Umbral Glaive
    cache.item_tags.insert(3089, vec!["SpellDamage".to_string()]); // Rabadon
    cache.item_tags.insert(3157, vec!["SpellDamage".to_string(), "Armor".to_string()]); // Zhonya
    cache.completed.extend(vec![6697, 6676, 3031, 3036, 3026, 3142, 3179, 3089, 3157]);
    set_dynamic_item_classifications(cache);

    let core = vec![6697, 6676, 3031];
    let fourth = vec![
        ItemStatEntry { ids: vec![3036], play: 2790, win: 1578, pick_rate: 56.56 }, // LDR -> 1st for 4th slot
        ItemStatEntry { ids: vec![3031], play: 2266, win: 1304, pick_rate: 57.55 }, // IE (already in core)
    ];
    let fifth = vec![
        ItemStatEntry { ids: vec![3026], play: 567, win: 359, pick_rate: 63.32 }, // GA -> 1st for 5th slot
        ItemStatEntry { ids: vec![3031], play: 476, win: 288, pick_rate: 60.50 }, // IE (already in core)
        ItemStatEntry { ids: vec![3089], play: 370, win: 244, pick_rate: 65.95 }, // Rabadon (blocked: AP offensive!)
        ItemStatEntry { ids: vec![3036], play: 314, win: 205, pick_rate: 65.29 }, // LDR (already in 4th slot)
        ItemStatEntry { ids: vec![3157], play: 260, win: 142, pick_rate: 54.62 }, // Zhonya (blocked: AP offensive!)
    ];
    let sixth = vec![
        ItemStatEntry { ids: vec![3142], play: 110, win: 71, pick_rate: 64.55 }, // Youmuu -> 1st for 6th slot
        ItemStatEntry { ids: vec![3026], play: 44, win: 25, pick_rate: 56.82 }, // GA (already in 5th slot)
    ];
    let candidates = vec![
        ItemStatEntry { ids: vec![3179], play: 5000, win: 2500, pick_rate: 0.5 }, // Umbral Glaive in candidate pool
    ];

    let final_6 = build_cohesive_final_build_for_core(
        &core,
        &fourth,
        &fifth,
        &sixth,
        &candidates,
        "AD",
    );

    // Strict sequential slot selection:
    // Slot 4: 3036 (LDR)
    // Slot 5: 3026 (GA)
    // Slot 6: 3142 (Youmuu's Ghostblade)
    // Umbral Glaive (3179) must NOT displace slot 6!
    // Rabadon (3089) and Zhonya (3157) must NOT be picked!
    assert_eq!(final_6, vec![6697, 6676, 3031, 3036, 3026, 3142]);
}

#[test]
fn test_vayne_adc_accepts_jaksho_and_randuins_universal_defense() {
    let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
    let mut cache = ItemClassificationCache::default();
    cache.item_tags.insert(6672, vec!["Damage".to_string(), "AttackSpeed".to_string()]); // Kraken
    cache.item_tags.insert(3124, vec!["Damage".to_string(), "AttackSpeed".to_string(), "OnHit".to_string()]); // Guinsoo
    cache.item_tags.insert(3302, vec!["Damage".to_string(), "AttackSpeed".to_string(), "ArmorPenetration".to_string(), "MagicPenetration".to_string()]); // Terminus
    cache.item_tags.insert(3153, vec!["Damage".to_string(), "AttackSpeed".to_string(), "LifeSteal".to_string()]); // BotRK
    cache.item_tags.insert(6665, vec!["Armor".to_string(), "SpellBlock".to_string(), "Health".to_string(), "AbilityHaste".to_string()]); // Jak'Sho
    cache.item_tags.insert(3143, vec!["Armor".to_string(), "Health".to_string()]); // Randuin's Omen
    cache.item_tags.insert(3089, vec!["SpellDamage".to_string()]); // Rabadon (offensive AP)
    cache.completed.extend(vec![6672, 3124, 3302, 3153, 6665, 3143, 3089]);
    set_dynamic_item_classifications(cache);

    let core = vec![6672, 3124, 3302];
    let fourth = vec![
        ItemStatEntry { ids: vec![3153], play: 3000, win: 1600, pick_rate: 0.6 },
    ];
    let fifth = vec![
        ItemStatEntry { ids: vec![6665], play: 2500, win: 1350, pick_rate: 0.5 }, // Jak'Sho
    ];
    let sixth = vec![
        ItemStatEntry { ids: vec![3089], play: 5000, win: 2500, pick_rate: 0.7 }, // Rabadon (blocked!)
        ItemStatEntry { ids: vec![3143], play: 1200, win: 650, pick_rate: 0.25 }, // Randuin
    ];

    let final_6 = build_cohesive_final_build_for_core(
        &core,
        &fourth,
        &fifth,
        &sixth,
        &[],
        "AD",
    );

    assert_eq!(final_6, vec![6672, 3124, 3302, 3153, 6665, 3143]);
    assert!(!final_6.contains(&3089), "Rabadon must be blocked from AD build");
}

#[test]
fn test_ezreal_mana_synergy_recognizes_frozen_heart() {
    let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
    let mut cache = ItemClassificationCache::default();
    cache.item_tags.insert(3078, vec!["Damage".to_string(), "AttackSpeed".to_string(), "Health".to_string(), "AbilityHaste".to_string()]); // Triforce
    cache.item_tags.insert(3004, vec!["Damage".to_string(), "Mana".to_string(), "AbilityHaste".to_string()]); // Manamune
    cache.item_tags.insert(3036, vec!["Damage".to_string(), "ArmorPenetration".to_string()]); // LDR
    cache.item_tags.insert(3072, vec!["Damage".to_string(), "LifeSteal".to_string(), "CriticalStrike".to_string()]); // BT
    cache.item_tags.insert(3110, vec!["Armor".to_string(), "Mana".to_string(), "AbilityHaste".to_string()]); // Frozen Heart
    cache.item_tags.insert(3153, vec!["Damage".to_string(), "AttackSpeed".to_string(), "LifeSteal".to_string()]); // BotRK
    cache.completed.extend(vec![3078, 3004, 3036, 3072, 3110, 3153]);
    set_dynamic_item_classifications(cache);

    let core = vec![3078, 3004, 3036];
    let fourth = vec![
        ItemStatEntry { ids: vec![3072], play: 2000, win: 1100, pick_rate: 0.4 },
    ];
    let fifth = vec![
        ItemStatEntry { ids: vec![3110], play: 1500, win: 850, pick_rate: 0.3 }, // Frozen Heart
    ];
    let sixth = vec![
        ItemStatEntry { ids: vec![3153], play: 1000, win: 550, pick_rate: 0.2 },
    ];

    let final_6 = build_cohesive_final_build_for_core(
        &core,
        &fourth,
        &fifth,
        &sixth,
        &[],
        "AD",
    );

    assert_eq!(final_6, vec![3078, 3004, 3036, 3072, 3110, 3153]);
    assert!(final_6.contains(&3110), "Frozen Heart must be recognized via Mana synergy");
}

#[test]
fn test_option_situational_excludes_core_final_boots_components() {
    let core = vec![3031, 3094, 3036];
    let final_items = vec![3031, 3094, 3036, 3026, 3072, 3139];
    let fourth = vec![ItemStatEntry { ids: vec![3026], play: 1000, win: 500, pick_rate: 0.1 }];
    let fifth = vec![ItemStatEntry { ids: vec![3072], play: 800, win: 400, pick_rate: 0.1 }];
    let sixth = vec![ItemStatEntry { ids: vec![3139], play: 600, win: 300, pick_rate: 0.1 }];
    let candidates = vec![
        ItemStatEntry { ids: vec![3006], play: 5000, win: 2500, pick_rate: 0.5 }, // Greaves (boot) -> exclude
        ItemStatEntry { ids: vec![1055], play: 4000, win: 2000, pick_rate: 0.4 }, // Doran's Blade (starter) -> exclude
        ItemStatEntry { ids: vec![1038], play: 3000, win: 1500, pick_rate: 0.3 }, // BF Sword (component) -> exclude
        ItemStatEntry { ids: vec![3046], play: 2500, win: 1200, pick_rate: 0.25 }, // Phantom Dancer -> VALID
        ItemStatEntry { ids: vec![3085], play: 2000, win: 1000, pick_rate: 0.2 }, // Runaan's Hurricane -> VALID
        ItemStatEntry { ids: vec![3078], play: 1500, win: 750, pick_rate: 0.15 }, // Trinity Force -> VALID
        ItemStatEntry { ids: vec![6676], play: 1200, win: 600, pick_rate: 0.12 }, // Collector -> VALID
        ItemStatEntry { ids: vec![3153], play: 1000, win: 500, pick_rate: 0.1 }, // Blade of Ruined King -> VALID
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
        assert!(!sit.contains(id), "Core item {id} must not be in situational");
    }
    for id in &final_items {
        assert!(!sit.contains(id), "Final item {id} must not be in situational");
    }
    // Boots, starters, and components must be excluded
    assert!(!sit.contains(&3006), "Boot 3006 must not be in situational");
    assert!(!sit.contains(&1055), "Starter 1055 must not be in situational");
    assert!(!sit.contains(&1038), "Component 1038 must not be in situational");

    // Valid situational items must be present
    assert_eq!(sit, vec![3046, 3085, 3078, 6676, 3153]);
}

#[test]
fn test_option_situational_filters_by_archetype() {
    let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
    let mut cache = ItemClassificationCache::default();
    cache.item_tags.insert(6697, vec!["Damage".to_string(), "ArmorPenetration".to_string()]);
    cache.item_tags.insert(6676, vec!["Damage".to_string(), "CriticalStrike".to_string()]);
    cache.item_tags.insert(3031, vec!["Damage".to_string(), "CriticalStrike".to_string()]);
    cache.item_tags.insert(3036, vec!["Damage".to_string(), "ArmorPenetration".to_string()]);
    cache.item_tags.insert(3026, vec!["Damage".to_string(), "Armor".to_string()]);
    cache.item_tags.insert(3142, vec!["Damage".to_string(), "ArmorPenetration".to_string()]);
    cache.item_tags.insert(3046, vec!["Damage".to_string(), "CriticalStrike".to_string()]);
    cache.item_tags.insert(3078, vec!["Damage".to_string(), "AttackSpeed".to_string()]);
    cache.item_tags.insert(3153, vec!["Damage".to_string(), "LifeSteal".to_string()]);
    cache.item_tags.insert(3089, vec!["SpellDamage".to_string()]);
    cache.item_tags.insert(3157, vec!["SpellDamage".to_string()]);
    cache.completed.extend(vec![6697, 6676, 3031, 3036, 3026, 3142, 3046, 3078, 3153, 3089, 3157]);
    set_dynamic_item_classifications(cache);

    let ad_core = vec![6697, 6676, 3031];
    let ad_final = vec![6697, 6676, 3031, 3036, 3026, 3142];
    let candidates = vec![
        ItemStatEntry { ids: vec![3089], play: 10000, win: 5000, pick_rate: 0.5 },
        ItemStatEntry { ids: vec![3157], play: 8000, win: 4000, pick_rate: 0.4 },
        ItemStatEntry { ids: vec![3046], play: 3000, win: 1500, pick_rate: 0.3 },
        ItemStatEntry { ids: vec![3078], play: 2000, win: 1000, pick_rate: 0.2 },
        ItemStatEntry { ids: vec![3153], play: 1000, win: 500, pick_rate: 0.1 },
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

    assert!(!sit.contains(&3089), "AP item Rabadon must be excluded from AD build");
    assert!(!sit.contains(&3157), "AP item Zhonya must be excluded from AD build");
    assert!(sit.contains(&3046));
    assert!(sit.contains(&3078));
    assert!(sit.contains(&3153));
}

#[test]
fn test_option_situational_hybrid_keeps_both() {
    let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
    let mut cache = ItemClassificationCache::default();
    cache.item_tags.insert(3115, vec!["SpellDamage".to_string(), "AttackSpeed".to_string()]);
    cache.item_tags.insert(3124, vec!["Damage".to_string(), "SpellDamage".to_string()]);
    cache.item_tags.insert(3089, vec!["SpellDamage".to_string()]);
    cache.item_tags.insert(3046, vec!["Damage".to_string(), "CriticalStrike".to_string()]);
    cache.item_tags.insert(3157, vec!["SpellDamage".to_string()]);
    cache.item_tags.insert(3072, vec!["Damage".to_string(), "LifeSteal".to_string()]);
    cache.completed.extend(vec![3115, 3124, 3089, 3046, 3157, 3072]);
    set_dynamic_item_classifications(cache);

    let hybrid_core = vec![3115, 3124, 3089];
    let hybrid_final = vec![3115, 3124, 3089, 3157];
    let candidates = vec![
        ItemStatEntry { ids: vec![3046], play: 2000, win: 1000, pick_rate: 0.3 },
        ItemStatEntry { ids: vec![3072], play: 1500, win: 750, pick_rate: 0.2 },
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
        ItemStatEntry { ids: vec![101], play: 100, win: 50, pick_rate: 0.1 },
        ItemStatEntry { ids: vec![102], play: 500, win: 250, pick_rate: 0.2 },
    ];
    let fifth = vec![
        ItemStatEntry { ids: vec![103], play: 300, win: 150, pick_rate: 0.15 },
    ];
    let sixth = vec![
        ItemStatEntry { ids: vec![104], play: 800, win: 400, pick_rate: 0.3 },
    ];
    let candidates = vec![
        ItemStatEntry { ids: vec![105], play: 200, win: 100, pick_rate: 0.1 },
        ItemStatEntry { ids: vec![106], play: 400, win: 200, pick_rate: 0.18 },
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

#[test]
fn test_extract_top_core_and_final_build_options() {
    let core_entries = vec![
        ItemStatEntry {
            ids: vec![3118, 4645, 3089],
            play: 15000,
            win: 8000,
            pick_rate: 60.0,
        },
        ItemStatEntry {
            ids: vec![3118, 3157, 3089],
            play: 5000,
            win: 2600,
            pick_rate: 20.0,
        },
        ItemStatEntry {
            ids: vec![3118, 4645, 3135],
            play: 2000,
            win: 1050,
            pick_rate: 8.0,
        },
        ItemStatEntry {
            ids: vec![3118, 3165, 3089],
            play: 500,
            win: 250,
            pick_rate: 2.0,
        },
    ];
    let candidates = vec![3118, 4645, 3089, 3157, 3135, 3165, 4629, 3102];
    let core_opts = extract_top_core_build_options(&core_entries, &candidates, "AP");
    assert_eq!(core_opts.len(), 3);
    assert_eq!(core_opts[0].item_ids, vec![3118, 4645, 3089]);
    assert_eq!(core_opts[0].play, 15000);
    assert_eq!(core_opts[1].item_ids, vec![3118, 3157, 3089]);
    assert_eq!(core_opts[1].play, 5000);
    assert_eq!(core_opts[2].item_ids, vec![3118, 4645, 3135]);
    assert_eq!(core_opts[2].play, 2000);

    let fourth = vec![
        ItemStatEntry {
            ids: vec![3157],
            play: 10000,
            win: 5500,
            pick_rate: 40.0,
        },
        ItemStatEntry {
            ids: vec![3135],
            play: 3000,
            win: 1600,
            pick_rate: 15.0,
        },
    ];
    let fifth = vec![ItemStatEntry {
        ids: vec![3135],
        play: 8000,
        win: 4200,
        pick_rate: 35.0,
    }];
    let sixth = vec![
        ItemStatEntry {
            ids: vec![4629],
            play: 6000,
            win: 3100,
            pick_rate: 30.0,
        },
        ItemStatEntry {
            ids: vec![3102],
            play: 2000,
            win: 1000,
            pick_rate: 10.0,
        },
    ];

    let final_opts = build_3_final_build_options(
        &core_opts[0].item_ids,
        &fourth,
        &fifth,
        &sixth,
        &candidates,
        "AP",
    );
    assert_eq!(final_opts.len(), 3);
    for opt in &final_opts {
        assert_eq!(opt.item_ids.len(), 6);
        assert_eq!(&opt.item_ids[0..3], &[3118, 4645, 3089]);
    }
}

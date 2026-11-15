use super::super::classification::TEST_CLASSIFICATION_MUTEX;
use super::*;
use crate::build_provider::models::ItemClassificationCache;
use crate::build_provider::*;

#[test]
fn test_shaco_ad_picks_ldr_ga_youmuu_sequentially_per_slot() {
    let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
    let mut cache = ItemClassificationCache::default();
    cache.item_tags.insert(
        6697,
        vec!["Damage".to_string(), "ArmorPenetration".to_string()],
    ); // Cyclosword
    cache.item_tags.insert(
        6676,
        vec![
            "Damage".to_string(),
            "CriticalStrike".to_string(),
            "ArmorPenetration".to_string(),
        ],
    ); // Collector
    cache.item_tags.insert(
        3031,
        vec!["Damage".to_string(), "CriticalStrike".to_string()],
    ); // Infinity Edge
    cache.item_tags.insert(
        3036,
        vec!["Damage".to_string(), "ArmorPenetration".to_string()],
    ); // Lord Dominik's Regards
    cache
        .item_tags
        .insert(3026, vec!["Damage".to_string(), "Armor".to_string()]); // Guardian Angel
    cache.item_tags.insert(
        3142,
        vec![
            "Damage".to_string(),
            "ArmorPenetration".to_string(),
            "NonbootsMovement".to_string(),
        ],
    ); // Youmuu's Ghostblade
    cache.item_tags.insert(
        3179,
        vec![
            "Damage".to_string(),
            "ArmorPenetration".to_string(),
            "AbilityHaste".to_string(),
        ],
    ); // Umbral Glaive
    cache
        .item_tags
        .insert(3089, vec!["SpellDamage".to_string()]); // Rabadon
    cache
        .item_tags
        .insert(3157, vec!["SpellDamage".to_string(), "Armor".to_string()]); // Zhonya
    cache
        .completed
        .extend(vec![6697, 6676, 3031, 3036, 3026, 3142, 3179, 3089, 3157]);
    set_dynamic_item_classifications(cache);

    let core = vec![6697, 6676, 3031];
    let fourth = vec![
        ItemStatEntry {
            ids: vec![3036],
            play: 2790,
            win: 1578,
            pick_rate: 56.56,
        }, // LDR -> 1st for 4th slot
        ItemStatEntry {
            ids: vec![3031],
            play: 2266,
            win: 1304,
            pick_rate: 57.55,
        }, // IE (already in core)
    ];
    let fifth = vec![
        ItemStatEntry {
            ids: vec![3026],
            play: 567,
            win: 359,
            pick_rate: 63.32,
        }, // GA -> 1st for 5th slot
        ItemStatEntry {
            ids: vec![3031],
            play: 476,
            win: 288,
            pick_rate: 60.50,
        }, // IE (already in core)
        ItemStatEntry {
            ids: vec![3089],
            play: 370,
            win: 244,
            pick_rate: 65.95,
        }, // Rabadon (blocked: AP offensive!)
        ItemStatEntry {
            ids: vec![3036],
            play: 314,
            win: 205,
            pick_rate: 65.29,
        }, // LDR (already in 4th slot)
        ItemStatEntry {
            ids: vec![3157],
            play: 260,
            win: 142,
            pick_rate: 54.62,
        }, // Zhonya (blocked: AP offensive!)
    ];
    let sixth = vec![
        ItemStatEntry {
            ids: vec![3142],
            play: 110,
            win: 71,
            pick_rate: 64.55,
        }, // Youmuu -> 1st for 6th slot
        ItemStatEntry {
            ids: vec![3026],
            play: 44,
            win: 25,
            pick_rate: 56.82,
        }, // GA (already in 5th slot)
    ];
    let candidates = vec![
        ItemStatEntry {
            ids: vec![3179],
            play: 5000,
            win: 2500,
            pick_rate: 0.5,
        }, // Umbral Glaive in candidate pool
    ];

    let final_6 =
        build_cohesive_final_build_for_core(&core, &fourth, &fifth, &sixth, &candidates, "AD");

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
    cache
        .item_tags
        .insert(6672, vec!["Damage".to_string(), "AttackSpeed".to_string()]); // Kraken
    cache.item_tags.insert(
        3124,
        vec![
            "Damage".to_string(),
            "AttackSpeed".to_string(),
            "OnHit".to_string(),
        ],
    ); // Guinsoo
    cache.item_tags.insert(
        3302,
        vec![
            "Damage".to_string(),
            "AttackSpeed".to_string(),
            "ArmorPenetration".to_string(),
            "MagicPenetration".to_string(),
        ],
    ); // Terminus
    cache.item_tags.insert(
        3153,
        vec![
            "Damage".to_string(),
            "AttackSpeed".to_string(),
            "LifeSteal".to_string(),
        ],
    ); // BotRK
    cache.item_tags.insert(
        6665,
        vec![
            "Armor".to_string(),
            "SpellBlock".to_string(),
            "Health".to_string(),
            "AbilityHaste".to_string(),
        ],
    ); // Jak'Sho
    cache
        .item_tags
        .insert(3143, vec!["Armor".to_string(), "Health".to_string()]); // Randuin's Omen
    cache
        .item_tags
        .insert(3089, vec!["SpellDamage".to_string()]); // Rabadon (offensive AP)
    cache
        .completed
        .extend(vec![6672, 3124, 3302, 3153, 6665, 3143, 3089]);
    set_dynamic_item_classifications(cache);

    let core = vec![6672, 3124, 3302];
    let fourth = vec![ItemStatEntry {
        ids: vec![3153],
        play: 3000,
        win: 1600,
        pick_rate: 0.6,
    }];
    let fifth = vec![
        ItemStatEntry {
            ids: vec![6665],
            play: 2500,
            win: 1350,
            pick_rate: 0.5,
        }, // Jak'Sho
    ];
    let sixth = vec![
        ItemStatEntry {
            ids: vec![3089],
            play: 5000,
            win: 2500,
            pick_rate: 0.7,
        }, // Rabadon (blocked!)
        ItemStatEntry {
            ids: vec![3143],
            play: 1200,
            win: 650,
            pick_rate: 0.25,
        }, // Randuin
    ];

    let final_6 = build_cohesive_final_build_for_core(&core, &fourth, &fifth, &sixth, &[], "AD");

    assert_eq!(final_6, vec![6672, 3124, 3302, 3153, 6665, 3143]);
    assert!(
        !final_6.contains(&3089),
        "Rabadon must be blocked from AD build"
    );
}

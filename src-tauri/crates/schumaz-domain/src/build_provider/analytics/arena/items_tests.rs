use super::items::build_arena_item_blocks_and_options;
use crate::build_provider::items::{TEST_CLASSIFICATION_MUTEX, set_dynamic_item_classifications};
use crate::build_provider::models::{ItemBlockKind, ItemClassificationCache, ItemStatEntry};
use std::collections::HashSet;

pub(super) fn install_arena_item_classifications() {
    let mut cache = ItemClassificationCache::default();

    for b in [223006, 223008, 223020, 223158, 223047, 223111] {
        cache.boots.insert(b);
        cache.map30_items.insert(b);
    }
    for p in [443090, 447109, 443054, 447119, 447103, 443056, 447115] {
        cache.arena_prismatics.insert(p);
        cache.map30_items.insert(p);
    }
    for c in [220000, 220007, 220012, 2142] {
        cache.arena_consumables.insert(c);
        cache.map30_items.insert(c);
    }

    let legendaries: &[(i32, &[&str])] = &[
        (226617, &["SpellDamage", "AbilityHaste", "Health"]),
        (223504, &["SpellDamage", "AttackSpeed"]),
        (223107, &["Health", "AbilityHaste"]),
        (226620, &["SpellDamage", "AbilityHaste"]),
        (223089, &["SpellDamage"]),
        (226653, &["SpellDamage", "Health"]),
        (222503, &["SpellDamage", "Mana", "AbilityHaste"]),
        (223135, &["SpellDamage", "MagicPenetration"]),
        (223157, &["SpellDamage", "Armor"]),
        (224645, &["SpellDamage", "MagicPenetration"]),
        (223153, &["Damage", "AttackSpeed", "LifeSteal"]),
        (223124, &["Damage", "AttackSpeed", "OnHit"]),
        (226672, &["Damage", "AttackSpeed", "OnHit"]),
        (223091, &["AttackSpeed", "SpellBlock", "OnHit"]),
        (226676, &["Damage", "CriticalStrike", "ArmorPenetration"]),
        (223031, &["Damage", "CriticalStrike"]),
        (223036, &["Damage", "CriticalStrike", "ArmorPenetration"]),
        (226699, &["Damage", "ArmorPenetration", "AbilityHaste"]),
        (226701, &["Damage", "ArmorPenetration"]),
        (223814, &["Damage", "Health", "ArmorPenetration"]),
        (223004, &["Damage", "Mana", "AbilityHaste"]),
        (223119, &["Health", "Mana", "AbilityHaste"]),
        (223078, &["Damage", "Health", "AttackSpeed"]),
        (4017, &["Damage", "ArmorPenetration", "AbilityHaste"]),
        (3430, &["Health", "Armor", "SpellBlock"]),
        (447111, &["Damage", "Health"]),
    ];

    for &(id, tags) in legendaries {
        cache.completed.insert(id);
        cache.map30_items.insert(id);
        cache
            .item_tags
            .insert(id, tags.iter().map(|s| s.to_string()).collect());
    }

    cache.map30_items.insert(223042);
    cache.map30_items.insert(223121);
    cache.evolved_fallbacks.insert(223042, 223004);
    cache.evolved_fallbacks.insert(223121, 223119);

    set_dynamic_item_classifications(cache);
}

#[test]
fn test_arena_items_filter_troll_boots_anvils_prismatics_and_preserve_447111_and_4017() {
    let _lock = TEST_CLASSIFICATION_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    install_arena_item_classifications();

    let boots = vec![
        ItemStatEntry {
            ids: vec![223158],
            play: 9500,
            win: 5100,
            pick_rate: 0.95,
        },
        ItemStatEntry {
            ids: vec![223020],
            play: 440,
            win: 210,
            pick_rate: 0.044,
        },
        ItemStatEntry {
            ids: vec![223006],
            play: 60,
            win: 25,
            pick_rate: 0.006,
        },
    ];
    let prismatics = vec![
        ItemStatEntry {
            ids: vec![443090],
            play: 4000,
            win: 2400,
            pick_rate: 0.20,
        },
        ItemStatEntry {
            ids: vec![447109],
            play: 3200,
            win: 1800,
            pick_rate: 0.16,
        },
    ];
    let cores = vec![
        ItemStatEntry {
            ids: vec![223153, 223124, 220007, 226672],
            play: 3000,
            win: 1700,
            pick_rate: 0.3,
        },
        ItemStatEntry {
            ids: vec![223153, 226672, 223124],
            play: 1200,
            win: 680,
            pick_rate: 0.12,
        },
        ItemStatEntry {
            ids: vec![447111, 4017, 223042],
            play: 1500,
            win: 900,
            pick_rate: 0.15,
        },
    ];
    let last = vec![
        ItemStatEntry {
            ids: vec![443090],
            play: 9000,
            win: 5000,
            pick_rate: 0.4,
        },
        ItemStatEntry {
            ids: vec![223158],
            play: 8500,
            win: 4500,
            pick_rate: 0.35,
        },
        ItemStatEntry {
            ids: vec![220007],
            play: 7000,
            win: 3500,
            pick_rate: 0.3,
        },
        ItemStatEntry {
            ids: vec![220012],
            play: 6500,
            win: 3400,
            pick_rate: 0.28,
        },
        ItemStatEntry {
            ids: vec![447111],
            play: 5000,
            win: 2900,
            pick_rate: 0.25,
        },
        ItemStatEntry {
            ids: vec![4017],
            play: 4500,
            win: 2600,
            pick_rate: 0.22,
        },
        ItemStatEntry {
            ids: vec![223091],
            play: 4000,
            win: 2200,
            pick_rate: 0.2,
        },
        ItemStatEntry {
            ids: vec![223121],
            play: 3500,
            win: 1900,
            pick_rate: 0.18,
        },
        ItemStatEntry {
            ids: vec![223078],
            play: 3000,
            win: 1600,
            pick_rate: 0.15,
        },
    ];

    let (blocks, options) = build_arena_item_blocks_and_options(&boots, &prismatics, &cores, &last);

    assert_eq!(blocks.len(), 5);
    assert_eq!(blocks[0].kind, ItemBlockKind::Prismatic);
    assert_eq!(blocks[0].item_ids, vec![443090, 447109]);
    assert_eq!(blocks[2].kind, ItemBlockKind::Boots);
    assert_eq!(blocks[2].item_ids, vec![223158, 223020]);

    assert_eq!(options.len(), 2);
    assert_eq!(options[0].core_item_ids, vec![223153, 223124, 226672]);
    assert_eq!(options[0].play, 4200);
    assert!((options[0].pick_rate - 73.7).abs() < 0.2);
    assert!((options[1].pick_rate - 26.3).abs() < 0.2);
    assert_eq!(options[1].core_item_ids, vec![447111, 4017, 223004]);

    let forbidden: HashSet<i32> = [
        443090, 447109, 223158, 220007, 220012, 1055, 1056, 2003, 3119,
    ]
    .into_iter()
    .collect();
    for opt in &options {
        for &id in opt
            .final_item_ids
            .iter()
            .chain(opt.situational_item_ids.iter())
        {
            assert!(
                !forbidden.contains(&id),
                "Forbidden item {} leaked into Arena build",
                id
            );
        }
    }
    assert!(options[0].final_item_ids.contains(&447111));
    assert!(options[0].final_item_ids.contains(&4017));
    assert!(
        options[0].final_item_ids.contains(&223119)
            || options[0].situational_item_ids.contains(&223119)
    );
}

#[test]
fn test_arena_shaco_ap_and_ad_build_options_stay_archetype_cohesive() {
    let _lock = TEST_CLASSIFICATION_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    install_arena_item_classifications();

    let cores = vec![
        ItemStatEntry {
            ids: vec![226653, 222503, 223089],
            play: 3000,
            win: 1650,
            pick_rate: 0.3,
        },
        ItemStatEntry {
            ids: vec![222503, 226653, 223089],
            play: 1000,
            win: 550,
            pick_rate: 0.1,
        },
        ItemStatEntry {
            ids: vec![226676, 223031, 223036],
            play: 2200,
            win: 1200,
            pick_rate: 0.22,
        },
    ];
    let last = vec![
        ItemStatEntry {
            ids: vec![223135],
            play: 4000,
            win: 2100,
            pick_rate: 0.2,
        },
        ItemStatEntry {
            ids: vec![223157],
            play: 3500,
            win: 1800,
            pick_rate: 0.18,
        },
        ItemStatEntry {
            ids: vec![224645],
            play: 3200,
            win: 1700,
            pick_rate: 0.16,
        },
        ItemStatEntry {
            ids: vec![226699],
            play: 3800,
            win: 2000,
            pick_rate: 0.19,
        },
        ItemStatEntry {
            ids: vec![226701],
            play: 3400,
            win: 1800,
            pick_rate: 0.17,
        },
        ItemStatEntry {
            ids: vec![223814],
            play: 3000,
            win: 1500,
            pick_rate: 0.15,
        },
    ];

    let (_, options) = build_arena_item_blocks_and_options(&[], &[], &cores, &last);
    assert_eq!(options.len(), 2);

    let ap_build = &options[0];
    assert_eq!(ap_build.core_item_ids, vec![226653, 222503, 223089]);
    for ad_id in [226699, 226701, 223814, 226676, 223031, 223036] {
        assert!(!ap_build.final_item_ids.contains(&ad_id));
        assert!(!ap_build.situational_item_ids.contains(&ad_id));
    }
    assert!(ap_build.final_item_ids.contains(&223135));

    let ad_build = &options[1];
    assert_eq!(ad_build.core_item_ids, vec![226676, 223031, 223036]);
    for ap_id in [223135, 223157, 224645, 226653, 222503, 223089] {
        assert!(!ad_build.final_item_ids.contains(&ap_id));
        assert!(!ad_build.situational_item_ids.contains(&ap_id));
    }
    assert!(ad_build.final_item_ids.contains(&226699));
}

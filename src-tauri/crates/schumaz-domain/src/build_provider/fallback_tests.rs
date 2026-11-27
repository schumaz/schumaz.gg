use super::*;
use crate::build_provider::models::{ChampionProfile, ItemClassificationCache, RuneTreeCatalog};
use crate::models::{QUEUE_ARAM, QUEUE_ARAM_MAYHEM, QUEUE_ARENA, QUEUE_SWIFTPLAY};

fn sample_rune_trees() -> Vec<RuneTreeCatalog> {
    vec![
        RuneTreeCatalog {
            id: 8200,
            key: "Sorcery".to_string(),
            slots: vec![vec![8229, 8214], vec![8226], vec![8210], vec![8237]],
        },
        RuneTreeCatalog {
            id: 8000,
            key: "Precision".to_string(),
            slots: vec![vec![8005, 8008], vec![9111], vec![9104], vec![8014]],
        },
        RuneTreeCatalog {
            id: 8300,
            key: "Inspiration".to_string(),
            slots: vec![vec![8369], vec![8304], vec![8316], vec![8347]],
        },
        RuneTreeCatalog {
            id: 8400,
            key: "Resolve".to_string(),
            slots: vec![vec![8437], vec![8446], vec![8429], vec![8451]],
        },
    ]
}

fn sample_ddragon_catalog() -> DataDragonCache {
    let mut cls = ItemClassificationCache::default();
    let mut register = |id: i32, tags: &[&str], gold: i32, maps: &[i32], kind: &str| {
        cls.item_tags
            .insert(id, tags.iter().map(|s| s.to_string()).collect());
        cls.item_gold.insert(id, gold);
        if maps.contains(&11) {
            cls.map11_items.insert(id);
        }
        if maps.contains(&12) {
            cls.map12_items.insert(id);
        }
        match kind {
            "starter" => {
                cls.starters.insert(id);
            }
            "consumable" => {
                cls.consumables.insert(id);
            }
            "boot" => {
                cls.boots.insert(id);
            }
            "completed" => {
                cls.completed.insert(id);
            }
            _ => {}
        }
    };

    register(1056, &["SpellDamage", "ManaRegen", "Health"], 400, &[11], "starter");
    register(1055, &["Damage", "LifeSteal", "Health"], 450, &[11], "starter");
    register(3112, &["SpellDamage", "ManaRegen", "Health"], 950, &[12], "starter");
    register(2003, &["HealthRegen"], 50, &[11, 12], "consumable");
    register(3020, &["MagicPenetration", "Boots"], 1100, &[11, 12], "boot");
    register(3006, &["AttackSpeed", "Boots"], 1100, &[11, 12], "boot");
    register(3118, &["SpellDamage", "Mana", "AbilityHaste"], 2800, &[11, 12], "completed");
    register(4645, &["SpellDamage", "MagicPenetration"], 3200, &[11, 12], "completed");
    register(3089, &["SpellDamage"], 3600, &[11, 12], "completed");
    register(3031, &["Damage", "CriticalStrike"], 3400, &[11, 12], "completed");
    register(3094, &["AttackSpeed", "CriticalStrike"], 2650, &[11, 12], "completed");
    // Guardian Angel: Map 11 only (must be excluded in ARAM).
    register(3026, &["Damage", "Armor"], 3200, &[11], "completed");

    let mut dd = DataDragonCache {
        version: "16.22.1".to_string(),
        rune_trees: sample_rune_trees(),
        item_classifications: cls,
        ..Default::default()
    };
    dd.champion_profiles.insert(
        103,
        ChampionProfile {
            tags: vec!["Mage".to_string(), "Assassin".to_string()],
            partype: "Mana".to_string(),
            attack: 3,
            defense: 4,
            magic: 8,
        },
    );
    dd.champion_profiles.insert(
        222,
        ChampionProfile {
            tags: vec!["Marksman".to_string()],
            partype: "Mana".to_string(),
            attack: 9,
            defense: 2,
            magic: 4,
        },
    );
    dd
}

#[test]
fn test_opgg_queue_param_and_default_spells_adaptation() {
    assert_eq!(opgg_queue_param_from_id(420), "ranked");
    assert_eq!(opgg_queue_param_from_id(QUEUE_SWIFTPLAY), "ranked");
    assert_eq!(opgg_queue_param_from_id(450), "aram");
    assert_eq!(opgg_queue_param_from_id(QUEUE_ARAM_MAYHEM), "aram");
    assert_eq!(opgg_queue_param_from_id(1700), "arena");
    assert_eq!(default_spells_for_queue(420), (4, 14));
    assert_eq!(default_spells_for_queue(QUEUE_SWIFTPLAY), (4, 14));
    assert_eq!(default_spells_for_queue(450), (4, 32));
    assert_eq!(default_spells_for_queue(QUEUE_ARAM_MAYHEM), (4, 32));
    assert_eq!(default_spells_for_queue(QUEUE_ARENA), (0, 0));
}

#[test]
fn test_fallback_build_uses_dynamic_ddragon_catalog_without_hardcoded_runes_or_aram_invalid_items() {
    let dd = sample_ddragon_catalog();

    // Summoner's Rift Mage: derives 9 dynamic perks (never removed rune 8135) + SR starter/core/boots
    let sr_mage = build_fallback_champion_build_with_ddragon(103, "Ahri", 420, Some(&dd));
    assert_eq!(sr_mage.primary_style_id, 8200);
    assert_eq!(sr_mage.sub_style_id, 8300);
    assert_eq!(sr_mage.selected_perk_ids.len(), 9);
    assert!(!sr_mage.selected_perk_ids.contains(&8135));
    assert_eq!((sr_mage.spell1_id, sr_mage.spell2_id), (4, 14));
    assert_eq!(sr_mage.item_blocks[0].item_ids, vec![1056, 2003]);
    assert!(sr_mage.item_blocks[1].item_ids.contains(&3118));
    assert_eq!(sr_mage.item_blocks[2].item_ids.first().copied(), Some(3020));

    // ARAM Marksman: must exclude Guardian Angel (3026, map 11 only) and prefer Guardian Horn / Hammer / map12 items
    let aram_adc = build_fallback_champion_build_with_ddragon(222, "Jinx", QUEUE_ARAM, Some(&dd));
    assert_eq!((aram_adc.spell1_id, aram_adc.spell2_id), (4, 32));
    assert_eq!(aram_adc.primary_style_id, 8000);
    assert_eq!(aram_adc.selected_perk_ids.len(), 9);
    assert!(!aram_adc.item_blocks[3].item_ids.contains(&3026));
    assert!(aram_adc.item_blocks[3].item_ids.contains(&3031));

    // ARAM Mayhem disables runes cleanly
    let mayhem = build_fallback_champion_build_with_ddragon(103, "Ahri", QUEUE_ARAM_MAYHEM, Some(&dd));
    assert!(!mayhem.supports_runes);
    assert!(mayhem.selected_perk_ids.is_empty());

    // Arena disables runes and spells and keeps 5 empty blocks
    let arena = build_fallback_champion_build_with_ddragon(103, "Ahri", QUEUE_ARENA, Some(&dd));
    assert!(!arena.supports_runes);
    assert!(!arena.supports_spells);
    assert_eq!(arena.item_blocks.len(), 5);
    assert!(arena.item_blocks.iter().all(|b| b.item_ids.is_empty()));

    // Without DataDragon catalog, zero hardcoded runes or items are fabricated
    let empty_fallback = build_fallback_champion_build(103, "Ahri", 420);
    assert!(empty_fallback.selected_perk_ids.is_empty());
    assert!(empty_fallback.item_blocks.iter().all(|b| b.item_ids.is_empty()));
}

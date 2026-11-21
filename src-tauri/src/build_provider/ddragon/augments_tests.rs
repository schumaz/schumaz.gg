use super::*;
use crate::build_provider::ddragon::item_parser::parse_ddragon_item_classifications;
use serde_json::json;

#[test]
fn test_parse_ddragon_item_classifications_handles_map30_prismatics_447111_and_223121_bug() {
    let item_json = json!({
        "data": {
            "443090": {
                "gold": { "total": 2750, "purchasable": false },
                "inStore": false,
                "maps": { "11": false, "12": false, "30": true },
                "tags": ["SpellDamage"]
            },
            "447111": {
                "gold": { "total": 2500, "purchasable": true },
                "maps": { "11": false, "12": false, "30": true },
                "tags": ["Damage", "Health"]
            },
            "220007": {
                "gold": { "total": 4000, "purchasable": true },
                "maps": { "11": false, "12": false, "30": true },
                "tags": ["Consumable"]
            },
            "4017": {
                "gold": { "total": 2500, "purchasable": true },
                "maps": { "11": false, "12": false, "30": true },
                "tags": ["Damage", "ArmorPenetration"]
            },
            "223119": {
                "gold": { "total": 2500, "purchasable": true },
                "maps": { "11": false, "12": false, "30": true },
                "tags": ["Health", "Mana"]
            },
            "223121": {
                "gold": { "total": 2500, "purchasable": false },
                "inStore": false,
                "specialRecipe": 3119,
                "maps": { "11": false, "12": false, "30": true },
                "tags": ["Health", "Mana"]
            },
            "3119": {
                "gold": { "total": 2400, "purchasable": true },
                "maps": { "11": true, "12": true, "30": false },
                "tags": ["Health", "Mana"]
            }
        }
    });

    let cache = parse_ddragon_item_classifications(&item_json);

    assert!(cache.arena_prismatics.contains(&443090));
    assert!(!cache.completed.contains(&443090));
    assert!(!cache.arena_prismatics.contains(&447111));
    assert!(cache.completed.contains(&447111));
    assert!(cache.map30_items.contains(&447111));
    assert!(cache.arena_consumables.contains(&220007));
    assert!(!cache.completed.contains(&220007));
    assert!(cache.map30_items.contains(&4017));
    assert!(cache.completed.contains(&4017));
    assert_eq!(cache.evolved_fallbacks.get(&223121), Some(&223119));
}

#[test]
fn test_parse_cherry_augments_and_supplement_descriptions() {
    let cherry_json = json!([
        {
            "id": 48,
            "nameTRA": "Primeiros Socorros",
            "rarity": "kSilver",
            "augmentSmallIconPath": "/lol-game-data/assets/ASSETS/UX/Cherry/Augments/Icons/FirstAidKit_small.png",
            "summaryDescription": ""
        },
        {
            "id": 1037,
            "nameTRA": "Primeiros Socorros",
            "rarity": "kSilver",
            "augmentSmallIconPath": "/lol-game-data/assets/ASSETS/UX/Cherry/Augments/Icons/FirstAidKit_small.png",
            "summaryDescription": ""
        },
        {
            "id": 1151,
            "nameTRA": "Bread and Cheese",
            "augmentNameId": "ARAM_BreadAndCheese",
            "rarity": "kGold",
            "augmentSmallIconPath": "/lol-game-data/assets/ASSETS/UX/Cherry/Augments/Icons/BreadAndCheese_small.png",
            "summaryDescription": ""
        },
        {
            "id": 1115,
            "nameTRA": "Scopiest Weapons",
            "augmentNameId": "ARAM_ScopiestWeapons",
            "rarity": "kPrismatic",
            "augmentSmallIconPath": "/lol-game-data/assets/ASSETS/UX/Cherry/Augments/Icons/ScopiestWeapons_small.png",
            "summaryDescription": ""
        },
        {
            "id": 3172,
            "nameTRA": "THRASH",
            "augmentNameId": "GoH_PentakillAug_E",
            "rarity": "kGold",
            "augmentSmallIconPath": "/lol-game-data/assets/ASSETS/UX/Cherry/Augments/Icons/Thrash_small.png",
            "summaryDescription": ""
        },
        {
            "id": 3176,
            "nameTRA": " ",
            "augmentNameId": "GoH_PentakillAug_Generic",
            "rarity": "kGold",
            "augmentSmallIconPath": "/lol-game-data/assets/ASSETS/UX/Cherry/Augments/Icons/Generic_small.png",
            "summaryDescription": ""
        },
        {
            "id": 1424,
            "nameTRA": "???",
            "augmentNameId": "ARAM_MissingPing",
            "rarity": "kPrismatic",
            "augmentSmallIconPath": "/lol-game-data/assets/ASSETS/UX/Cherry/Augments/Icons/MissingPing_small.png",
            "summaryDescription": ""
        }
    ]);

    let mut map = parse_cdragon_cherry_augments(&cherry_json);
    // Internal template 3176 (GoH_PentakillAug_Generic with blank nameTRA) is skipped; "???" is kept
    assert!(!map.contains_key(&3176));
    assert_eq!(map[&1424].name, "???");
    assert_eq!(map[&3172].ability_slot.as_deref(), Some("E"));
    assert_eq!(map[&48].rarity, 1);
    assert_eq!(
        map[&48].icon_url,
        "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/cherry/augments/icons/firstaidkit_small.png"
    );

    let desc_json = json!({
        "augments": [
            {
                "id": 48,
                "name": "Primeiros Socorros",
                "desc": "Recebe <healing>@HealShieldPower*100@%</healing> de Cura e Escudo.",
                "dataValues": { "HealShieldPower": 0.25 }
            },
            {
                "id": 151,
                "name": "Bread And Cheese",
                "desc": "Your @spell3KeyBind@ gains @EAbilityHaste@ Ability Haste.",
                "dataValues": { "EAbilityHaste": [100.0, 200.0] }
            },
            {
                "id": 115,
                "name": "Scopiest Weapons",
                "desc": "Gain @RangeIncrease@ Attack Range.",
                "dataValues": { "MeleeRangeIncrease": [150.0], "RangedRangeIncrease": [90.0] }
            }
        ]
    });

    supplement_arena_augment_descriptions(&mut map, &desc_json);
    assert_eq!(map[&48].desc, "Recebe 25% de Cura e Escudo.");
    assert_eq!(map[&1037].desc, "Recebe 25% de Cura e Escudo.");
    // @spell3KeyBind@ -> E, propagated to ARAM Mayhem counterpart 1151 along with ability_slot
    assert_eq!(map[&1151].desc, "Your E gains 100 Ability Haste.");
    assert_eq!(map[&1151].ability_slot.as_deref(), Some("E"));
    // @RangeIncrease@ -> Melee/Ranged pair 150/90
    assert_eq!(map[&1115].desc, "Gain 150/90 Attack Range.");
}

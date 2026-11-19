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

    // 443090 (2750g) is a real Prismatic and NOT a purchasable completed legendary
    assert!(cache.arena_prismatics.contains(&443090));
    assert!(!cache.completed.contains(&443090));

    // 447111 (2500g Overlord's Bloodmail) is a purchasable legendary, NOT a Prismatic
    assert!(!cache.arena_prismatics.contains(&447111));
    assert!(cache.completed.contains(&447111));
    assert!(cache.map30_items.contains(&447111));

    // 220007 (Prismatic Anvil) is a consumable and NOT a completed legendary
    assert!(cache.arena_consumables.contains(&220007));
    assert!(!cache.completed.contains(&220007));

    // 4-digit Arena exclusive 4017 is preserved as Map 30 completed legendary
    assert!(cache.map30_items.contains(&4017));
    assert!(cache.completed.contains(&4017));

    // Riot bug protection: 223121 (Arena Fimbulwinter) maps to Map 30 223119, NEVER SR 3119
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
            "id": 1324,
            "nameTRA": "Shake de Proteína",
            "rarity": "kPrismatic",
            "augmentSmallIconPath": "/lol-game-data/assets/ASSETS/UX/Cherry/Augments/Icons/ProteinShake_small.png",
            "summaryDescription": ""
        }
    ]);

    let mut map = parse_cdragon_cherry_augments(&cherry_json);
    assert_eq!(map.len(), 2);
    assert_eq!(map[&48].name, "Primeiros Socorros");
    assert_eq!(map[&48].rarity, 1);
    assert_eq!(
        map[&48].icon_url,
        "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/cherry/augments/icons/firstaidkit_small.png"
    );
    assert_eq!(map[&1324].rarity, 8);

    let desc_json = json!({
        "augments": [
            {
                "id": 48,
                "name": "Primeiros Socorros",
                "desc": "Recebe <healing>@HealShieldPower*100@%</healing> de Cura e Escudo.",
                "dataValues": { "HealShieldPower": 0.25 }
            }
        ]
    });

    supplement_arena_augment_descriptions(&mut map, &desc_json);
    assert_eq!(map[&48].desc, "Recebe 25% de Cura e Escudo.");
}

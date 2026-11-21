use super::*;
use crate::build_provider::ddragon::cache::{
    DataDragonDiskBundle, hydrate_ddragon_from_dir, save_ddragon_bundle_to_dir,
};
use crate::build_provider::ddragon::item_parser::parse_ddragon_item_classifications;
use crate::build_provider::models::DataDragonCache;
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

#[test]
fn test_supplement_descriptions_strips_riot_inline_icons_and_brace_templates() {
    let cherry_json = json!([
        {
            "id": 1048,
            "nameTRA": "Jeweled Gauntlet",
            "rarity": "kPrismatic",
            "augmentSmallIconPath": "/lol-game-data/assets/jg.png",
            "summaryDescription": ""
        },
        {
            "id": 1097,
            "nameTRA": "Witchful Thinking",
            "rarity": "kSilver",
            "augmentSmallIconPath": "/lol-game-data/assets/wt.png",
            "summaryDescription": ""
        },
        {
            "id": 1238,
            "nameTRA": "Transmute: Prismatic",
            "rarity": "kGold",
            "augmentSmallIconPath": "/lol-game-data/assets/tp.png",
            "summaryDescription": ""
        },
        {
            "id": 1087,
            "nameTRA": "Typhoon",
            "rarity": "kSilver",
            "augmentSmallIconPath": "/lol-game-data/assets/ty.png",
            "summaryDescription": ""
        },
        {
            "id": 1090,
            "nameTRA": "Vengeance",
            "rarity": "kGold",
            "augmentSmallIconPath": "/lol-game-data/assets/vg.png",
            "summaryDescription": ""
        }
    ]);
    let desc_en = json!({
        "augments": [
            {
                "id": 48,
                "name": "Jeweled Gauntlet",
                "desc": "Your Abilities can <crit>Critically Strike</crit>. <br><br>Gain <crit>%i:scaleCrit% @CritGranted@ Crit Chance</crit>.",
                "dataValues": { "CritChance": [0.25] }
            },
            {
                "id": 97,
                "name": "Witchful Thinking",
                "desc": "Gain %i:scaleAP% <scaleAP>@AP@ Ability Power</scaleAP>.",
                "dataValues": { "AP": [60.0] }
            },
            {
                "id": 238,
                "name": "Transmute: Prismatic",
                "desc": "Gain a random <keywordMajor>%i:Augment% Prismatic Augment</keywordMajor>.",
                "dataValues": {}
            },
            {
                "id": 87,
                "name": "Typhoon",
                "desc": "Your Attacks apply {{ Item_Keyword_OnHit }} effects and {{ Item_Grievous_Wounds }} for 25%.",
                "dataValues": {}
            },
            {
                "id": 90,
                "name": "Vengeance",
                "desc": "{{ Cherry_Vengeance@TeamSize@_Summary }}",
                "dataValues": {}
            }
        ]
    });

    let mut map_en = parse_cdragon_cherry_augments(&cherry_json);
    supplement_arena_augment_descriptions_for_locale(&mut map_en, &desc_en, AppLocale::EnUs);
    assert_eq!(
        map_en[&1048].desc,
        "Your Abilities can Critically Strike. Gain Crit Chance."
    );
    assert_eq!(map_en[&1097].desc, "Gain 60 Ability Power.");
    assert_eq!(map_en[&1238].desc, "Gain a random Prismatic Augment.");
    assert_eq!(
        map_en[&1087].desc,
        "Your Attacks apply On-Hit effects and Grievous Wounds for 25%."
    );
    assert_eq!(map_en[&1090].desc, "");

    let desc_pt = json!({
        "augments": [
            {
                "id": 58,
                "name": "Soco Místico",
                "desc": "{{ Item_Keyword_OnHit }} reduz o Tempo de Recarga em @CooldownRefund@s %i:scaleCooldown%.",
                "dataValues": { "CooldownRefund": 1.2 }
            }
        ]
    });
    let mut map_pt = HashMap::new();
    supplement_arena_augment_descriptions_for_locale(&mut map_pt, &desc_pt, AppLocale::PtBr);
    assert_eq!(
        map_pt[&58].desc,
        "ao contato reduz o Tempo de Recarga em 1.2s."
    );
}

#[test]
fn test_hydrate_ddragon_from_dir_heals_cached_riot_inline_markup() {
    let dir = std::env::temp_dir().join(format!(
        "schumazgg_ddragon_heal_riot_markup_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let mut arena_augments = HashMap::new();
    arena_augments.insert(
        1048,
        ArenaAugmentMetadata {
            id: 1048,
            name: "Jeweled Gauntlet".to_string(),
            desc: "Your Abilities can Critically Strike. Gain %i:scaleCrit% Crit Chance."
                .to_string(),
            icon_url: "https://raw.communitydragon.org/latest/jg.png".to_string(),
            rarity: 8,
            ability_slot: None,
        },
    );
    arena_augments.insert(
        1087,
        ArenaAugmentMetadata {
            id: 1087,
            name: "Typhoon".to_string(),
            desc: "Applies {{ Item_Keyword_OnHit }} effects and %is:scalecrit% bonus.".to_string(),
            icon_url: "https://raw.communitydragon.org/latest/ty.png".to_string(),
            rarity: 1,
            ability_slot: None,
        },
    );
    let bundle = DataDragonDiskBundle {
        ddragon: DataDragonCache {
            version: "16.22.1".to_string(),
            arena_augments,
            ..Default::default()
        },
        item_classifications: None,
    };
    save_ddragon_bundle_to_dir(&dir, AppLocale::EnUs, &bundle).unwrap();

    let hydrated = hydrate_ddragon_from_dir(&dir, AppLocale::EnUs).unwrap();
    assert_eq!(
        hydrated.arena_augments[&1048].desc,
        "Your Abilities can Critically Strike. Gain Crit Chance."
    );
    assert_eq!(
        hydrated.arena_augments[&1087].desc,
        "Applies On-Hit effects and bonus."
    );

    let _ = std::fs::remove_dir_all(&dir);
}

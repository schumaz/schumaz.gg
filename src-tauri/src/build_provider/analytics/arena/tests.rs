use super::build_parser::parse_arena_champion_payload;
use super::items_tests::install_arena_item_classifications;
use super::tier_parser::parse_arena_tier_list;
use crate::build_provider::ddragon::enrich_build_with_ddragon_for_queue_with_flash_key_and_locale;
use crate::build_provider::items::TEST_CLASSIFICATION_MUTEX;
use crate::build_provider::models::{ChampionMetadata, DataDragonCache};
use crate::lcu_injector::FlashKeyPreference;
use crate::models::{AppLocale, ArenaAugmentMetadata, QUEUE_ARENA};
use serde_json::json;
use std::collections::HashMap;

fn make_arena_ddragon() -> DataDragonCache {
    let mut champions = HashMap::new();
    for (id, key, name) in [
        (350, "Yuumi", "Yuumi"),
        (11, "MasterYi", "Master Yi"),
        (106, "Volibear", "Volibear"),
        (86, "Garen", "Garen"),
    ] {
        champions.insert(
            id,
            ChampionMetadata {
                champion_id: id,
                key_name: key.to_string(),
                display_name: name.to_string(),
                icon_url: format!(
                    "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/{key}.png"
                ),
            },
        );
    }

    let mut arena_augments = HashMap::new();
    arena_augments.insert(
        48,
        ArenaAugmentMetadata {
            id: 48,
            name: "Primeiros Socorros".to_string(),
            desc: "Recebe +25% de Cura e Escudo.".to_string(),
            icon_url: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/cherry/augments/icons/firstaidkit_small.png".to_string(),
            rarity: 1,
            ability_slot: None,
        },
    );
    arena_augments.insert(
        1324,
        ArenaAugmentMetadata {
            id: 1324,
            name: "Shake de Proteína".to_string(),
            desc: String::new(),
            icon_url: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/cherry/augments/icons/proteinshake_small.png".to_string(),
            rarity: 8,
            ability_slot: None,
        },
    );

    DataDragonCache {
        version: "16.22.1".to_string(),
        champions,
        arena_augments,
        ..Default::default()
    }
}

#[test]
fn test_parse_arena_tier_list_skips_rip_and_computes_arena_metrics() {
    let dd = make_arena_ddragon();
    let payload = json!({
        "meta": { "version": "16.22" },
        "data": [
            {
                "id": 60086,
                "is_rip": true,
                "average_stats": null
            },
            {
                "id": 86,
                "is_rip": false,
                "average_stats": {
                    "win": 34380,
                    "play": 65563,
                    "total_place": 223617,
                    "first_place": 11422,
                    "pick_rate": 0.105347,
                    "ban_rate": 0.0442321,
                    "tier": 2,
                    "rank": 50,
                    "tier_data": { "tier": 2, "rank": 50 }
                }
            },
            {
                "id": 11,
                "is_rip": false,
                "average_stats": {
                    "win": 42000,
                    "play": 70000,
                    "total_place": 210000,
                    "first_place": 14000,
                    "pick_rate": 0.125,
                    "ban_rate": 0.21,
                    "tier": 1,
                    "rank": 3,
                    "tier_data": { "tier": 1, "rank": 3 }
                }
            }
        ]
    });

    let list = parse_arena_tier_list(&payload, &dd).expect("valid arena tier list");
    assert_eq!(list.len(), 2);

    assert_eq!(list[0].id, 11);
    assert_eq!(list[0].name, "Master Yi");
    assert_eq!(list[0].tier, 0);
    assert_eq!(list[0].rank, 3);
    assert_eq!(list[0].win_rate, Some(60.0));
    assert_eq!(list[0].first_place_rate, Some(20.0));
    assert_eq!(list[0].avg_placement, Some(3.0));
    assert_eq!(list[0].pick_rate, Some(12.5));
    assert_eq!(list[0].ban_rate, Some(21.0));

    assert_eq!(list[1].id, 86);
    assert_eq!(list[1].tier, 1);
    assert_eq!(list[1].win_rate, Some(52.4));
    assert_eq!(list[1].first_place_rate, Some(17.4));
    assert_eq!(list[1].avg_placement, Some(3.41));
}

#[test]
fn test_parse_arena_champion_payload_extracts_augments_synergies_and_enriches_locale() {
    let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
    install_arena_item_classifications();
    let dd = make_arena_ddragon();

    let payload = json!({
        "data": {
            "summary": {
                "average_stats": {
                    "win": 5400,
                    "play": 10000,
                    "first_place": 1800,
                    "total_place": 32500,
                    "pick_rate": 0.085,
                    "tier": 2,
                    "tier_data": { "tier": 2 }
                }
            },
            "boots": [{ "ids": [223158], "play": 9000, "win": 4800, "pick_rate": 0.9 }],
            "prism_items": [{ "ids": [443090], "play": 3000, "win": 1800, "pick_rate": 0.3 }],
            "core_items": [{ "ids": [226617, 223504, 223107], "play": 2500, "win": 1400, "pick_rate": 0.25 }],
            "last_items": [{ "ids": [226620], "play": 2000, "win": 1100, "pick_rate": 0.2 }],
            "augment_group": [
                {
                    "rarity": 1,
                    "augments": [
                        { "id": 48, "play": 1200, "win": 720, "first_place": 240, "total_place": 3600, "pick_rate": 0.12, "win_rate": 0.60 }
                    ]
                },
                {
                    "rarity": 8,
                    "augments": [
                        { "id": 1324, "play": 1800, "win": 1170, "first_place": 450, "total_place": 5040, "pick_rate": 0.18 }
                    ]
                }
            ],
            "synergies": [
                { "champion_id": 11, "op_rank": 1, "play": 900, "win": 540, "first_place": 225, "total_place": 2700, "pick_rate": 0.09 },
                { "champion_id": 106, "op_rank": 2, "play": 600, "win": 330, "first_place": 120, "total_place": 1980, "pick_rate": 0.06 }
            ]
        }
    });

    let mut build =
        parse_arena_champion_payload(&payload, 350, "Yuumi", &dd).expect("parsed arena build");
    enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
        &mut build,
        &dd,
        FlashKeyPreference::D,
        AppLocale::PtBr,
        Some(QUEUE_ARENA),
    );

    assert!(!build.supports_runes);
    assert!(!build.supports_spells);
    assert_eq!(build.tier, Some(1));
    assert_eq!(build.spell1_id, 0);
    assert_eq!(build.spell2_id, 0);
    assert!(build.spells.is_empty());
    assert_eq!(build.win_rate, 54.0);
    assert_eq!(build.first_place_rate, Some(18.0));
    assert_eq!(build.avg_placement, Some(3.25));
    assert_eq!(build.item_blocks[0].title, "Itens Prismáticos");

    assert_eq!(build.arena_augments.len(), 2);
    assert_eq!(build.arena_augments[0].id, 1324);
    assert_eq!(build.arena_augments[0].name, "Shake de Proteína");
    assert_eq!(build.arena_augments[0].rarity, 8);
    assert_eq!(build.arena_augments[0].win_rate, 65.0);
    assert_eq!(build.arena_augments[1].id, 48);
    assert_eq!(build.arena_augments[1].name, "Primeiros Socorros");
    assert_eq!(
        build.arena_augments[1].desc,
        "Recebe +25% de Cura e Escudo."
    );

    assert_eq!(build.arena_synergies.len(), 2);
    assert_eq!(build.arena_synergies[0].champion_id, 11);
    assert_eq!(build.arena_synergies[0].name, "Master Yi");
    assert_eq!(build.arena_synergies[0].win_rate, 60.0);
    assert_eq!(build.arena_synergies[0].first_place_rate, 25.0);
    assert_eq!(build.arena_synergies[0].avg_placement, 3.0);
}

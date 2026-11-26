use std::collections::HashMap;

use super::super::matches_parser::parse_match_history_page_for_locale;
use super::build_match_history_page_response;
use crate::models::{
    AppLocale, ArenaAugmentMetadata, ChampionMetadata, DataDragonCache, ItemMetadata, RuneMetadata,
    SpellMetadata,
};

fn sample_pagination_cache() -> DataDragonCache {
    let mut champions = HashMap::new();
    champions.insert(
        103,
        ChampionMetadata {
            champion_id: 103,
            key_name: "Ahri".to_string(),
            display_name: "Ahri".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Ahri.png"
                .to_string(),
        },
    );

    let mut items = HashMap::new();
    items.insert(
        6655,
        ItemMetadata {
            id: 6655,
            name: "Companheiro de Luden".to_string(),
            gold_total: 2750,
            stats: vec![],
            plaintext: "Burst".to_string(),
        },
    );
    items.insert(
        3340,
        ItemMetadata {
            id: 3340,
            name: "Sentinela Invisível".to_string(),
            gold_total: 0,
            stats: vec![],
            plaintext: "Ward".to_string(),
        },
    );

    let mut runes = HashMap::new();
    runes.insert(
        8112,
        RuneMetadata {
            id: 8112,
            key: "Electrocute".to_string(),
            name: "Eletrocutar".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/Domination/Electrocute/Electrocute.png".to_string(),
            short_desc: "3 hits".to_string(),
        },
    );

    let mut spells = HashMap::new();
    spells.insert(
        4,
        SpellMetadata {
            id: 4,
            name: "Flash".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/spell/SummonerFlash.png"
                .to_string(),
            ..Default::default()
        },
    );

    let mut arena_augments = HashMap::new();
    arena_augments.insert(
        501,
        ArenaAugmentMetadata {
            id: 501,
            name: "Eureka".to_string(),
            desc: "Gain AH".to_string(),
            icon_url: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/cherry/augments/icons/eureka_small.png".to_string(),
            rarity: 8,
            ability_slot: None,
        },
    );

    DataDragonCache {
        version: "16.22.1".to_string(),
        champions,
        runes,
        spells,
        items,
        arena_augments,
        ..Default::default()
    }
}

fn make_game(game_id: u64, queue_id: i32, game_mode: &str, creation_ms: i64) -> serde_json::Value {
    let stats = if queue_id == 1700 {
        serde_json::json!({
            "win": true,
            "kills": 6,
            "deaths": 2,
            "assists": 4,
            "item0": 6655,
            "item6": 3340,
            "subteamPlacement": 1,
            "playerAugment1": 501
        })
    } else {
        serde_json::json!({
            "win": true,
            "kills": 6,
            "deaths": 2,
            "assists": 4,
            "item0": 6655,
            "item6": 3340,
            "perk0": 8112
        })
    };

    serde_json::json!({
        "gameId": game_id,
        "queueId": queue_id,
        "gameMode": game_mode,
        "gameType": if queue_id < 0 { "CUSTOM_GAME" } else { "MATCHED_GAME" },
        "gameDuration": 1500,
        "gameCreation": creation_ms,
        "participants": [{
            "championId": 103,
            "spell1Id": 4,
            "spell2Id": 14,
            "stats": stats
        }]
    })
}

#[test]
fn test_paginated_window_preserves_raw_cursor_across_practice_tool_exclusions_and_collects_metadata()
{
    let cache = sample_pagination_cache();
    let mut games = Vec::new();
    for idx in 0..10 {
        let is_practice = idx == 2 || idx == 7;
        let (qid, mode) = if is_practice {
            (-1, "PRACTICETOOL")
        } else if idx == 9 {
            (1700, "CHERRY")
        } else {
            (420, "CLASSIC")
        };
        games.push(make_game(
            100 + idx as u64,
            qid,
            mode,
            1_700_000_000_000 - (idx as i64) * 60_000,
        ));
    }
    let history = serde_json::json!({ "games": { "games": games } });

    let parsed =
        parse_match_history_page_for_locale(&history, 0, &cache, 0, 10, AppLocale::PtBr);
    assert_eq!(parsed.matches.len(), 8);
    assert_eq!(parsed.raw_games_count, 10);
    assert_eq!(parsed.next_beg_index, 10);
    assert!(parsed.has_more);

    let page = build_match_history_page_response(&history, 0, &cache, 0, 10, AppLocale::PtBr);
    assert_eq!(page.matches.len(), 8);
    assert_eq!(page.next_beg_index, 10);
    assert!(page.has_more);
    assert!(page.item_metadata.contains_key(&6655));
    assert!(page.item_metadata.contains_key(&3340));
    assert!(page.rune_metadata.contains_key(&8112));
    assert!(page.spell_metadata.contains_key(&4));
    assert!(page.augment_metadata.contains_key(&501));
}

#[test]
fn test_paginated_window_handles_partial_and_empty_final_pages() {
    let cache = sample_pagination_cache();
    let partial_games: Vec<serde_json::Value> = (0..4)
        .map(|i| make_game(200 + i, 420, "CLASSIC", 1_700_000_000_000 - (i as i64) * 1000))
        .collect();
    let partial_history = serde_json::json!({ "games": { "games": partial_games } });

    let partial = build_match_history_page_response(
        &partial_history,
        0,
        &cache,
        10,
        10,
        AppLocale::EnUs,
    );
    assert_eq!(partial.matches.len(), 4);
    assert_eq!(partial.next_beg_index, 14);
    assert!(!partial.has_more);

    let empty_history = serde_json::json!({ "games": { "games": [] } });
    let empty = build_match_history_page_response(
        &empty_history,
        0,
        &cache,
        20,
        10,
        AppLocale::EnUs,
    );
    assert!(empty.matches.is_empty());
    assert_eq!(empty.next_beg_index, 20);
    assert!(!empty.has_more);
}

#[test]
fn test_paginated_window_caps_oversized_fallback_payload_to_newest_page_size() {
    let cache = sample_pagination_cache();
    let games: Vec<serde_json::Value> = (1..=15)
        .map(|i| make_game(300 + i, 420, "CLASSIC", 1_700_000_000_000 + (i as i64) * 1000))
        .collect();
    let history = serde_json::json!({ "games": { "games": games } });

    let page = build_match_history_page_response(&history, 0, &cache, 0, 10, AppLocale::PtBr);
    assert_eq!(page.matches.len(), 10);
    assert_eq!(page.matches[0].game_id, 315);
    assert_eq!(page.matches[9].game_id, 306);
    assert_eq!(page.next_beg_index, 10);
    assert!(page.has_more);
}

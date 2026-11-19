use std::collections::HashMap;

use super::*;
use crate::models::{ChampionMetadata, SpellMetadata};

fn sample_cache() -> DataDragonCache {
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

    let mut spells = HashMap::new();
    spells.insert(
        4,
        SpellMetadata {
            id: 4,
            name: "Flash".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/spell/SummonerFlash.png"
                .to_string(),
            modes: Vec::new(),
        },
    );
    spells.insert(
        14,
        SpellMetadata {
            id: 14,
            name: "Incendiar".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/spell/SummonerDot.png"
                .to_string(),
            modes: Vec::new(),
        },
    );

    DataDragonCache {
        version: "16.22.1".to_string(),
        champions,
        runes: HashMap::new(),
        spells,

        ..Default::default()
    }
}

#[test]
fn test_parse_recent_matches_strictly_excludes_practice_tool_and_caps_at_10() {
    let cache = sample_cache();
    let mut games = vec![serde_json::json!({
        "gameId": 999,
        "queueId": 0,
        "gameMode": "PRACTICETOOL",
        "gameType": "CUSTOM_GAME",
        "gameDuration": 300,
        "gameCreation": 2000000000000i64,
        "participants": [{
            "championId": 103,
            "spell1Id": 4,
            "spell2Id": 14,
            "stats": { "win": true, "kills": 0, "deaths": 0, "assists": 0 }
        }]
    })];

    for i in 1..=12 {
        games.push(serde_json::json!({
            "gameId": i,
            "queueId": if i % 2 == 0 { 420 } else { 450 },
            "gameMode": if i % 2 == 0 { "CLASSIC" } else { "ARAM" },
            "gameType": "MATCHED_GAME",
            "gameDuration": 1500 + i * 10,
            "gameCreation": 1700000000000i64 + i * 100000,
            "participants": [{
                "championId": 103,
                "spell1Id": 4,
                "spell2Id": 14,
                "stats": {
                    "win": i % 3 != 0,
                    "kills": 8,
                    "deaths": 2,
                    "assists": 10,
                    "totalMinionsKilled": 180,
                    "neutralMinionsKilled": 15,
                    "item0": 6655,
                    "item1": 3020,
                    "item2": 3089,
                    "item3": 0,
                    "item4": 0,
                    "item5": 0,
                    "goldEarned": 14500,
                    "totalDamageDealtToChampions": 29800,
                    "visionScore": 22,
                    "wardsPlaced": 9,
                    "champLevel": 16,
                    "doubleKills": 1
                }
            }]
        }));
    }

    let history = serde_json::json!({ "games": { "games": games } });
    let parsed = parse_recent_matches(&history, 0, &cache, 10);

    assert_eq!(parsed.len(), 10);
    assert!(parsed.iter().all(|m| m.game_id != 999));
    assert!(parsed.iter().all(|m| m.game_mode_key != "practicetool"));
    // Most recent first (gameId 12 down to 3)
    assert_eq!(parsed[0].game_id, 12);
    assert_eq!(parsed[0].game_mode_label, "Solo/Duo");
    assert_eq!(parsed[0].cs, 195);
    assert_eq!(parsed[0].item_ids.len(), 6);
    assert_eq!(parsed[0].double_kills, 1);
}

use std::collections::HashMap;

use super::*;
use crate::models::{ArenaAugmentMetadata, ChampionMetadata, RuneMetadata, SpellMetadata};
use crate::profile_provider::match_metadata::{
    collect_matches_augment_metadata, collect_matches_rune_metadata,
};

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
            ..Default::default()
        },
    );
    spells.insert(
        14,
        SpellMetadata {
            id: 14,
            name: "Incendiar".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/spell/SummonerDot.png"
                .to_string(),
            ..Default::default()
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

#[test]
fn test_parse_recent_matches_extracts_runes_only_when_positive_and_trinket() {
    let mut cache = sample_cache();
    cache.runes.insert(
        8112,
        RuneMetadata {
            id: 8112,
            key: "Electrocute".to_string(),
            name: "Eletrocutar".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/Domination/Electrocute/Electrocute.png".to_string(),
            short_desc: "Dano explosivo".to_string(),
        },
    );
    cache.runes.insert(
        8200,
        RuneMetadata {
            id: 8200,
            key: "Sorcery".to_string(),
            name: "Feitiçaria".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/7202_Sorcery.png".to_string(),
            short_desc: String::new(),
        },
    );

    let history = serde_json::json!({
        "games": {
            "games": [
                {
                    "gameId": 101,
                    "queueId": 420,
                    "gameMode": "CLASSIC",
                    "gameDuration": 1600,
                    "gameCreation": 1700000002000i64,
                    "participants": [{
                        "championId": 103,
                        "spell1Id": 4,
                        "spell2Id": 14,
                        "stats": {
                            "win": true,
                            "kills": 5,
                            "deaths": 1,
                            "assists": 4,
                            "perk0": 8112,
                            "perkPrimaryStyle": 0,
                            "perkSubStyle": 8200,
                            "item6": 3340
                        }
                    }]
                },
                {
                    "gameId": 102,
                    "queueId": 450,
                    "gameMode": "ARAM",
                    "gameDuration": 1200,
                    "gameCreation": 1700000001000i64,
                    "participants": [{
                        "championId": 103,
                        "stats": { "win": false, "perk0": 0, "perkPrimaryStyle": -1, "item6": 0 }
                    }]
                }
            ]
        }
    });

    let parsed = parse_recent_matches(&history, 0, &cache, 10);
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].primary_rune_id, 8112);
    assert!(parsed[0].primary_rune_icon_url.contains("Electrocute.png"));
    assert_eq!(parsed[0].primary_style_id, 0);
    assert_eq!(parsed[0].primary_style_icon_url, "");
    assert_eq!(parsed[0].sub_style_id, 8200);
    assert!(parsed[0].sub_style_icon_url.contains("7202_Sorcery.png"));
    assert_eq!(parsed[0].trinket_id, 3340);
    assert_eq!(
        parsed[0].trinket_icon_url,
        "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/item/3340.png"
    );

    assert_eq!(parsed[1].primary_rune_id, 0);
    assert_eq!(parsed[1].primary_rune_icon_url, "");
    assert_eq!(parsed[1].primary_style_id, 0);
    assert_eq!(parsed[1].trinket_id, 0);
    assert_eq!(parsed[1].trinket_icon_url, "");

    let rune_meta = collect_matches_rune_metadata(&parsed, &cache.runes);
    assert_eq!(rune_meta.len(), 2);
    assert!(!rune_meta.contains_key(&0));
}

#[test]
fn test_parse_recent_matches_remake_detection_and_controls() {
    let cache = sample_cache();
    let history = serde_json::json!({
        "games": {
            "games": [
                {
                    "gameId": 201,
                    "queueId": 420,
                    "gameDuration": 175,
                    "gameCreation": 1700000004000i64,
                    "participants": [{ "championId": 103, "stats": { "win": false, "gameEndedInEarlySurrender": true, "kills": 1, "deaths": 0, "assists": 0 } }]
                },
                {
                    "gameId": 202,
                    "queueId": 420,
                    "gameDuration": 195,
                    "gameCreation": 1700000003000i64,
                    "participants": [{ "championId": 103, "stats": { "win": false, "kills": 0, "deaths": 0, "assists": 0 } }]
                },
                {
                    "gameId": 203,
                    "queueId": 420,
                    "gameDuration": 180,
                    "gameCreation": 1700000002000i64,
                    "participants": [{ "championId": 103, "stats": { "win": false, "kills": 2, "deaths": 1, "assists": 0 } }]
                },
                {
                    "gameId": 204,
                    "queueId": 420,
                    "gameDuration": 0,
                    "gameCreation": 1700000001000i64,
                    "participants": [{ "championId": 103, "stats": { "win": true, "kills": 0, "deaths": 0, "assists": 0 } }]
                }
            ]
        }
    });

    let parsed = parse_recent_matches(&history, 0, &cache, 10);
    assert!(parsed[0].remake);
    assert!(parsed[1].remake);
    assert!(!parsed[2].remake);
    assert!(!parsed[3].remake);
}

#[test]
fn test_parse_recent_matches_arena_placement_and_augments() {
    let mut cache = sample_cache();
    cache.arena_augments.insert(
        401,
        ArenaAugmentMetadata {
            id: 401,
            name: "Infernal Conduit".to_string(),
            desc: "Abilities apply burn.".to_string(),
            icon_url: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/cherry/augments/icons/infernalconduit_small.png".to_string(),
            rarity: 8,
            ability_slot: None,
        },
    );

    let history = serde_json::json!({
        "games": {
            "games": [
                {
                    "gameId": 301,
                    "queueId": 1700,
                    "mapId": 30,
                    "gameMode": "CHERRY",
                    "gameDuration": 900,
                    "gameCreation": 1700000003000i64,
                    "participants": [{
                        "championId": 103,
                        "stats": { "win": true, "subteamPlacement": 1, "placement": 4, "playerAugment1": 401, "playerAugment2": 0, "playerAugment3": 402 }
                    }]
                },
                {
                    "gameId": 302,
                    "queueId": 1700,
                    "gameMode": "CHERRY",
                    "gameDuration": 850,
                    "gameCreation": 1700000002000i64,
                    "participants": [{
                        "championId": 103,
                        "stats": { "win": false, "subteamPlacement": 0, "placement": 6, "playerAugment1": 401 }
                    }]
                },
                {
                    "gameId": 303,
                    "queueId": 2400,
                    "gameMode": "KIWI",
                    "gameDuration": 750,
                    "gameCreation": 1700000001000i64,
                    "participants": [{
                        "championId": 103,
                        "stats": { "win": true, "subteamPlacement": 2, "playerAugment1": 401, "playerAugment4": 403 }
                    }]
                }
            ]
        }
    });

    let parsed = parse_recent_matches(&history, 0, &cache, 10);
    assert_eq!(parsed[0].arena_placement, Some(1));
    assert_eq!(parsed[0].augment_ids, vec![401, 402]);
    assert_eq!(parsed[1].arena_placement, Some(6));
    assert_eq!(parsed[1].augment_ids, vec![401]);
    assert_eq!(parsed[2].game_mode_key, "aram_mayhem");
    assert_eq!(parsed[2].arena_placement, None);
    assert_eq!(parsed[2].augment_ids, vec![401, 403]);

    let aug_meta = collect_matches_augment_metadata(&parsed, &cache.arena_augments);
    assert_eq!(aug_meta.get(&401).map(|a| a.name.as_str()), Some("Infernal Conduit"));
    assert!(!aug_meta.contains_key(&402));
}

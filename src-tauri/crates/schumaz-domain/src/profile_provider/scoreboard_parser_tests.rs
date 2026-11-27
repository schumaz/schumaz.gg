use std::collections::HashMap;

use super::parse_match_scoreboard_detail_for_locale;
use crate::models::{
    AppLocale, ArenaAugmentMetadata, ChampionMetadata, DataDragonCache, ItemMetadata, RuneMetadata,
    SpellMetadata,
};

fn sample_scoreboard_cache() -> DataDragonCache {
    let mut champions = HashMap::new();
    for (id, key) in [(103, "Ahri"), (238, "Zed"), (81, "Ezreal")] {
        champions.insert(
            id,
            ChampionMetadata {
                champion_id: id,
                key_name: key.to_string(),
                display_name: key.to_string(),
                icon_url: format!("https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/{key}.png"),
            },
        );
    }

    let mut items = HashMap::new();
    for (id, name, gold) in [
        (6655, "Companheiro de Luden", 2750),
        (3157, "Ampulheta de Zhonya", 3250),
        (3340, "Sentinela Invisível", 0),
    ] {
        items.insert(
            id,
            ItemMetadata { id, name: name.to_string(), gold_total: gold, stats: vec![], plaintext: String::new() },
        );
    }

    let mut runes = HashMap::new();
    runes.insert(8112, RuneMetadata {
        id: 8112,
        key: "Electrocute".to_string(),
        name: "Eletrocutar".to_string(),
        icon_url: "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/Domination/Electrocute/Electrocute.png".to_string(),
        short_desc: "Burst".to_string(),
    });
    runes.insert(8200, RuneMetadata {
        id: 8200,
        key: "Sorcery".to_string(),
        name: "Feitiçaria".to_string(),
        icon_url: "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/7202_Sorcery.png".to_string(),
        short_desc: String::new(),
    });

    let mut spells = HashMap::new();
    spells.insert(4, SpellMetadata {
        id: 4,
        name: "Flash".to_string(),
        icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/spell/SummonerFlash.png".to_string(),
        ..Default::default()
    });

    let mut arena_augments = HashMap::new();
    arena_augments.insert(501, ArenaAugmentMetadata {
        id: 501,
        name: "Eureka".to_string(),
        desc: "Ability Haste".to_string(),
        icon_url: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/cherry/augments/icons/eureka_small.png".to_string(),
        rarity: 8,
        ability_slot: None,
    });

    DataDragonCache { version: "16.22.1".to_string(), champions, runes, spells, items, arena_augments, ..Default::default() }
}

#[test]
fn test_parses_5v5_scoreboard_grouped_strictly_by_team_id_with_objectives_kp_and_bot_safety() {
    let cache = sample_scoreboard_cache();
    // Intentionally scramble participant order (200, 100, 200, 100) to ensure strict teamId grouping.
    let payload = serde_json::json!({
        "gameId": 7001, "queueId": 420, "mapId": 11, "gameMode": "CLASSIC",
        "gameDuration": 1800, "gameCreation": 1700000100000i64,
        "teams": [
            { "teamId": 200, "win": "Fail", "towerKills": 3, "dragonKills": 1, "baronKills": 0, "riftHeraldKills": 0, "hordeKills": 3, "inhibitorKills": 0 },
            { "teamId": 100, "win": "Win", "towerKills": 9, "dragonKills": 3, "baronKills": 1, "riftHeraldKills": 1, "voidgrubKills": 6, "inhibitorKills": 2 }
        ],
        "participantIdentities": [
            { "participantId": 1, "player": { "summonerId": 0, "gameName": "", "tagLine": "", "summonerName": "Zed Bot" } },
            { "participantId": 2, "player": { "summonerId": 987654321, "gameName": "Schuma", "tagLine": "BR1" } },
            { "participantId": 3, "player": { "summonerId": 0, "gameName": "", "tagLine": "", "summonerName": "" } },
            { "participantId": 4, "player": { "summonerId": 112233, "gameName": "AllyMid", "tagLine": "BR1" } }
        ],
        "participants": [
            {
                "participantId": 1, "teamId": 200, "championId": 238, "spell1Id": 4, "spell2Id": 14,
                "stats": {
                    "win": false, "kills": 0, "deaths": 4, "assists": 0, "totalMinionsKilled": 100,
                    "neutralMinionsKilled": 5, "goldEarned": 8500, "totalDamageDealtToChampions": 9000,
                    "visionScore": 12, "wardsPlaced": 4, "visionWardsBoughtInGame": 1, "champLevel": 12,
                    "item0": 3157, "item6": 3340, "perk0": 8112, "perkSubStyle": 8200
                }
            },
            {
                "participantId": 2, "teamId": 100, "championId": 103, "spell1Id": 4, "spell2Id": 12,
                "stats": {
                    "win": true, "kills": 6, "deaths": 1, "assists": 4, "totalMinionsKilled": 180,
                    "neutralMinionsKilled": 20, "goldEarned": 15200, "totalDamageDealtToChampions": 32000,
                    "visionScore": 28, "wardsPlaced": 9, "visionWardsBoughtInGame": 3, "champLevel": 17,
                    "item0": 6655, "item6": 3340, "perk0": 8112, "perkSubStyle": 8200
                }
            },
            {
                "participantId": 3, "teamId": 200, "championId": 81, "spell1Id": 4, "spell2Id": 7,
                "stats": {
                    "win": false, "kills": 0, "deaths": 6, "assists": 0, "totalMinionsKilled": 110,
                    "neutralMinionsKilled": 0, "goldEarned": 8100, "totalDamageDealtToChampions": 11000,
                    "visionScore": 10, "wardsPlaced": 3, "champLevel": 11, "item0": 0, "item6": 3340
                }
            },
            {
                "participantId": 4, "teamId": 100, "championId": 999, "spell1Id": 4, "spell2Id": 14,
                "stats": {
                    "win": true, "kills": 4, "deaths": 2, "assists": 4, "totalMinionsKilled": 150,
                    "neutralMinionsKilled": 10, "goldEarned": 13100, "totalDamageDealtToChampions": 19500,
                    "visionScore": 20, "wardsPlaced": 7, "champLevel": 16, "item0": 6655, "item6": 3340
                }
            }
        ]
    });

    let detail = parse_match_scoreboard_detail_for_locale(&payload, 987654321, None, &cache, AppLocale::PtBr)
        .expect("valid 5v5 scoreboard");

    assert!(!detail.is_arena && !detail.remake);
    assert_eq!(detail.max_damage_dealt_to_champions, 32000);
    assert_eq!(detail.teams.len(), 2);

    let blue = &detail.teams[0];
    assert_eq!(blue.team_id, 100);
    assert!(blue.win);
    assert_eq!((blue.towers, blue.dragons, blue.barons, blue.heralds, blue.voidgrubs, blue.inhibitors), (9, 3, 1, 1, 6, 2));
    assert_eq!(blue.total_kills, 10);
    assert_eq!(blue.total_gold, 28300);

    let local = &blue.participants[0];
    assert!(local.is_local_player);
    assert_eq!((local.riot_id_name.as_str(), local.riot_id_tag.as_str()), ("Schuma", "BR1"));
    assert_eq!((local.kp_percent, local.cs, local.trinket_id, local.primary_rune_id), (100, 200, 3340, 8112));

    let ally = &blue.participants[1];
    assert_eq!(ally.kp_percent, 80);
    assert_eq!(ally.champion_name, "Campeão 999");

    let red = &detail.teams[1];
    assert_eq!(red.team_id, 200);
    assert!(!red.win);
    assert_eq!(red.voidgrubs, 3);
    // Bots with summonerId: 0 must never be flagged as local, and KP% must stay 0 when team_kills == 0.
    assert!(!red.participants[0].is_local_player);
    assert_eq!(red.participants[0].riot_id_name, "Zed Bot");
    assert_eq!(red.participants[0].kp_percent, 0);
    assert_eq!(red.participants[1].riot_id_name, "Ezreal");

    assert!(detail.item_metadata.contains_key(&6655) && detail.item_metadata.contains_key(&3157) && detail.item_metadata.contains_key(&3340));
    assert!(detail.rune_metadata.contains_key(&8112) && detail.spell_metadata.contains_key(&4));
}

#[test]
fn test_parses_arena_duos_sorted_by_placement_with_puuid_match_and_no_fake_spell_icons() {
    let cache = sample_scoreboard_cache();
    let payload = serde_json::json!({
        "gameId": 8002, "queueId": 1700, "mapId": 30, "gameMode": "CHERRY",
        "gameDuration": 1200, "gameCreation": 1700000200000i64,
        "participantIdentities": [
            { "participantId": 1, "player": { "puuid": "opp-1", "gameName": "Duo4A", "tagLine": "NA1" } },
            { "participantId": 2, "player": { "puuid": "opp-2", "gameName": "Duo4B", "tagLine": "NA1" } },
            { "participantId": 3, "player": { "puuid": "local-puuid", "gameName": "Schuma", "tagLine": "BR1" } },
            { "participantId": 4, "player": { "puuid": "partner-puuid", "gameName": "Mate", "tagLine": "BR1" } }
        ],
        "participants": [
            {
                "participantId": 1, "teamId": 100, "championId": 238, "spell1Id": 2201, "spell2Id": 2202,
                "stats": { "win": false, "kills": 3, "deaths": 5, "assists": 2, "totalDamageDealtToChampions": 18000, "playerSubteamId": 4, "subteamPlacement": 4, "playerAugment1": 501, "item0": 3157 }
            },
            {
                "participantId": 2, "teamId": 100, "championId": 81, "spell1Id": 2201, "spell2Id": 2202,
                "stats": { "win": false, "kills": 2, "deaths": 5, "assists": 3, "totalDamageDealtToChampions": 14000, "playerSubteamId": 4, "subteamPlacement": 4, "item0": 6655 }
            },
            {
                "participantId": 3, "teamId": 100, "championId": 103, "spell1Id": 2201, "spell2Id": 2202,
                "stats": { "win": true, "kills": 10, "deaths": 2, "assists": 6, "totalDamageDealtToChampions": 46000, "playerSubteamId": 1, "subteamPlacement": 1, "playerAugment1": 501, "item0": 6655 }
            },
            {
                "participantId": 4, "teamId": 100, "championId": 81, "spell1Id": 2201, "spell2Id": 2202,
                "stats": { "win": true, "kills": 6, "deaths": 3, "assists": 8, "totalDamageDealtToChampions": 31000, "playerSubteamId": 1, "subteamPlacement": 1, "item0": 3157 }
            }
        ]
    });

    let detail = parse_match_scoreboard_detail_for_locale(&payload, 0, Some("local-puuid"), &cache, AppLocale::EnUs)
        .expect("valid arena scoreboard");

    assert!(detail.is_arena);
    assert_eq!(detail.game_mode_label, "Arena");
    assert_eq!(detail.max_damage_dealt_to_champions, 46000);
    assert!(detail.teams.is_empty());
    assert_eq!(detail.arena_teams.len(), 2);

    assert_eq!(detail.arena_teams[0].placement, 1);
    assert!(detail.arena_teams[0].win);
    assert_eq!(detail.arena_teams[0].total_damage, 77000);
    assert!(detail.arena_teams[0].participants[0].is_local_player);
    assert_eq!(detail.arena_teams[0].participants[0].augment_ids, vec![501]);
    // Unknown Arena spells must never fall back to SummonerFlash icon URLs
    assert!(detail.arena_teams[0].participants[0].spell1_icon_url.is_empty());
    assert!(!detail.spell_metadata.contains_key(&2201));

    assert_eq!(detail.arena_teams[1].placement, 4);
    assert!(detail.arena_teams[1].win);
    assert!(detail.augment_metadata.contains_key(&501));
}

#[test]
fn test_rejects_empty_participants_and_detects_remake_matches() {
    let cache = sample_scoreboard_cache();
    let empty_payload = serde_json::json!({ "gameId": 1, "participants": [] });
    assert!(parse_match_scoreboard_detail_for_locale(&empty_payload, 1, None, &cache, AppLocale::PtBr).is_err());

    let remake_payload = serde_json::json!({
        "gameId": 9003, "queueId": 420, "mapId": 11, "gameMode": "CLASSIC", "gameDuration": 185,
        "participants": [
            { "participantId": 1, "teamId": 100, "championId": 103, "stats": { "win": false, "gameEndedInEarlySurrender": true } },
            { "participantId": 6, "teamId": 200, "championId": 238, "stats": { "win": true } }
        ]
    });
    let detail = parse_match_scoreboard_detail_for_locale(&remake_payload, 0, None, &cache, AppLocale::PtBr)
        .expect("remake payload");
    assert!(detail.remake);
    assert_eq!(detail.teams.len(), 2);
}

use super::*;
use crate::models::{ChampionMetadata, QUEUE_CLASH, QUEUE_SOLO_DUO, QUEUE_SWIFTPLAY};
use std::collections::HashMap;

fn sample_ddragon() -> DataDragonCache {
    let mut champions = HashMap::new();
    champions.insert(
        103,
        ChampionMetadata {
            champion_id: 103,
            key_name: "Ahri".to_string(),
            display_name: "Ahri".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/16.23.1/img/champion/Ahri.png"
                .to_string(),
        },
    );
    champions.insert(
        222,
        ChampionMetadata {
            champion_id: 222,
            key_name: "Jinx".to_string(),
            display_name: "Jinx".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/16.23.1/img/champion/Jinx.png"
                .to_string(),
        },
    );
    DataDragonCache {
        version: "16.23.1".to_string(),
        champions,
        ..Default::default()
    }
}

#[test]
fn test_evaluate_lobby_ws_event_swiftplay_primary_and_secondary_slots() {
    let dd = sample_ddragon();
    let lobby_json = serde_json::json!({
        "gameConfig": {
            "queueId": QUEUE_SWIFTPLAY,
            "gameMode": "SWIFTPLAY"
        },
        "localMember": {
            "playerSlots": [
                {
                    "championId": 103,
                    "positionPreference": "MIDDLE",
                    "spell1": 4,
                    "spell2": 14,
                    "perks": "{}"
                },
                {
                    "championId": 222,
                    "positionPreference": "BOTTOM",
                    "spell1": 4,
                    "spell2": 7,
                    "perks": "{}"
                }
            ]
        }
    });

    let transition = evaluate_lobby_ws_event("Update", &lobby_json, None, Some(&dd));
    let LobbyPreselectTransition::Active(payload) = transition else {
        panic!("Expected active lobby transition");
    };

    assert_eq!(payload.champion_id, 103);
    assert_eq!(payload.queue_id, QUEUE_SWIFTPLAY);
    assert_eq!(payload.queue_mode, QueueMode::SummonersRift);
    assert!(payload.is_lobby_preselect);
    assert_eq!(payload.active_slot_index, Some(0));
    assert_eq!(payload.assigned_position.as_deref(), Some("mid"));
    assert_eq!(payload.lobby_slots.len(), 2);
    assert_eq!(payload.lobby_slots[0].champion_name, "Ahri");
    assert_eq!(payload.lobby_slots[1].champion_name, "Jinx");
    assert_eq!(
        payload.lobby_slots[1].assigned_position.as_deref(),
        Some("adc")
    );

    // Changing only spells/perks in LCU echo must not register as a payload change
    let mut updated_echo = lobby_json.clone();
    updated_echo["localMember"]["playerSlots"][0]["spell2"] = serde_json::json!(12);
    updated_echo["localMember"]["playerSlots"][0]["perks"] = serde_json::json!("{\"perkStyle\":8100}");
    let LobbyPreselectTransition::Active(echo_payload) =
        evaluate_lobby_ws_event("Update", &updated_echo, Some(0), Some(&dd))
    else {
        panic!("Expected active echo transition");
    };
    assert!(!lobby_payload_changed(Some(&payload), &echo_payload));

    // Preserving preferred_slot_index = 1 focuses Jinx
    let LobbyPreselectTransition::Active(slot1_payload) =
        evaluate_lobby_ws_event("Update", &lobby_json, Some(1), Some(&dd))
    else {
        panic!("Expected active slot 1 transition");
    };
    assert_eq!(slot1_payload.champion_id, 222);
    assert_eq!(slot1_payload.active_slot_index, Some(1));
    assert!(lobby_payload_changed(Some(&payload), &slot1_payload));
}

#[test]
fn test_evaluate_lobby_ws_event_ignores_non_preselect_queues_and_handles_fill() {
    let dd = sample_ddragon();
    for q in [QUEUE_SOLO_DUO, QUEUE_CLASH] {
        let non_preselect = serde_json::json!({
            "gameConfig": { "queueId": q, "gameMode": "CLASSIC" },
            "localMember": { "playerSlots": [{ "championId": 103, "positionPreference": "MIDDLE" }] }
        });
        assert_eq!(
            evaluate_lobby_ws_event("Update", &non_preselect, None, Some(&dd)),
            LobbyPreselectTransition::NotPreselectQueue
        );
    }

    let fill_lobby = serde_json::json!({
        "gameConfig": { "queueId": QUEUE_SWIFTPLAY, "gameMode": "SWIFTPLAY" },
        "localMember": {
            "playerSlots": [
                { "championId": 103, "positionPreference": "FILL" },
                { "championId": 0, "positionPreference": "UNSELECTED" }
            ]
        }
    });
    let LobbyPreselectTransition::Active(fill_payload) =
        evaluate_lobby_ws_event("Update", &fill_lobby, None, Some(&dd))
    else {
        panic!("Expected active fill transition");
    };
    assert_eq!(fill_payload.lobby_slots.len(), 1);
    assert_eq!(fill_payload.assigned_position, None);
    assert_eq!(
        evaluate_lobby_ws_event("Delete", &serde_json::json!({}), None, None),
        LobbyPreselectTransition::Ended
    );
}

#[test]
fn test_extract_assigned_match_champion_id_matches_summoner_or_puuid() {
    let gameflow = serde_json::json!({
        "gameData": {
            "teamOne": [
                { "summonerId": 11, "puuid": "other-puuid", "championId": 103 }
            ],
            "teamTwo": [
                { "summonerId": 42, "puuid": "local-puuid", "championId": 222 }
            ]
        }
    });
    assert_eq!(
        extract_assigned_match_champion_id(&gameflow, 42, None),
        Some(222)
    );
    assert_eq!(
        extract_assigned_match_champion_id(&gameflow, 0, Some("local-puuid")),
        Some(222)
    );
    assert_eq!(
        extract_assigned_match_champion_id(&gameflow, 999, Some("missing")),
        None
    );
}

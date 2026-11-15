use super::*;

#[test]
fn test_extract_queue_id_and_classify_mode() {
    let fixture_ranked = serde_json::json!({
        "gameData": {
            "queue": { "id": 420 }
        }
    });
    assert_eq!(extract_queue_id(&fixture_ranked), Ok(420));
    assert_eq!(QueueMode::from(420), QueueMode::SummonersRift);

    let fixture_aram = serde_json::json!({
        "gameData": {
            "queue": { "id": 450 }
        }
    });
    assert_eq!(extract_queue_id(&fixture_aram), Ok(450));
    assert_eq!(QueueMode::from(450), QueueMode::Aram);
}

#[test]
fn test_extract_queue_id_with_aram_mayhem_game_mode() {
    let fixture_mayhem = serde_json::json!({
        "gameData": {
            "queue": {
                "gameMode": "ARAM_MAYHEM",
                "id": 2400
            }
        }
    });
    assert_eq!(extract_queue_id(&fixture_mayhem), Ok(QUEUE_ARAM_MAYHEM));
    assert_eq!(QueueMode::from(QUEUE_ARAM_MAYHEM), QueueMode::AramMayhem);

    let fixture_mayhem_mode_only = serde_json::json!({
        "gameData": {
            "gameMode": "MAYHEM"
        }
    });
    assert_eq!(
        extract_queue_id(&fixture_mayhem_mode_only),
        Ok(QUEUE_ARAM_MAYHEM)
    );
}

#[test]
fn test_evaluate_ws_event_aram_mayhem() {
    let fixture_mayhem = serde_json::json!({
        "localPlayerCellId": 0,
        "actions": [],
        "myTeam": [
            { "cellId": 0, "championId": 222, "championPickIntent": 0 }
        ]
    });

    let transition = evaluate_ws_event("Update", &fixture_mayhem, QUEUE_ARAM_MAYHEM);
    assert_eq!(
        transition,
        WsSessionTransition::Active(ChampSelectedPayload {
            champion_id: 222,
            queue_id: QUEUE_ARAM_MAYHEM,
            queue_mode: QueueMode::AramMayhem,
            locked_in: true,
            assigned_position: None,
        })
    );
}

#[test]
fn test_evaluate_ws_event_create_update_and_delete() {
    let fixture_pick = serde_json::json!({
        "localPlayerCellId": 0,
        "actions": [
            [
                { "actorCellId": 0, "type": "pick", "championId": 103, "completed": false }
            ]
        ],
        "myTeam": []
    });

    let t_create = evaluate_ws_event("Create", &fixture_pick, 420);
    assert_eq!(
        t_create,
        WsSessionTransition::Active(ChampSelectedPayload {
            champion_id: 103,
            queue_id: 420,
            queue_mode: QueueMode::SummonersRift,
            locked_in: false,
            assigned_position: None,
        })
    );

    let t_delete = evaluate_ws_event("Delete", &serde_json::json!({}), 420);
    assert_eq!(t_delete, WsSessionTransition::Ended);
}

#[test]
fn test_extract_champion_from_practice_tool_session() {
    let fixture_practice = serde_json::json!({
        "localPlayerCellId": 0,
        "isCustomGame": true,
        "actions": [
            [
                { "actorCellId": 0, "type": "pick", "championId": 103, "completed": false }
            ]
        ],
        "myTeam": []
    });

    let result = extract_selected_champion(&fixture_practice);
    assert_eq!(result, Ok(Some((103, false))));
}

#[test]
fn test_is_custom_or_practice_session_and_fallback() {
    let fixture_custom = serde_json::json!({
        "localPlayerCellId": 0,
        "isCustomGame": true,
        "actions": [
            [
                { "actorCellId": 0, "type": "pick", "championId": 103, "completed": false }
            ]
        ]
    });

    assert!(is_custom_or_practice_session(&fixture_custom));

    let transition = evaluate_ws_event("Update", &fixture_custom, 0);
    assert_eq!(
        transition,
        WsSessionTransition::Active(ChampSelectedPayload {
            champion_id: 103,
            queue_id: QUEUE_PRACTICE_TOOL,
            queue_mode: QueueMode::PracticeTool,
            locked_in: false,
            assigned_position: None,
        })
    );
}

#[test]
fn test_extract_queue_id_with_normalized_game_mode() {
    let fixture_with_underscore = serde_json::json!({
        "gameData": {
            "queue": {
                "gameMode": "PRACTICE_TOOL",
                "id": -1
            }
        }
    });
    assert_eq!(
        extract_queue_id(&fixture_with_underscore),
        Ok(QUEUE_PRACTICE_TOOL)
    );
}

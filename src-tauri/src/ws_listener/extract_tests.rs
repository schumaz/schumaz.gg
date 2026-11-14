use super::*;

#[test]
fn test_extract_champion_from_solo_duo_active_pick_action() {
    let fixture = serde_json::json!({
        "localPlayerCellId": 2,
        "myTeam": [
            { "cellId": 0, "championId": 0, "championPickIntent": 0 },
            { "cellId": 1, "championId": 0, "championPickIntent": 0 },
            { "cellId": 2, "championId": 0, "championPickIntent": 0 }
        ],
        "actions": [
            [
                { "actorCellId": 0, "type": "ban", "championId": 157, "completed": true },
                { "actorCellId": 1, "type": "ban", "championId": 238, "completed": true },
                { "actorCellId": 2, "type": "ban", "championId": 11, "completed": true }
            ],
            [
                { "actorCellId": 0, "type": "pick", "championId": 51, "completed": true },
                { "actorCellId": 1, "type": "pick", "championId": 89, "completed": true },
                { "actorCellId": 2, "type": "pick", "championId": 103, "completed": false }
            ]
        ]
    });

    let result = extract_selected_champion(&fixture);
    assert_eq!(result, Ok(Some((103, false))));
}

#[test]
fn test_extract_champion_prefers_my_team_after_finalization_trade() {
    let fixture = serde_json::json!({
        "localPlayerCellId": 2,
        "myTeam": [
            { "cellId": 0, "championId": 103, "assignedPosition": "middle" },
            { "cellId": 2, "championId": 157, "assignedPosition": "top" }
        ],
        "actions": [
            [
                { "actorCellId": 2, "type": "pick", "championId": 103, "completed": true }
            ]
        ],
        "timer": {
            "phase": "FINALIZATION",
            "adjustedTimeLeftInPhase": 2500,
            "isInfinite": false
        }
    });

    assert_eq!(extract_selected_champion(&fixture), Ok(Some((157, true))));
}

#[test]
fn test_extract_champion_from_aram_my_team() {
    let fixture = serde_json::json!({
        "localPlayerCellId": 3,
        "actions": [],
        "myTeam": [
            { "cellId": 0, "championId": 22, "championPickIntent": 0 },
            { "cellId": 3, "championId": 222, "championPickIntent": 0 }
        ]
    });

    let result = extract_selected_champion(&fixture);
    assert_eq!(result, Ok(Some((222, true))));
}

#[test]
fn test_extract_champion_ignores_ban_actions() {
    let fixture = serde_json::json!({
        "localPlayerCellId": 1,
        "myTeam": [
            { "cellId": 1, "championId": 0, "championPickIntent": 0 }
        ],
        "actions": [
            [
                { "actorCellId": 1, "type": "ban", "championId": 157, "completed": false }
            ]
        ]
    });

    let result = extract_selected_champion(&fixture);
    assert_eq!(result, Ok(None));
}

#[test]
fn test_should_emit_champ_select_deduplicates_timer_ticks() {
    let p1 = Some(ChampSelectedPayload {
        champion_id: 103,
        queue_id: 420,
        queue_mode: QueueMode::SummonersRift,
        locked_in: false,
        assigned_position: Some("mid".to_string()),
    });
    let p2 = Some(ChampSelectedPayload {
        champion_id: 103,
        queue_id: 420,
        queue_mode: QueueMode::SummonersRift,
        locked_in: false,
        assigned_position: Some("mid".to_string()),
    });
    let p3_locked = Some(ChampSelectedPayload {
        champion_id: 103,
        queue_id: 420,
        queue_mode: QueueMode::SummonersRift,
        locked_in: true,
        assigned_position: Some("mid".to_string()),
    });

    assert!(!should_emit_champ_select(&p1, &p2));
    assert!(should_emit_champ_select(&p1, &p3_locked));
}

#[test]
fn test_extract_champ_select_timer_and_finalization_window() {
    let session_early = serde_json::json!({
        "timer": {
            "phase": "FINALIZATION",
            "adjustedTimeLeftInPhase": 18500,
            "isInfinite": false
        }
    });
    let timer_early = extract_champ_select_timer(&session_early).unwrap();
    assert_eq!(
        evaluate_finalization_window(&timer_early, FINALIZATION_SAFETY_WINDOW_MS),
        FinalizationAction::ScheduleAfter(Duration::from_millis(15_000))
    );

    let session_end = serde_json::json!({
        "timer": {
            "phase": "FINALIZATION",
            "adjustedTimeLeftInPhase": 3200,
            "isInfinite": false
        }
    });
    let timer_end = extract_champ_select_timer(&session_end).unwrap();
    assert_eq!(
        evaluate_finalization_window(&timer_end, FINALIZATION_SAFETY_WINDOW_MS),
        FinalizationAction::TriggerNow
    );

    let session_starting = serde_json::json!({
        "timer": {
            "phase": "GAME_STARTING",
            "adjustedTimeLeftInPhase": 0,
            "isInfinite": false
        }
    });
    let timer_starting = extract_champ_select_timer(&session_starting).unwrap();
    assert_eq!(
        evaluate_finalization_window(&timer_starting, FINALIZATION_SAFETY_WINDOW_MS),
        FinalizationAction::TriggerNow
    );

    let session_infinite = serde_json::json!({
        "timer": {
            "phase": "FINALIZATION",
            "adjustedTimeLeftInPhase": 0,
            "isInfinite": true
        }
    });
    let timer_infinite = extract_champ_select_timer(&session_infinite).unwrap();
    assert_eq!(
        evaluate_finalization_window(&timer_infinite, FINALIZATION_SAFETY_WINDOW_MS),
        FinalizationAction::NotApplicable
    );
}

#[test]
fn test_extract_assigned_position_and_normalization() {
    let fixture = serde_json::json!({
        "localPlayerCellId": 4,
        "myTeam": [
            { "cellId": 4, "assignedPosition": "utility" }
        ]
    });

    let pos = extract_assigned_position(&fixture);
    assert_eq!(pos, Some("support".to_string()));
}

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

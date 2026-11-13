pub use crate::models::{
    ChampSelectedPayload, QueueMode, QUEUE_ARAM, QUEUE_ARAM_MAYHEM, QUEUE_FLEX,
    QUEUE_NORMAL_DRAFT, QUEUE_PRACTICE_TOOL, QUEUE_QUICKPLAY, QUEUE_SOLO_DUO,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsSessionTransition {
    Active(ChampSelectedPayload),
    Ended,
    NoChampionSelected,
}

/// Evaluates raw WebSocket message details and determines session state changes.
/// Helper to detect if a session JSON represents a Custom Game or Practice Tool.
pub fn is_custom_or_practice_session(session_json: &serde_json::Value) -> bool {
    session_json
        .get("isCustomGame")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Evaluates raw WebSocket message details and determines session state changes.
pub fn evaluate_ws_event(
    event_type: &str,
    data: &serde_json::Value,
    current_queue_id: i32,
) -> WsSessionTransition {
    if event_type == "Delete" {
        return WsSessionTransition::Ended;
    }

    let effective_queue_id = if current_queue_id <= 0 && is_custom_or_practice_session(data) {
        QUEUE_PRACTICE_TOOL
    } else {
        current_queue_id
    };

    let assigned_pos = extract_assigned_position(data);

    match extract_selected_champion(data) {
        Ok(Some((champ_id, locked_in))) => WsSessionTransition::Active(ChampSelectedPayload {
            champion_id: champ_id,
            queue_id: effective_queue_id,
            queue_mode: QueueMode::from(effective_queue_id),
            locked_in,
            assigned_position: assigned_pos,
        }),
        Ok(None) => WsSessionTransition::NoChampionSelected,
        Err(_) => WsSessionTransition::NoChampionSelected,
    }
}

/// Deduplicates timer ticks and only triggers an emit when champion or lock-in state transitions.
pub fn should_emit_champ_select(
    last_payload: &Option<ChampSelectedPayload>,
    current_payload: &Option<ChampSelectedPayload>,
) -> bool {
    last_payload != current_payload
}

/// Pure parser function to extract the selected `championId` and `locked_in` status.
///
/// Handles:
/// 1. Draft and Practice Tool pick actions via `actions` where `type == "pick"` and `actorCellId == localPlayerCellId`.
/// 2. ARAM / Blind Pick / Intent fallback via `myTeam` where `cellId == localPlayerCellId`.
///
/// Explicitly ignores `"ban"` actions so banning a champion never triggers a build recommendation.
pub fn extract_selected_champion(
    session_json: &serde_json::Value,
) -> Result<Option<(i32, bool)>, String> {
    let local_player_cell_id = session_json
        .get("localPlayerCellId")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "Missing 'localPlayerCellId' in session JSON".to_string())?
        as i32;

    // 1. Check draft actions (Solo/Duo, Flex, Normal Draft, Practice Tool)
    if let Some(actions) = session_json.get("actions").and_then(|v| v.as_array()) {
        for action_group in actions {
            if let Some(sub_actions) = action_group.as_array() {
                for action in sub_actions {
                    let action_type = action.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    let actor_cell_id = action
                        .get("actorCellId")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(-1) as i32;

                    // Only consider "pick" actions for the local player (strictly ignore "ban")
                    if action_type == "pick" && actor_cell_id == local_player_cell_id {
                        let champ_id = action
                            .get("championId")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0) as i32;
                        let completed = action
                            .get("completed")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        if champ_id > 0 {
                            return Ok(Some((champ_id, completed)));
                        }
                    }
                }
            }
        }
    }

    // 2. Check `myTeam` (works for ARAM, Blind Pick, Practice Tool without actions, or draft intent)
    if let Some(my_team) = session_json.get("myTeam").and_then(|v| v.as_array()) {
        for member in my_team {
            let cell_id = member.get("cellId").and_then(|v| v.as_i64()).unwrap_or(-1) as i32;

            if cell_id == local_player_cell_id {
                let champ_id = member
                    .get("championId")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;

                // In ARAM or locked pick, championId > 0
                if champ_id > 0 {
                    return Ok(Some((champ_id, true)));
                }

                // In draft, before lock, check championPickIntent
                let intent_id = member
                    .get("championPickIntent")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;
                if intent_id > 0 {
                    return Ok(Some((intent_id, false)));
                }
            }
        }
    }

    Ok(None)
}

/// Normalizes draft `assignedPosition` strings ("top", "jungle", "middle", "bottom", "utility")
/// to standard OP.GG positions ("top", "jungle", "mid", "adc", "support").
pub fn normalize_position(pos: &str) -> Option<String> {
    let lower = pos.trim().to_lowercase();
    match lower.as_str() {
        "top" => Some("top".to_string()),
        "jungle" | "jng" => Some("jungle".to_string()),
        "middle" | "mid" => Some("mid".to_string()),
        "bottom" | "bot" | "adc" => Some("adc".to_string()),
        "utility" | "support" | "supp" => Some("support".to_string()),
        _ => None,
    }
}

/// Extracts the local player's `assignedPosition` from the draft `myTeam` payload.
pub fn extract_assigned_position(session_json: &serde_json::Value) -> Option<String> {
    let local_player_cell_id = session_json
        .get("localPlayerCellId")
        .and_then(|v| v.as_i64())? as i32;

    let my_team = session_json.get("myTeam").and_then(|v| v.as_array())?;
    for member in my_team {
        let cell_id = member.get("cellId").and_then(|v| v.as_i64()).unwrap_or(-1) as i32;
        if cell_id == local_player_cell_id
            && let Some(pos) = member.get("assignedPosition").and_then(|v| v.as_str())
        {
            return normalize_position(pos);
        }
    }
    None
}

/// Extracts the queue ID from a `GET /lol-gameflow/v1/session` JSON payload.
/// Supports Standard queues, ARAM Mayhem, Practice Tool, and Custom games.
pub fn extract_queue_id(gameflow_json: &serde_json::Value) -> Result<i32, String> {
    // 1. Check for Practice Tool, ARAM Mayhem, or custom game indicators in gameMode
    let game_mode = gameflow_json
        .pointer("/gameData/queue/gameMode")
        .or_else(|| gameflow_json.pointer("/gameData/gameMode"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if game_mode.eq_ignore_ascii_case("PRACTICETOOL")
        || game_mode.eq_ignore_ascii_case("PRACTICE_TOOL")
    {
        return Ok(QUEUE_PRACTICE_TOOL);
    }

    if game_mode.eq_ignore_ascii_case("ARAM_MAYHEM") || game_mode.to_uppercase().contains("MAYHEM")
    {
        return Ok(QUEUE_ARAM_MAYHEM);
    }

    let is_custom = gameflow_json
        .pointer("/gameData/isCustomGame")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let id_opt = gameflow_json
        .pointer("/gameData/queue/id")
        .and_then(|v| v.as_i64())
        .map(|id| id as i32);

    if let Some(id) = id_opt {
        if id == -1 || (is_custom && id <= 0) {
            return Ok(QUEUE_PRACTICE_TOOL);
        }
        return Ok(id);
    }

    if is_custom {
        return Ok(QUEUE_PRACTICE_TOOL);
    }

    Err("Could not find '/gameData/queue/id' in gameflow session JSON".to_string())
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_extract_champion_from_solo_duo_active_pick_action() {
        // Real fixture snippet from Solo/Duo champion select
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
    fn test_extract_champion_from_aram_my_team() {
        // In ARAM, champion is directly assigned in `myTeam`
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
        // When local player is currently banning, it should NOT be detected as a pick
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

        // Also handles when id is arbitrary or not standard but gameMode is ARAM_MAYHEM
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

        // When current_queue_id is 0, it falls back to QUEUE_PRACTICE_TOOL
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
}

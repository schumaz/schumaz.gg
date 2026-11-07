//! Module 2: LCU WebSocket Event Listener
//! Listens to OnJsonApiEvent_lol-champ-select_v1_session and OnJsonApiEvent_lol-gameflow_v1_session.

use serde::{Deserialize, Serialize};

/// Represents common League of Legends queue modes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QueueMode {
    SoloDuo,
    Flex,
    Aram,
    NormalDraft,
    Quickplay,
    Other(i32),
}

impl QueueMode {
    pub fn from_queue_id(queue_id: i32) -> Self {
        match queue_id {
            420 => QueueMode::SoloDuo,
            440 => QueueMode::Flex,
            450 => QueueMode::Aram,
            400 => QueueMode::NormalDraft,
            490 => QueueMode::Quickplay,
            other => QueueMode::Other(other),
        }
    }
}

/// Payload emitted to the frontend when a champion is selected or locked in.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChampSelectedPayload {
    pub champion_id: i32,
    pub queue_id: i32,
    pub queue_mode: QueueMode,
    pub is_locked: bool,
}

/// Extracts the active champion selection for the local player from a `champ-select/v1/session` JSON payload.
/// Returns Ok(Some((champion_id, is_locked))) if a champion is selected, or Ok(None) if unselected.
pub fn extract_selected_champion(
    session_json: &serde_json::Value,
) -> Result<Option<(i32, bool)>, String> {
    let local_player_cell_id = session_json
        .get("localPlayerCellId")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| {
            "Missing or invalid 'localPlayerCellId' in champ-select session".to_string()
        })? as i32;

    // 1. Check myTeam directly (used in ARAM, Quickplay, or when pick is completed)
    if let Some(my_team) = session_json.get("myTeam").and_then(|v| v.as_array()) {
        for member in my_team {
            let cell_id = member.get("cellId").and_then(|v| v.as_i64()).unwrap_or(-1) as i32;
            if cell_id == local_player_cell_id {
                let champ_id = member
                    .get("championId")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;
                if champ_id > 0 {
                    return Ok(Some((champ_id, true)));
                }

                // Also check pick intent (hovered champion before lock in)
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

    // 2. Check draft actions (Solo/Duo, Flex, Normal Draft)
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

    Ok(None)
}

/// Extracts the queue ID from a `GET /lol-gameflow/v1/session` JSON payload.
pub fn extract_queue_id(gameflow_json: &serde_json::Value) -> Result<i32, String> {
    gameflow_json
        .pointer("/gameData/queue/id")
        .and_then(|v| v.as_i64())
        .map(|id| id as i32)
        .ok_or_else(|| "Could not find '/gameData/queue/id' in gameflow session JSON".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_champion_from_solo_duo_active_pick_action() {
        let fixture = serde_json::json!({
            "localPlayerCellId": 2,
            "myTeam": [
                { "cellId": 0, "championId": 0, "championPickIntent": 0 },
                { "cellId": 1, "championId": 0, "championPickIntent": 0 },
                { "cellId": 2, "championId": 0, "championPickIntent": 0 },
                { "cellId": 3, "championId": 0, "championPickIntent": 0 },
                { "cellId": 4, "championId": 0, "championPickIntent": 0 }
            ],
            "actions": [
                [
                    { "actorCellId": 0, "championId": 157, "type": "ban", "completed": true },
                    { "actorCellId": 2, "championId": 238, "type": "ban", "completed": true }
                ],
                [
                    { "actorCellId": 2, "championId": 103, "type": "pick", "completed": false, "isInProgress": true }
                ]
            ]
        });

        let result = extract_selected_champion(&fixture);
        assert!(result.is_ok());
        let champion = result.unwrap();
        assert_eq!(champion, Some((103, false))); // Ahri (103), hovered/unlocked
    }

    #[test]
    fn test_extract_champion_from_aram_my_team() {
        let fixture = serde_json::json!({
            "localPlayerCellId": 3,
            "myTeam": [
                { "cellId": 0, "championId": 1, "championPickIntent": 0 },
                { "cellId": 1, "championId": 22, "championPickIntent": 0 },
                { "cellId": 2, "championId": 51, "championPickIntent": 0 },
                { "cellId": 3, "championId": 222, "championPickIntent": 0 },
                { "cellId": 4, "championId": 81, "championPickIntent": 0 }
            ],
            "actions": []
        });

        let result = extract_selected_champion(&fixture);
        assert!(result.is_ok());
        let champion = result.unwrap();
        assert_eq!(champion, Some((222, true))); // Jinx (222), auto-locked in ARAM
    }

    #[test]
    fn test_extract_champion_ignores_ban_actions() {
        // Player is banning Yasuo (157), but has not picked any champion yet
        let fixture = serde_json::json!({
            "localPlayerCellId": 1,
            "myTeam": [
                { "cellId": 1, "championId": 0, "championPickIntent": 0 }
            ],
            "actions": [
                [
                    { "actorCellId": 1, "championId": 157, "type": "ban", "completed": false, "isInProgress": true }
                ]
            ]
        });

        let result = extract_selected_champion(&fixture);
        assert!(result.is_ok());
        let champion = result.unwrap();
        assert_eq!(champion, None);
    }

    #[test]
    fn test_extract_queue_id_and_classify_mode() {
        let make_gameflow = |id: i32| {
            serde_json::json!({
                "gameData": {
                    "queue": {
                        "id": id,
                        "type": "NORMAL"
                    }
                }
            })
        };

        assert_eq!(extract_queue_id(&make_gameflow(420)).unwrap(), 420);
        assert_eq!(QueueMode::from_queue_id(420), QueueMode::SoloDuo);

        assert_eq!(extract_queue_id(&make_gameflow(440)).unwrap(), 440);
        assert_eq!(QueueMode::from_queue_id(440), QueueMode::Flex);

        assert_eq!(extract_queue_id(&make_gameflow(450)).unwrap(), 450);
        assert_eq!(QueueMode::from_queue_id(450), QueueMode::Aram);

        assert_eq!(extract_queue_id(&make_gameflow(400)).unwrap(), 400);
        assert_eq!(QueueMode::from_queue_id(400), QueueMode::NormalDraft);

        assert_eq!(extract_queue_id(&make_gameflow(9999)).unwrap(), 9999);
        assert_eq!(QueueMode::from_queue_id(9999), QueueMode::Other(9999));
    }
}

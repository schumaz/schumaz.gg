//! Module 2: LCU WebSocket Event Listener
//! Listens to OnJsonApiEvent_lol-champ-select_v1_session and OnJsonApiEvent_lol-gameflow_v1_session.

use irelia::ws::LcuWebSocket;
use irelia::ws::types::{Event, EventKind};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;
use tauri::Emitter;

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

/// Evaluated transition result from an incoming WebSocket message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsSessionTransition {
    Active(ChampSelectedPayload),
    NoChampionSelected,
    Ended,
}

/// Determines whether a new champion select event should be emitted to avoid flooding.
pub fn should_emit_champ_select(
    prev: &Option<ChampSelectedPayload>,
    current: &Option<ChampSelectedPayload>,
) -> bool {
    prev != current
}

/// Pure evaluation of incoming WebSocket payload from `OnJsonApiEvent_lol-champ-select_v1_session`.
pub fn evaluate_ws_event(
    event_type: &str,
    data: &serde_json::Value,
    current_queue_id: i32,
) -> WsSessionTransition {
    if event_type.eq_ignore_ascii_case("Delete") {
        return WsSessionTransition::Ended;
    }

    match extract_selected_champion(data) {
        Ok(Some((champ_id, is_locked))) => WsSessionTransition::Active(ChampSelectedPayload {
            champion_id: champ_id,
            queue_id: current_queue_id,
            queue_mode: QueueMode::from_queue_id(current_queue_id),
            is_locked,
        }),
        _ => WsSessionTransition::NoChampionSelected,
    }
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

/// Starts the WebSocket listener background task to observe Champion Select events in real time.
pub async fn start_ws_listener(app_handle: tauri::AppHandle) {
    let mut ws = LcuWebSocket::new();

    let active_queue_id = Arc::new(AtomicI32::new(0));
    let last_payload: Arc<tokio::sync::Mutex<Option<ChampSelectedPayload>>> =
        Arc::new(tokio::sync::Mutex::new(None));

    let app_handle_clone = app_handle.clone();
    let queue_id_ref = Arc::clone(&active_queue_id);
    let last_ref = Arc::clone(&last_payload);

    ws.subscribe(
        EventKind::json_api_event_callback_str("/lol-champ-select/v1/session"),
        move |event: &Event| {
            let handle = app_handle_clone.clone();
            let q_ref = Arc::clone(&queue_id_ref);
            let last_payload_ref = Arc::clone(&last_ref);

            let event_type = event.2.event_type.clone();
            let data = event.2.data.clone();

            tokio::spawn(async move {
                // If queueId is unknown, attempt to fetch it once
                let mut current_q = q_ref.load(Ordering::Relaxed);
                if current_q == 0 {
                    if let Ok(conn) = crate::lcu_connector::LcuConnection::connect() {
                        if let Ok(fetched_q) = conn.get_current_queue_id().await {
                            q_ref.store(fetched_q, Ordering::Relaxed);
                            current_q = fetched_q;
                        }
                    }
                }

                match evaluate_ws_event(&event_type, &data, current_q) {
                    WsSessionTransition::Active(payload) => {
                        let mut last_guard = last_payload_ref.lock().await;
                        let current_opt = Some(payload.clone());
                        if should_emit_champ_select(&last_guard, &current_opt) {
                            let _ = handle.emit("champ-selected", &payload);
                            *last_guard = current_opt;
                        }
                    }
                    WsSessionTransition::Ended => {
                        let mut last_guard = last_payload_ref.lock().await;
                        if last_guard.is_some() {
                            let _ = handle.emit("champ-select-ended", ());
                            *last_guard = None;
                        }
                        q_ref.store(0, Ordering::Relaxed);
                    }
                    WsSessionTransition::NoChampionSelected => {}
                }
            });
        },
    );

    // Keep the task alive in background
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
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

    #[test]
    fn test_should_emit_champ_select_deduplicates_timer_ticks() {
        let payload1 = ChampSelectedPayload {
            champion_id: 103,
            queue_id: 420,
            queue_mode: QueueMode::SoloDuo,
            is_locked: false,
        };

        let payload2_same = payload1.clone();

        let payload3_locked = ChampSelectedPayload {
            champion_id: 103,
            queue_id: 420,
            queue_mode: QueueMode::SoloDuo,
            is_locked: true,
        };

        let payload4_swapped = ChampSelectedPayload {
            champion_id: 222,
            queue_id: 420,
            queue_mode: QueueMode::SoloDuo,
            is_locked: true,
        };

        // First selection: should emit
        assert!(should_emit_champ_select(&None, &Some(payload1.clone())));

        // Identical timer tick update: should NOT emit
        assert!(!should_emit_champ_select(
            &Some(payload1.clone()),
            &Some(payload2_same)
        ));

        // Player locks in: should emit
        assert!(should_emit_champ_select(
            &Some(payload1.clone()),
            &Some(payload3_locked.clone())
        ));

        // Player swaps champion: should emit
        assert!(should_emit_champ_select(
            &Some(payload3_locked.clone()),
            &Some(payload4_swapped.clone())
        ));

        // Session reset: should emit
        assert!(should_emit_champ_select(&Some(payload4_swapped), &None));
    }

    #[test]
    fn test_evaluate_ws_event_create_update_and_delete() {
        let session_data = serde_json::json!({
            "localPlayerCellId": 0,
            "myTeam": [
                { "cellId": 0, "championId": 84, "championPickIntent": 0 }
            ],
            "actions": []
        });

        // Update event with selected champion
        let transition = evaluate_ws_event("Update", &session_data, 450);
        assert_eq!(
            transition,
            WsSessionTransition::Active(ChampSelectedPayload {
                champion_id: 84,
                queue_id: 450,
                queue_mode: QueueMode::Aram,
                is_locked: true
            })
        );

        // Delete event indicates champ select ended
        let end_transition = evaluate_ws_event("Delete", &serde_json::Value::Null, 450);
        assert_eq!(end_transition, WsSessionTransition::Ended);
    }
}

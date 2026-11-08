//! Module 2: Champion Select and Queue Detection via WebSocket
//! Handles real-time monitoring of LoL client lobby and draft states without polling.

use irelia::ws::LcuWebSocket;
use irelia::ws::types::{Event, EventKind};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;
use tauri::Emitter;

pub const QUEUE_SOLO_DUO: i32 = 420;
pub const QUEUE_FLEX: i32 = 440;
pub const QUEUE_ARAM: i32 = 450;
pub const QUEUE_NORMAL_DRAFT: i32 = 400;
pub const QUEUE_QUICKPLAY: i32 = 490;
pub const QUEUE_PRACTICE_TOOL: i32 = -1;

/// Represents game queue classifications.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum QueueMode {
    SummonersRift,
    Aram,
    PracticeTool,
    Other,
}

impl From<i32> for QueueMode {
    fn from(queue_id: i32) -> Self {
        match queue_id {
            QUEUE_ARAM => QueueMode::Aram,
            QUEUE_PRACTICE_TOOL => QueueMode::PracticeTool,
            QUEUE_SOLO_DUO | QUEUE_FLEX | QUEUE_NORMAL_DRAFT | QUEUE_QUICKPLAY => {
                QueueMode::SummonersRift
            }
            _ => QueueMode::Other,
        }
    }
}

/// Payload emitted to the Tauri frontend when the user selects or locks in a champion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChampSelectedPayload {
    pub champion_id: i32,
    pub queue_id: i32,
    pub queue_mode: QueueMode,
    pub locked_in: bool,
}

/// Represents state transitions evaluated from raw WebSocket events.
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

    match extract_selected_champion(data) {
        Ok(Some((champ_id, locked_in))) => WsSessionTransition::Active(ChampSelectedPayload {
            champion_id: champ_id,
            queue_id: effective_queue_id,
            queue_mode: QueueMode::from(effective_queue_id),
            locked_in,
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

/// Extracts the queue ID from a `GET /lol-gameflow/v1/session` JSON payload.
/// Supports Standard queues, Practice Tool, and Custom games.
pub fn extract_queue_id(gameflow_json: &serde_json::Value) -> Result<i32, String> {
    // 1. Check for Practice Tool or custom game indicators
    let game_mode = gameflow_json
        .pointer("/gameData/queue/gameMode")
        .or_else(|| gameflow_json.pointer("/gameData/gameMode"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if game_mode.eq_ignore_ascii_case("PRACTICETOOL") {
        return Ok(QUEUE_PRACTICE_TOOL);
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

/// Custom error handler for irelia WebSocket that retries connection on transient drops.
struct ReconnectingErrorHandler;

impl irelia::ws::ErrorHandler for ReconnectingErrorHandler {
    fn on_error(
        &mut self,
        _error: irelia::ws::WebSocketError,
    ) -> std::ops::ControlFlow<(), irelia::ws::Flow> {
        std::thread::sleep(Duration::from_millis(1500));
        std::ops::ControlFlow::Continue(irelia::ws::Flow::TryReconnect)
    }
}

/// Dispatches session state changes and triggers automated build injection when applicable.
async fn handle_session_transition(
    handle: &tauri::AppHandle,
    transition: WsSessionTransition,
    last_payload_ref: &Arc<tokio::sync::Mutex<Option<ChampSelectedPayload>>>,
    q_ref: &Arc<AtomicI32>,
) {
    match transition {
        WsSessionTransition::Active(payload) => {
            let mut last_guard = last_payload_ref.lock().await;
            let current_opt = Some(payload.clone());
            if should_emit_champ_select(&last_guard, &current_opt) {
                let _ = handle.emit("champ-selected", &payload);
                *last_guard = current_opt;

                // Automatically fetch recommended build and inject when locked in or in Practice Tool
                if payload.locked_in || payload.queue_mode == QueueMode::PracticeTool {
                    let handle_for_inject = handle.clone();
                    let champ_id = payload.champion_id;
                    let q_id = if payload.queue_id == 0 || payload.queue_id == QUEUE_PRACTICE_TOOL {
                        QUEUE_SOLO_DUO
                    } else {
                        payload.queue_id
                    };

                    tokio::spawn(async move {
                        let Ok(conn) = crate::lcu_connector::LcuConnection::connect() else {
                            return;
                        };
                        let Ok(summoner) = conn.get_current_summoner().await else {
                            return;
                        };
                        let http_client = reqwest::Client::new();
                        let Ok(build) = crate::build_provider::get_recommended_build(
                            &http_client,
                            champ_id,
                            q_id,
                        )
                        .await
                        else {
                            return;
                        };

                        let _ = handle_for_inject.emit("recommended-build", &build);
                        let report = crate::lcu_injector::inject_champion_build(
                            &conn,
                            summoner.summoner_id,
                            &build,
                        )
                        .await;
                        let _ = handle_for_inject.emit("build-injected", &report);
                    });
                }
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
}

/// Starts the WebSocket listener and fallback polling task to observe Champion Select events in real time.
pub async fn start_ws_listener(app_handle: tauri::AppHandle) {
    let active_queue_id = Arc::new(AtomicI32::new(0));
    let last_payload: Arc<tokio::sync::Mutex<Option<ChampSelectedPayload>>> =
        Arc::new(tokio::sync::Mutex::new(None));

    // Fallback sync loop: Ensures Champion Select (especially Practice Tool) is detected
    // even if the WebSocket missed an event, was mid-reconnection, or client was opened later.
    let fallback_handle = app_handle.clone();
    let fallback_q_ref = Arc::clone(&active_queue_id);
    let fallback_last_ref = Arc::clone(&last_payload);

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(1500)).await;

            let Ok(conn) = crate::lcu_connector::LcuConnection::connect() else {
                continue;
            };

            match conn.get_champ_select_session().await {
                Ok(Some(session_json)) => {
                    let mut current_q = fallback_q_ref.load(Ordering::Relaxed);
                    if current_q <= 0 {
                        let maybe_q = conn.get_current_queue_id().await.ok();
                        if let Some(fetched_q) = maybe_q {
                            fallback_q_ref.store(fetched_q, Ordering::Relaxed);
                            current_q = fetched_q;
                        }
                    }

                    let transition = evaluate_ws_event("Update", &session_json, current_q);
                    handle_session_transition(
                        &fallback_handle,
                        transition,
                        &fallback_last_ref,
                        &fallback_q_ref,
                    )
                    .await;
                }
                Ok(None) => {
                    let mut last_guard = fallback_last_ref.lock().await;
                    if last_guard.is_some() {
                        let _ = fallback_handle.emit("champ-select-ended", ());
                        *last_guard = None;
                    }
                    fallback_q_ref.store(0, Ordering::Relaxed);
                }
                Err(_) => {}
            }
        }
    });

    // Real-time WebSocket listener with auto-reconnecting error handler
    let mut ws = LcuWebSocket::new_with_error_handler(ReconnectingErrorHandler);
    let ws_handle = app_handle.clone();
    let ws_q_ref = Arc::clone(&active_queue_id);
    let ws_last_ref = Arc::clone(&last_payload);

    ws.subscribe(
        EventKind::json_api_event_callback_str("/lol-champ-select/v1/session"),
        move |event: &Event| {
            let handle = ws_handle.clone();
            let q_ref = Arc::clone(&ws_q_ref);
            let last_payload_ref = Arc::clone(&ws_last_ref);

            let event_type = event.2.event_type.clone();
            let data = event.2.data.clone();

            tokio::spawn(async move {
                let mut current_q = q_ref.load(Ordering::Relaxed);
                if current_q <= 0 {
                    let maybe_fetched = match crate::lcu_connector::LcuConnection::connect() {
                        Ok(conn) => conn.get_current_queue_id().await.ok(),
                        Err(_) => None,
                    };
                    if let Some(fetched_q) = maybe_fetched {
                        q_ref.store(fetched_q, Ordering::Relaxed);
                        current_q = fetched_q;
                    }
                }

                let transition = evaluate_ws_event(&event_type, &data, current_q);
                handle_session_transition(&handle, transition, &last_payload_ref, &q_ref).await;
            });
        },
    );

    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

#[cfg(test)]
mod tests {
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
                    { "actorCellId": 0, "type": "ban", "championId": 64, "completed": true }
                ],
                [
                    { "actorCellId": 2, "type": "pick", "championId": 103, "completed": false }
                ]
            ]
        });

        let result = extract_selected_champion(&fixture);
        assert_eq!(result, Ok(Some((103, false))));
    }

    #[test]
    fn test_extract_champion_ignores_ban_actions() {
        // If the local player is currently banning a champion, it must NOT be extracted as their pick
        let fixture = serde_json::json!({
            "localPlayerCellId": 3,
            "myTeam": [
                { "cellId": 3, "championId": 0, "championPickIntent": 0 }
            ],
            "actions": [
                [
                    { "actorCellId": 3, "type": "ban", "championId": 119, "completed": false }
                ]
            ]
        });

        let result = extract_selected_champion(&fixture);
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn test_extract_champion_from_aram_my_team() {
        // Real fixture snippet from ARAM where championId is directly assigned on myTeam
        let fixture = serde_json::json!({
            "localPlayerCellId": 4,
            "myTeam": [
                { "cellId": 0, "championId": 222, "championPickIntent": 0 },
                { "cellId": 4, "championId": 51, "championPickIntent": 0 }
            ],
            "actions": []
        });

        let result = extract_selected_champion(&fixture);
        // ARAM is automatically locked in (true)
        assert_eq!(result, Ok(Some((51, true))));
    }

    #[test]
    fn test_extract_champion_from_practice_tool_session() {
        // Practice Tool session snippet where player picks a champion
        let fixture_practice = serde_json::json!({
            "localPlayerCellId": 0,
            "myTeam": [
                { "cellId": 0, "championId": 103, "championPickIntent": 0 }
            ],
            "actions": [
                [
                    { "actorCellId": 0, "type": "pick", "championId": 103, "completed": false }
                ]
            ]
        });

        let result = extract_selected_champion(&fixture_practice);
        assert_eq!(result, Ok(Some((103, false))));

        // When locked in
        let fixture_locked = serde_json::json!({
            "localPlayerCellId": 0,
            "myTeam": [
                { "cellId": 0, "championId": 103, "championPickIntent": 0 }
            ],
            "actions": [
                [
                    { "actorCellId": 0, "type": "pick", "championId": 103, "completed": true }
                ]
            ]
        });

        let result_locked = extract_selected_champion(&fixture_locked);
        assert_eq!(result_locked, Ok(Some((103, true))));
    }

    #[test]
    fn test_extract_queue_id_and_classify_mode() {
        let fixture_aram = serde_json::json!({
            "gameData": {
                "queue": {
                    "id": 450
                }
            }
        });
        let queue_id = extract_queue_id(&fixture_aram).expect("Queue id should parse");
        assert_eq!(queue_id, 450);
        assert_eq!(QueueMode::from(queue_id), QueueMode::Aram);

        let fixture_ranked = serde_json::json!({
            "gameData": {
                "queue": {
                    "id": 420
                }
            }
        });
        let queue_id_ranked =
            extract_queue_id(&fixture_ranked).expect("Ranked queue id should parse");
        assert_eq!(queue_id_ranked, 420);
        assert_eq!(QueueMode::from(queue_id_ranked), QueueMode::SummonersRift);

        // Practice Tool with id: -1 and gameMode: "PRACTICETOOL"
        let fixture_practice_tool = serde_json::json!({
            "gameData": {
                "isCustomGame": true,
                "queue": {
                    "id": -1,
                    "gameMode": "PRACTICETOOL"
                }
            }
        });
        let queue_id_practice =
            extract_queue_id(&fixture_practice_tool).expect("Practice tool queue id should parse");
        assert_eq!(queue_id_practice, QUEUE_PRACTICE_TOOL);
        assert_eq!(QueueMode::from(queue_id_practice), QueueMode::PracticeTool);

        // Practice Tool with gameMode on root gameData
        let fixture_practice_tool_root = serde_json::json!({
            "gameData": {
                "isCustomGame": true,
                "gameMode": "PRACTICETOOL"
            }
        });
        let queue_id_practice_root = extract_queue_id(&fixture_practice_tool_root)
            .expect("Practice tool queue id should parse");
        assert_eq!(queue_id_practice_root, QUEUE_PRACTICE_TOOL);
        assert_eq!(
            QueueMode::from(queue_id_practice_root),
            QueueMode::PracticeTool
        );
    }

    #[test]
    fn test_should_emit_champ_select_deduplicates_timer_ticks() {
        let payload1 = Some(ChampSelectedPayload {
            champion_id: 103,
            queue_id: 420,
            queue_mode: QueueMode::SummonersRift,
            locked_in: false,
        });

        // Same payload received in a subsequent timer tick
        let payload1_tick = Some(ChampSelectedPayload {
            champion_id: 103,
            queue_id: 420,
            queue_mode: QueueMode::SummonersRift,
            locked_in: false,
        });

        assert!(!should_emit_champ_select(&payload1, &payload1_tick));

        // State changes to locked_in: true
        let payload2 = Some(ChampSelectedPayload {
            champion_id: 103,
            queue_id: 420,
            queue_mode: QueueMode::SummonersRift,
            locked_in: true,
        });

        assert!(should_emit_champ_select(&payload1, &payload2));
    }

    #[test]
    fn test_evaluate_ws_event_create_update_and_delete() {
        let fixture_active = serde_json::json!({
            "localPlayerCellId": 1,
            "myTeam": [
                { "cellId": 1, "championId": 84, "championPickIntent": 0 }
            ],
            "actions": []
        });

        let transition = evaluate_ws_event("Update", &fixture_active, 450);
        assert_eq!(
            transition,
            WsSessionTransition::Active(ChampSelectedPayload {
                champion_id: 84,
                queue_id: 450,
                queue_mode: QueueMode::Aram,
                locked_in: true,
            })
        );

        let transition_end = evaluate_ws_event("Delete", &serde_json::json!({}), 450);
        assert_eq!(transition_end, WsSessionTransition::Ended);
    }

    #[test]
    fn test_is_custom_or_practice_session_and_fallback() {
        let fixture_custom = serde_json::json!({
            "isCustomGame": true,
            "localPlayerCellId": 0,
            "myTeam": [
                { "cellId": 0, "championId": 103, "championPickIntent": 0 }
            ],
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

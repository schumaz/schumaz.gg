use irelia::ws::LcuWebSocket;
use irelia::ws::types::{Event, EventKind};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;
use tauri::Emitter;

use super::extract::*;

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
                if let Some(prev) = last_guard.as_ref()
                    && prev.champion_id != payload.champion_id
                {
                    crate::lcu_injector::clear_champion_selections();
                }
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
                    let assigned_pos = payload.assigned_position.clone();

                    tokio::spawn(async move {
                        let Ok(conn) = crate::lcu_connector::LcuConnection::connect() else {
                            return;
                        };
                        let Ok(summoner) = conn.get_current_summoner().await else {
                            return;
                        };
                        let http_client = reqwest::Client::new();
                        let Ok(build) = crate::build_provider::get_recommended_build_with_filters(
                            &http_client,
                            champ_id,
                            q_id,
                            assigned_pos.as_deref(),
                            None,
                            None,
                            None,
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

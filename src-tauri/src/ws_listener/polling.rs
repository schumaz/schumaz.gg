use irelia::ws::LcuWebSocket;
use irelia::ws::types::{Event, EventKind};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;
use tauri_specta::Event as SpectaEvent;

use super::coordinator::ChampSelectCoordinator;
use super::extract::{WsSessionTransition, evaluate_ws_event};
use super::gameflow::{process_gameflow_phase_update, sync_gameflow_phase_before_champ_select_end};
use super::reconcile_state::{shared_reconcile_lock, shared_session_state};
use super::session_handler::{handle_session_transition, process_session_timer};
use crate::models;

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

/// Starts the WebSocket listener and fallback polling task to observe Champion Select events in real time.
pub async fn start_ws_listener(app_handle: tauri::AppHandle) {
    let active_queue_id = Arc::new(AtomicI32::new(0));
    let session_state = shared_session_state();
    let reconcile_lock = shared_reconcile_lock();
    let coordinator = ChampSelectCoordinator::default();
    let last_gameflow_phase = Arc::new(tokio::sync::Mutex::new(String::new()));

    // Fallback sync loop: ensures Champion Select, FINALIZATION timer countdowns, and gameflow
    // phase transitions are observed even if WebSocket misses an event or reconnects mid-match.
    let fallback_handle = app_handle.clone();
    let fallback_q_ref = Arc::clone(&active_queue_id);
    let fallback_state_ref = Arc::clone(&session_state);
    let fallback_reconcile_lock = Arc::clone(&reconcile_lock);
    let fallback_coord = coordinator.clone();
    let fallback_phase_state = Arc::clone(&last_gameflow_phase);

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(1500)).await;

            let Ok(conn) = crate::lcu_connector::LcuConnection::connect() else {
                continue;
            };

            if let Ok(phase) = conn.get_gameflow_phase().await {
                process_gameflow_phase_update(&fallback_handle, &fallback_phase_state, phase).await;
            }

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
                        &fallback_state_ref,
                        &fallback_q_ref,
                        &fallback_coord,
                        &fallback_phase_state,
                    )
                    .await;

                    process_session_timer(
                        &fallback_handle,
                        &session_json,
                        &fallback_state_ref,
                        &fallback_reconcile_lock,
                        &fallback_q_ref,
                        &fallback_coord,
                    )
                    .await;
                }
                Ok(None) => {
                    fallback_coord.cancel_pending().await;
                    fallback_coord.cancel_finalization_timer().await;
                    let had_session = {
                        let mut state_guard = fallback_state_ref.lock().await;
                        let was_active = state_guard.last_payload.is_some();
                        state_guard.reset_session();
                        was_active
                    };
                    fallback_q_ref.store(0, Ordering::Relaxed);
                    if had_session {
                        sync_gameflow_phase_before_champ_select_end(
                            &fallback_handle,
                            &fallback_phase_state,
                        )
                        .await;
                        let _ = models::ChampSelectEnded.emit(&fallback_handle);
                    }
                }
                Err(_) => {}
            }
        }
    });

    // Real-time WebSocket listener with auto-reconnecting error handler
    let mut ws = LcuWebSocket::new_with_error_handler(ReconnectingErrorHandler);
    let ws_handle = app_handle.clone();
    let ws_q_ref = Arc::clone(&active_queue_id);
    let ws_state_ref = Arc::clone(&session_state);
    let ws_reconcile_lock = Arc::clone(&reconcile_lock);
    let ws_coord = coordinator.clone();
    let ws_phase_state = Arc::clone(&last_gameflow_phase);

    ws.subscribe(
        EventKind::json_api_event_callback_str("/lol-champ-select/v1/session"),
        move |event: &Event| {
            let handle = ws_handle.clone();
            let q_ref = Arc::clone(&ws_q_ref);
            let state_ref = Arc::clone(&ws_state_ref);
            let r_lock = Arc::clone(&ws_reconcile_lock);
            let coord = ws_coord.clone();
            let phase_state = Arc::clone(&ws_phase_state);

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
                let is_active = matches!(transition, WsSessionTransition::Active(_));
                handle_session_transition(
                    &handle,
                    transition,
                    &state_ref,
                    &q_ref,
                    &coord,
                    &phase_state,
                )
                .await;

                if is_active {
                    process_session_timer(&handle, &data, &state_ref, &r_lock, &q_ref, &coord)
                        .await;
                }
            });
        },
    );

    // Subscribe to gameflow phase changes to automatically return to Profile and refresh stats after a match
    let phase_handle = app_handle.clone();
    let phase_state = Arc::clone(&last_gameflow_phase);
    ws.subscribe(
        EventKind::json_api_event_callback_str("/lol-gameflow/v1/gameflow-phase"),
        move |event: &Event| {
            let handle = phase_handle.clone();
            let state = Arc::clone(&phase_state);
            let next_phase = event.2.data.as_str().unwrap_or("").to_string();

            if next_phase.is_empty() {
                return;
            }

            tokio::spawn(async move {
                process_gameflow_phase_update(&handle, &state, next_phase).await;
            });
        },
    );

    // Subscribe to mission and objective updates so delayed client hydration (~30s after login)
    // and newly granted/updated missions automatically clear notification badges and dots.
    for endpoint in [
        crate::lcu_connector::OBJECTIVES_READY_ENDPOINT,
        crate::lcu_connector::LOL_OBJECTIVES_ENDPOINT,
        crate::lcu_connector::TFT_OBJECTIVES_ENDPOINT,
        crate::lcu_connector::JADE_OBJECTIVES_ENDPOINT,
        crate::lcu_connector::LOL_MISSIONS_ENDPOINT,
        crate::lcu_connector::LOL_MISSIONS_SERIES_ENDPOINT,
    ] {
        ws.subscribe(
            EventKind::json_api_event_callback_str(endpoint),
            move |_event: &Event| {
                crate::lcu_connector::schedule_debounced_auto_mark_missions();
            },
        );
    }

    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

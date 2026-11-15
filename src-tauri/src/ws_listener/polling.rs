use irelia::ws::LcuWebSocket;
use irelia::ws::types::{Event, EventKind};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;
use tauri_specta::Event as SpectaEvent;

use super::coordinator::ChampSelectCoordinator;
use super::extract::{WsSessionTransition, evaluate_ws_event};
use super::gameflow::{process_gameflow_phase_update, sync_gameflow_phase_before_champ_select_end};
use super::reconcile_state::{
    ChampSelectSessionState, shared_reconcile_lock, shared_session_state,
};
use super::session_handler::{handle_session_transition, process_session_timer};
use crate::models;

const IDLE_FALLBACK_POLL_MS: u64 = 4000;
const CHAMP_SELECT_FALLBACK_POLL_MS: u64 = 1500;

/// Adaptive fallback polling interval: 1.5s during Champ Select / active session, 4s otherwise.
pub fn resolve_fallback_poll_interval(phase: &str, has_active_session: bool) -> Duration {
    if phase == "ChampSelect" || has_active_session {
        Duration::from_millis(CHAMP_SELECT_FALLBACK_POLL_MS)
    } else {
        Duration::from_millis(IDLE_FALLBACK_POLL_MS)
    }
}

/// Only query `/lol-champ-select/v1/session` when in Champ Select or cleaning up an active session.
pub fn should_poll_champ_select_session(phase: &str, has_active_session: bool) -> bool {
    phase == "ChampSelect" || has_active_session
}

/// Custom error handler for irelia WebSocket that stops immediately when LCU is disconnected
/// so irelia never runs full-process scans in the background while the League Client is closed.
struct MonitorAwareErrorHandler;

impl irelia::ws::ErrorHandler for MonitorAwareErrorHandler {
    fn on_error(
        &mut self,
        _error: irelia::ws::WebSocketError,
    ) -> std::ops::ControlFlow<(), irelia::ws::Flow> {
        if !crate::lcu_connector::is_lcu_connected() {
            return std::ops::ControlFlow::Break(());
        }
        std::thread::sleep(Duration::from_millis(1500));
        if !crate::lcu_connector::is_lcu_connected() {
            return std::ops::ControlFlow::Break(());
        }
        std::ops::ControlFlow::Continue(irelia::ws::Flow::TryReconnect)
    }
}

#[derive(Clone)]
struct WsListenerCtx {
    app_handle: tauri::AppHandle,
    active_queue_id: Arc<AtomicI32>,
    session_state: Arc<tokio::sync::Mutex<ChampSelectSessionState>>,
    reconcile_lock: Arc<tokio::sync::Mutex<()>>,
    coordinator: ChampSelectCoordinator,
    last_gameflow_phase: Arc<tokio::sync::Mutex<String>>,
}

async fn clear_active_session(ctx: &WsListenerCtx, emit_ended: bool) {
    ctx.coordinator.cancel_pending().await;
    ctx.coordinator.cancel_finalization_timer().await;
    let had_session = {
        let mut state_guard = ctx.session_state.lock().await;
        let was_active = state_guard.last_payload.is_some();
        state_guard.reset_session();
        was_active
    };
    ctx.active_queue_id.store(0, Ordering::Relaxed);
    if emit_ended && had_session {
        sync_gameflow_phase_before_champ_select_end(&ctx.app_handle, &ctx.last_gameflow_phase)
            .await;
        let _ = models::ChampSelectEnded.emit(&ctx.app_handle);
    }
}

fn spawn_configured_ws(ctx: &WsListenerCtx) -> LcuWebSocket {
    let mut ws = LcuWebSocket::new_with_error_handler(MonitorAwareErrorHandler);

    let ws_ctx = ctx.clone();
    ws.subscribe(
        EventKind::json_api_event_callback_str("/lol-champ-select/v1/session"),
        move |event: &Event| {
            let ctx = ws_ctx.clone();
            let event_type = event.2.event_type.clone();
            let data = event.2.data.clone();

            tokio::spawn(async move {
                let mut current_q = ctx.active_queue_id.load(Ordering::Relaxed);
                if current_q <= 0 {
                    let maybe_fetched = match crate::lcu_connector::LcuConnection::connect() {
                        Ok(conn) => conn.get_current_queue_id().await.ok(),
                        Err(_) => None,
                    };
                    if let Some(fetched_q) = maybe_fetched {
                        ctx.active_queue_id.store(fetched_q, Ordering::Relaxed);
                        current_q = fetched_q;
                    }
                }

                let transition = evaluate_ws_event(&event_type, &data, current_q);
                let is_active = matches!(transition, WsSessionTransition::Active(_));
                handle_session_transition(
                    &ctx.app_handle,
                    transition,
                    &ctx.session_state,
                    &ctx.active_queue_id,
                    &ctx.coordinator,
                    &ctx.last_gameflow_phase,
                )
                .await;

                if is_active {
                    process_session_timer(
                        &ctx.app_handle,
                        &data,
                        &ctx.session_state,
                        &ctx.reconcile_lock,
                        &ctx.active_queue_id,
                        &ctx.coordinator,
                    )
                    .await;
                }
            });
        },
    );

    let phase_handle = ctx.app_handle.clone();
    let phase_state = Arc::clone(&ctx.last_gameflow_phase);
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

    ws
}

async fn run_fallback_poll_loop(ctx: WsListenerCtx) {
    let mut conn_rx = crate::lcu_connector::subscribe_lcu_connection();

    loop {
        if !*conn_rx.borrow() {
            clear_active_session(&ctx, false).await;
            {
                let mut phase_guard = ctx.last_gameflow_phase.lock().await;
                phase_guard.clear();
            }
            if conn_rx.changed().await.is_err() {
                break;
            }
            continue;
        }

        let current_phase = ctx.last_gameflow_phase.lock().await.clone();
        let has_active_session = ctx.session_state.lock().await.last_payload.is_some();
        let poll_interval = resolve_fallback_poll_interval(&current_phase, has_active_session);

        tokio::select! {
            _ = tokio::time::sleep(poll_interval) => {}
            changed = conn_rx.changed() => {
                if changed.is_err() {
                    break;
                }
                continue;
            }
        }

        if !*conn_rx.borrow() {
            continue;
        }

        let Ok(conn) = crate::lcu_connector::LcuConnection::connect() else {
            continue;
        };

        let mut effective_phase = current_phase;
        if let Ok(phase) = conn.get_gameflow_phase().await {
            effective_phase = phase.clone();
            process_gameflow_phase_update(&ctx.app_handle, &ctx.last_gameflow_phase, phase).await;
        }

        if !should_poll_champ_select_session(&effective_phase, has_active_session) {
            continue;
        }

        match conn.get_champ_select_session().await {
            Ok(Some(session_json)) => {
                let mut current_q = ctx.active_queue_id.load(Ordering::Relaxed);
                if current_q <= 0
                    && let Ok(fetched_q) = conn.get_current_queue_id().await
                {
                    ctx.active_queue_id.store(fetched_q, Ordering::Relaxed);
                    current_q = fetched_q;
                }

                let transition = evaluate_ws_event("Update", &session_json, current_q);
                handle_session_transition(
                    &ctx.app_handle,
                    transition,
                    &ctx.session_state,
                    &ctx.active_queue_id,
                    &ctx.coordinator,
                    &ctx.last_gameflow_phase,
                )
                .await;

                process_session_timer(
                    &ctx.app_handle,
                    &session_json,
                    &ctx.session_state,
                    &ctx.reconcile_lock,
                    &ctx.active_queue_id,
                    &ctx.coordinator,
                )
                .await;
            }
            Ok(None) => {
                clear_active_session(&ctx, true).await;
            }
            Err(_) => {}
        }
    }
}

/// Starts the WebSocket listener and adaptive fallback polling task coordinated with `lcu_monitor`.
pub async fn start_ws_listener(app_handle: tauri::AppHandle) {
    let ctx = WsListenerCtx {
        app_handle,
        active_queue_id: Arc::new(AtomicI32::new(0)),
        session_state: shared_session_state(),
        reconcile_lock: shared_reconcile_lock(),
        coordinator: ChampSelectCoordinator::default(),
        last_gameflow_phase: Arc::new(tokio::sync::Mutex::new(String::new())),
    };

    tokio::spawn(run_fallback_poll_loop(ctx.clone()));

    let mut conn_rx = crate::lcu_connector::subscribe_lcu_connection();
    let mut active_ws: Option<LcuWebSocket> = None;

    loop {
        if !*conn_rx.borrow() {
            if let Some(ws) = active_ws.take() {
                let _ = ws.abort();
            }
            if conn_rx.changed().await.is_err() {
                break;
            }
            continue;
        }

        if active_ws.as_ref().is_none_or(LcuWebSocket::is_finished) {
            active_ws = Some(spawn_configured_ws(&ctx));
        }

        tokio::select! {
            changed = conn_rx.changed() => {
                if changed.is_err() {
                    if let Some(ws) = active_ws.take() {
                        let _ = ws.abort();
                    }
                    break;
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(5)) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_fallback_poll_interval_is_adaptive() {
        assert_eq!(
            resolve_fallback_poll_interval("ChampSelect", false),
            Duration::from_millis(1500)
        );
        assert_eq!(
            resolve_fallback_poll_interval("Lobby", true),
            Duration::from_millis(1500)
        );
        assert_eq!(
            resolve_fallback_poll_interval("Lobby", false),
            Duration::from_millis(4000)
        );
        assert_eq!(
            resolve_fallback_poll_interval("InProgress", false),
            Duration::from_millis(4000)
        );
        assert_eq!(
            resolve_fallback_poll_interval("", false),
            Duration::from_millis(4000)
        );
    }

    #[test]
    fn test_should_poll_champ_select_session_only_when_needed() {
        assert!(should_poll_champ_select_session("ChampSelect", false));
        assert!(should_poll_champ_select_session("Lobby", true));
        assert!(!should_poll_champ_select_session("Lobby", false));
        assert!(!should_poll_champ_select_session("InProgress", false));
        assert!(!should_poll_champ_select_session("None", false));
        assert!(!should_poll_champ_select_session("", false));
    }
}

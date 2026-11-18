use irelia::ws::LcuWebSocket;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;

use super::coordinator::ChampSelectCoordinator;
use super::extract::evaluate_ws_event;
use super::gameflow::process_gameflow_phase_update;
use super::reconcile_state::{shared_reconcile_lock, shared_session_state};
use super::session_handler::{handle_session_transition, process_session_timer};
use super::subscription::{WsListenerCtx, clear_active_session, spawn_configured_ws};

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

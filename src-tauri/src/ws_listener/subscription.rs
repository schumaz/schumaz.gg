use irelia::ws::LcuWebSocket;
use irelia::ws::types::{Event, EventKind};
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;
use tauri_specta::Event as SpectaEvent;

use super::coordinator::ChampSelectCoordinator;
use super::extract::{WsSessionTransition, evaluate_ws_event};
use super::gameflow::{process_gameflow_phase_update, sync_gameflow_phase_before_champ_select_end};
use super::reconcile_state::{
    ChampSelectSessionState, shared_active_queue_id, shared_coordinator,
    shared_last_gameflow_phase, shared_poll_lock, shared_reconcile_lock, shared_session_state,
};
use super::session_handler::{
    handle_session_transition, process_session_timer, should_defer_initial_inject_to_finalization,
};
use crate::{logging, models};

/// Spawns an async task from synchronous WebSocket callback threads (e.g. `irelia` WS thread)
/// onto the captured Tokio runtime, falling back to `tauri::async_runtime` if needed.
fn spawn_on_runtime<F>(rt_handle: Option<&tokio::runtime::Handle>, future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    if let Some(handle) = rt_handle {
        handle.spawn(future);
    } else if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(future);
    } else {
        tauri::async_runtime::spawn(future);
    }
}

/// Custom error handler for irelia WebSocket that stops immediately when LCU is disconnected
/// so irelia never runs full-process scans in the background while the League Client is closed.
struct MonitorAwareErrorHandler;

impl irelia::ws::ErrorHandler for MonitorAwareErrorHandler {
    fn on_error(
        &mut self,
        error: irelia::ws::WebSocketError,
    ) -> std::ops::ControlFlow<(), irelia::ws::Flow> {
        if !crate::lcu_connector::is_lcu_connected() {
            logging::log_info("ws.listener", "event=ws_stopped reason=lcu_disconnected");
            return std::ops::ControlFlow::Break(());
        }
        logging::log_warn(
            "ws.listener",
            format!("event=ws_reconnect_scheduled err={error:?}"),
        );
        std::thread::sleep(Duration::from_millis(1500));
        if !crate::lcu_connector::is_lcu_connected() {
            logging::log_info("ws.listener", "event=ws_stopped reason=lcu_disconnected");
            return std::ops::ControlFlow::Break(());
        }
        std::ops::ControlFlow::Continue(irelia::ws::Flow::TryReconnect)
    }
}

#[derive(Clone)]
pub(crate) struct WsListenerCtx {
    pub(crate) app_handle: tauri::AppHandle,
    pub(crate) active_queue_id: Arc<AtomicI32>,
    pub(crate) session_state: Arc<tokio::sync::Mutex<ChampSelectSessionState>>,
    pub(crate) reconcile_lock: Arc<tokio::sync::Mutex<()>>,
    pub(crate) coordinator: ChampSelectCoordinator,
    pub(crate) last_gameflow_phase: Arc<tokio::sync::Mutex<String>>,
    pub(crate) poll_lock: Arc<tokio::sync::Mutex<()>>,
}

impl WsListenerCtx {
    /// Builds a listener context backed by process-wide shared state so WS callbacks,
    /// fallback polling, and on-demand IPC bootstrap snapshots stay synchronized.
    pub(crate) fn for_app(app_handle: tauri::AppHandle) -> Self {
        Self {
            app_handle,
            active_queue_id: shared_active_queue_id(),
            session_state: shared_session_state(),
            reconcile_lock: shared_reconcile_lock(),
            coordinator: shared_coordinator(),
            last_gameflow_phase: shared_last_gameflow_phase(),
            poll_lock: shared_poll_lock(),
        }
    }
}

pub(super) async fn clear_active_session(ctx: &WsListenerCtx, emit_ended: bool) {
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
        logging::log_info("ws.champ_select", "event=champ_select_ended");
        sync_gameflow_phase_before_champ_select_end(&ctx.app_handle, &ctx.last_gameflow_phase)
            .await;
        let _ = models::ChampSelectEnded.emit(&ctx.app_handle);
    }
}

pub(super) fn spawn_configured_ws(ctx: &WsListenerCtx) -> LcuWebSocket {
    let rt_handle = tokio::runtime::Handle::try_current().ok();
    let mut ws = LcuWebSocket::new_with_error_handler(MonitorAwareErrorHandler);
    logging::log_info("ws.listener", "event=ws_subscribed");

    let ws_ctx = ctx.clone();
    let session_rt = rt_handle.clone();
    ws.subscribe(
        EventKind::json_api_event_callback_str("/lol-champ-select/v1/session"),
        move |event: &Event| {
            let ctx = ws_ctx.clone();
            let event_type = event.2.event_type.clone();
            let data = event.2.data.clone();

            spawn_on_runtime(session_rt.as_ref(), async move {
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
                if is_active && ctx.last_gameflow_phase.lock().await.is_empty() {
                    process_gameflow_phase_update(
                        &ctx.app_handle,
                        &ctx.last_gameflow_phase,
                        "ChampSelect".to_string(),
                    )
                    .await;
                }

                let defer_initial_inject = matches!(
                    &transition,
                    WsSessionTransition::Active(payload)
                        if should_defer_initial_inject_to_finalization(payload, &data)
                );

                handle_session_transition(
                    &ctx.app_handle,
                    transition,
                    &ctx.session_state,
                    &ctx.active_queue_id,
                    &ctx.coordinator,
                    &ctx.last_gameflow_phase,
                    defer_initial_inject,
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
    let phase_rt = rt_handle.clone();
    ws.subscribe(
        EventKind::json_api_event_callback_str("/lol-gameflow/v1/gameflow-phase"),
        move |event: &Event| {
            let handle = phase_handle.clone();
            let state = Arc::clone(&phase_state);
            let next_phase = event.2.data.as_str().unwrap_or("").to_string();

            if next_phase.is_empty() {
                return;
            }

            spawn_on_runtime(phase_rt.as_ref(), async move {
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
        let missions_rt = rt_handle.clone();
        ws.subscribe(
            EventKind::json_api_event_callback_str(endpoint),
            move |_event: &Event| {
                let _guard = missions_rt.as_ref().map(tokio::runtime::Handle::enter);
                crate::lcu_connector::schedule_debounced_auto_mark_missions();
            },
        );
    }

    ws
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn_on_runtime_succeeds_from_non_tokio_thread() {
        let rt_handle = tokio::runtime::Handle::current();
        let (tx, rx) = tokio::sync::oneshot::channel();

        std::thread::spawn(move || {
            spawn_on_runtime(Some(&rt_handle), async move {
                let _ = tx.send(42);
            });
        })
        .join()
        .expect("worker thread should not panic without active Tokio TLS context");

        assert_eq!(rx.await.expect("spawned task should complete"), 42);
    }
}

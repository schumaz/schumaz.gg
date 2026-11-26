use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;
use tauri_specta::Event as SpectaEvent;
use tokio::sync::mpsc::UnboundedSender;

use super::actor::{LcuEventActorMsg, register_shared_actor_sender, run_lcu_event_actor};
use super::coordinator::ChampSelectCoordinator;
use super::gameflow::sync_gameflow_phase_before_champ_select_end;
use super::polling::resolve_next_poll_delay;
use super::reconcile_state::{
    ChampSelectSessionState, shared_active_queue_id, shared_coordinator,
    shared_last_gameflow_phase, shared_poll_lock, shared_reconcile_lock, shared_session_state,
};
use super::ws_client::{LcuWsEvent, LcuWsHandle, WsEventCallback, endpoint_to_json_api_topic};
use crate::{logging, models};

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
    crate::lcu_injector::clear_champion_selections();
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

pub(super) fn spawn_configured_ws(actor_tx: &UnboundedSender<LcuEventActorMsg>) -> LcuWsHandle {
    let rt_handle = tokio::runtime::Handle::try_current().ok();
    let mut subscriptions: Vec<(String, WsEventCallback)> = Vec::with_capacity(9);

    let champ_tx = actor_tx.clone();
    subscriptions.push((
        endpoint_to_json_api_topic(crate::lcu_connector::CHAMP_SELECT_SESSION_ENDPOINT),
        Arc::new(move |event: &LcuWsEvent| {
            let _ = champ_tx.send(LcuEventActorMsg::WsChampSelect {
                event_type: event.event_type.clone(),
                data: event.data.clone(),
            });
        }),
    ));

    let phase_tx = actor_tx.clone();
    subscriptions.push((
        endpoint_to_json_api_topic(crate::lcu_connector::GAMEFLOW_PHASE_ENDPOINT),
        Arc::new(move |event: &LcuWsEvent| {
            let next_phase = event.data.as_str().unwrap_or("").trim().to_string();
            if !next_phase.is_empty() {
                let _ = phase_tx.send(LcuEventActorMsg::WsGameflowPhase { phase: next_phase });
            }
        }),
    ));

    let lobby_tx = actor_tx.clone();
    subscriptions.push((
        endpoint_to_json_api_topic(crate::lcu_connector::LOBBY_ENDPOINT),
        Arc::new(move |event: &LcuWsEvent| {
            let _ = lobby_tx.send(LcuEventActorMsg::WsLobby {
                event_type: event.event_type.clone(),
                data: event.data.clone(),
            });
        }),
    ));

    for endpoint in [
        crate::lcu_connector::OBJECTIVES_READY_ENDPOINT,
        crate::lcu_connector::LOL_OBJECTIVES_ENDPOINT,
        crate::lcu_connector::TFT_OBJECTIVES_ENDPOINT,
        crate::lcu_connector::JADE_OBJECTIVES_ENDPOINT,
        crate::lcu_connector::LOL_MISSIONS_ENDPOINT,
        crate::lcu_connector::LOL_MISSIONS_SERIES_ENDPOINT,
    ] {
        let missions_rt = rt_handle.clone();
        subscriptions.push((
            endpoint_to_json_api_topic(endpoint),
            Arc::new(move |_event: &LcuWsEvent| {
                let _guard = missions_rt.as_ref().map(tokio::runtime::Handle::enter);
                crate::lcu_connector::schedule_debounced_auto_mark_missions();
            }),
        ));
    }

    let ws = LcuWsHandle::spawn(subscriptions);
    logging::log_info("ws.listener", "event=ws_subscribed");
    ws
}

async fn run_fallback_poll_loop(
    ctx: WsListenerCtx,
    actor_tx: UnboundedSender<LcuEventActorMsg>,
) {
    let mut conn_rx = crate::lcu_connector::subscribe_lcu_connection();
    let mut needs_immediate_poll = true;

    loop {
        if !*conn_rx.borrow() {
            needs_immediate_poll = true;
            let (done_tx, done_rx) = tokio::sync::oneshot::channel();
            if actor_tx
                .send(LcuEventActorMsg::LcuDisconnected { done: done_tx })
                .is_ok()
            {
                let _ = done_rx.await;
            } else {
                clear_active_session(&ctx, false).await;
                ctx.last_gameflow_phase.lock().await.clear();
            }
            if conn_rx.changed().await.is_err() {
                break;
            }
            continue;
        }

        let current_phase = ctx.last_gameflow_phase.lock().await.clone();
        let has_active_session = ctx.session_state.lock().await.last_payload.is_some();

        if let Some(poll_interval) =
            resolve_next_poll_delay(needs_immediate_poll, &current_phase, has_active_session)
        {
            tokio::select! {
                _ = tokio::time::sleep(poll_interval) => {}
                changed = conn_rx.changed() => {
                    if changed.is_err() { break; }
                    if *conn_rx.borrow() { needs_immediate_poll = true; }
                    continue;
                }
            }
        } else {
            needs_immediate_poll = false;
        }

        if !*conn_rx.borrow() {
            continue;
        }

        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
        if actor_tx
            .send(LcuEventActorMsg::FallbackPoll { done: done_tx })
            .is_ok()
        {
            let _ = done_rx.await;
        }
    }
}

/// Starts the WebSocket listener, sequential FIFO event actor, and adaptive fallback polling task.
pub async fn start_ws_listener(app_handle: tauri::AppHandle) {
    let ctx = WsListenerCtx::for_app(app_handle);
    let (actor_tx, actor_rx) = tokio::sync::mpsc::unbounded_channel();
    register_shared_actor_sender(actor_tx.clone());

    tokio::spawn(run_lcu_event_actor(ctx.clone(), actor_rx));
    tokio::spawn(run_fallback_poll_loop(ctx.clone(), actor_tx.clone()));

    let mut conn_rx = crate::lcu_connector::subscribe_lcu_connection();
    let mut active_ws: Option<LcuWsHandle> = None;

    loop {
        if !*conn_rx.borrow() {
            if let Some(ws) = active_ws.take() {
                ws.abort();
            }
            if conn_rx.changed().await.is_err() {
                break;
            }
            continue;
        }

        if active_ws.as_ref().is_none_or(LcuWsHandle::is_finished) {
            active_ws = Some(spawn_configured_ws(&actor_tx));
        }

        tokio::select! {
            changed = conn_rx.changed() => {
                if changed.is_err() {
                    if let Some(ws) = active_ws.take() {
                        ws.abort();
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

    #[tokio::test]
    async fn test_actor_enqueue_from_non_tokio_thread_preserves_fifo_order() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        std::thread::spawn(move || {
            tx.send(LcuEventActorMsg::WsChampSelect {
                event_type: "Update".to_string(),
                data: serde_json::json!({ "championId": 103 }),
            })
            .unwrap();
            tx.send(LcuEventActorMsg::WsChampSelect {
                event_type: "Delete".to_string(),
                data: serde_json::Value::Null,
            })
            .unwrap();
        })
        .join()
        .expect("worker thread should enqueue without active Tokio TLS context");

        match rx.recv().await {
            Some(LcuEventActorMsg::WsChampSelect { event_type, .. }) => {
                assert_eq!(event_type, "Update");
            }
            _ => panic!("expected first FIFO event to be Update"),
        }
        match rx.recv().await {
            Some(LcuEventActorMsg::WsChampSelect { event_type, .. }) => {
                assert_eq!(event_type, "Delete");
            }
            _ => panic!("expected second FIFO event to be Delete"),
        }
    }
}

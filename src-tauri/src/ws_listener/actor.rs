use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;

use super::extract::{WsSessionTransition, evaluate_ws_event};
use super::gameflow::process_gameflow_phase_update;
use super::lobby_handler::handle_lobby_ws_payload;
use super::polling::poll_lcu_session_once_locked;
use super::reconcile_state::{ChampSelectSessionState, should_probe_lcu_for_snapshot};
use super::session_handler::{
    handle_session_transition, process_session_timer, should_defer_initial_inject_to_finalization,
};
use super::subscription::{WsListenerCtx, clear_active_session};

pub(crate) enum LcuEventActorMsg {
    WsChampSelect {
        event_type: String,
        data: serde_json::Value,
    },
    WsGameflowPhase {
        phase: String,
    },
    WsLobby {
        event_type: String,
        data: serde_json::Value,
    },
    FallbackPoll {
        done: oneshot::Sender<()>,
    },
    ProbeSnapshotIfNeeded {
        done: oneshot::Sender<()>,
    },
    LcuDisconnected {
        done: oneshot::Sender<()>,
    },
}

static SHARED_ACTOR_TX: OnceLock<RwLock<Option<UnboundedSender<LcuEventActorMsg>>>> =
    OnceLock::new();

fn actor_sender_slot() -> &'static RwLock<Option<UnboundedSender<LcuEventActorMsg>>> {
    SHARED_ACTOR_TX.get_or_init(|| RwLock::new(None))
}

pub(crate) fn register_shared_actor_sender(tx: UnboundedSender<LcuEventActorMsg>) {
    if let Ok(mut guard) = actor_sender_slot().write() {
        *guard = Some(tx);
    }
}

pub(crate) async fn try_probe_snapshot_via_actor() -> bool {
    let tx_opt = actor_sender_slot()
        .read()
        .ok()
        .and_then(|guard| guard.clone());
    let Some(tx) = tx_opt else {
        return false;
    };
    let (done_tx, done_rx) = oneshot::channel();
    if tx
        .send(LcuEventActorMsg::ProbeSnapshotIfNeeded { done: done_tx })
        .is_err()
    {
        return false;
    }
    done_rx.await.is_ok()
}

pub(crate) trait WsActorDriver: Send + Sync {
    fn fetch_current_queue_id(&self) -> impl std::future::Future<Output = Option<i32>> + Send;

    fn on_champ_select_event(
        &self,
        event_type: &str,
        data: &serde_json::Value,
        current_q: i32,
    ) -> impl std::future::Future<Output = ()> + Send;

    fn on_gameflow_phase(&self, phase: String) -> impl std::future::Future<Output = ()> + Send;

    fn on_lobby_event(
        &self,
        event_type: &str,
        data: &serde_json::Value,
    ) -> impl std::future::Future<Output = ()> + Send;

    fn on_fallback_poll(&self) -> impl std::future::Future<Output = ()> + Send;

    fn on_lcu_disconnected(&self) -> impl std::future::Future<Output = ()> + Send;
}

struct LiveWsActorDriver<'a> {
    ctx: &'a WsListenerCtx,
}

impl WsActorDriver for LiveWsActorDriver<'_> {
    async fn fetch_current_queue_id(&self) -> Option<i32> {
        let conn = crate::lcu_connector::LcuConnection::connect().ok()?;
        conn.get_current_queue_id().await.ok()
    }

    async fn on_champ_select_event(
        &self,
        event_type: &str,
        data: &serde_json::Value,
        current_q: i32,
    ) {
        let transition = evaluate_ws_event(event_type, data, current_q);
        let is_active = matches!(transition, WsSessionTransition::Active(_));
        if is_active && self.ctx.last_gameflow_phase.lock().await.is_empty() {
            process_gameflow_phase_update(
                &self.ctx.app_handle,
                &self.ctx.last_gameflow_phase,
                "ChampSelect".to_string(),
            )
            .await;
        }

        let defer_initial_inject = matches!(
            &transition,
            WsSessionTransition::Active(payload)
                if should_defer_initial_inject_to_finalization(payload, data)
        );

        handle_session_transition(
            &self.ctx.app_handle,
            transition,
            &self.ctx.session_state,
            &self.ctx.active_queue_id,
            &self.ctx.coordinator,
            &self.ctx.last_gameflow_phase,
            defer_initial_inject,
        )
        .await;

        if is_active {
            process_session_timer(
                &self.ctx.app_handle,
                data,
                &self.ctx.session_state,
                &self.ctx.reconcile_lock,
                &self.ctx.active_queue_id,
                &self.ctx.coordinator,
            )
            .await;
        }
    }

    async fn on_gameflow_phase(&self, phase: String) {
        process_gameflow_phase_update(&self.ctx.app_handle, &self.ctx.last_gameflow_phase, phase)
            .await;
    }

    async fn on_lobby_event(&self, event_type: &str, data: &serde_json::Value) {
        handle_lobby_ws_payload(self.ctx, event_type, data).await;
    }

    async fn on_fallback_poll(&self) {
        poll_lcu_session_once_locked(self.ctx).await;
    }

    async fn on_lcu_disconnected(&self) {
        clear_active_session(self.ctx, false).await;
        self.ctx.last_gameflow_phase.lock().await.clear();
    }
}

pub(crate) async fn process_actor_message<D: WsActorDriver>(
    driver: &D,
    active_queue_id: &Arc<AtomicI32>,
    last_gameflow_phase: &Arc<tokio::sync::Mutex<String>>,
    session_state: &Arc<tokio::sync::Mutex<ChampSelectSessionState>>,
    poll_lock: &Arc<tokio::sync::Mutex<()>>,
    msg: LcuEventActorMsg,
) {
    let _poll_guard = poll_lock.lock().await;
    match msg {
        LcuEventActorMsg::WsChampSelect { event_type, data } => {
            let mut current_q = active_queue_id.load(Ordering::Relaxed);
            if event_type != "Delete"
                && current_q <= 0
                && let Some(fetched_q) = driver.fetch_current_queue_id().await
            {
                active_queue_id.store(fetched_q, Ordering::Relaxed);
                current_q = fetched_q;
            }
            driver
                .on_champ_select_event(&event_type, &data, current_q)
                .await;
        }
        LcuEventActorMsg::WsGameflowPhase { phase } => {
            driver.on_gameflow_phase(phase).await;
        }
        LcuEventActorMsg::WsLobby { event_type, data } => {
            driver.on_lobby_event(&event_type, &data).await;
        }
        LcuEventActorMsg::FallbackPoll { done } => {
            driver.on_fallback_poll().await;
            let _ = done.send(());
        }
        LcuEventActorMsg::ProbeSnapshotIfNeeded { done } => {
            let needs_probe = {
                let phase = last_gameflow_phase.lock().await;
                let state = session_state.lock().await;
                should_probe_lcu_for_snapshot(&phase, &state)
            };
            if needs_probe {
                driver.on_fallback_poll().await;
            }
            let _ = done.send(());
        }
        LcuEventActorMsg::LcuDisconnected { done } => {
            driver.on_lcu_disconnected().await;
            let _ = done.send(());
        }
    }
}

pub(crate) async fn run_lcu_event_actor_with_driver<D: WsActorDriver>(
    driver: &D,
    active_queue_id: &Arc<AtomicI32>,
    last_gameflow_phase: &Arc<tokio::sync::Mutex<String>>,
    session_state: &Arc<tokio::sync::Mutex<ChampSelectSessionState>>,
    poll_lock: &Arc<tokio::sync::Mutex<()>>,
    mut rx: UnboundedReceiver<LcuEventActorMsg>,
) {
    while let Some(msg) = rx.recv().await {
        process_actor_message(
            driver,
            active_queue_id,
            last_gameflow_phase,
            session_state,
            poll_lock,
            msg,
        )
        .await;
    }
}

/// Runs the single sequential FIFO actor for all LCU WebSocket events, fallback polls, and probes.
pub(crate) async fn run_lcu_event_actor(
    ctx: WsListenerCtx,
    rx: UnboundedReceiver<LcuEventActorMsg>,
) {
    let driver = LiveWsActorDriver { ctx: &ctx };
    run_lcu_event_actor_with_driver(
        &driver,
        &ctx.active_queue_id,
        &ctx.last_gameflow_phase,
        &ctx.session_state,
        &ctx.poll_lock,
        rx,
    )
    .await;
}

#[cfg(test)]
#[path = "actor_tests.rs"]
mod tests;

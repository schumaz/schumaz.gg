use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;

use super::extract::{WsSessionTransition, evaluate_ws_event};
use super::gameflow::process_gameflow_phase_update;
use super::reconcile_state::{
    ChampSelectSessionState, build_active_champ_select_snapshot, should_probe_lcu_for_snapshot,
};
use super::session_handler::{
    handle_session_transition, process_session_timer, should_defer_initial_inject_to_finalization,
};
use super::subscription::{WsListenerCtx, clear_active_session, spawn_configured_ws};
use super::ws_client::LcuWsHandle;
use crate::models::ActiveChampSelectSnapshot;

const IDLE_FALLBACK_POLL_MS: u64 = 4000;
const CHAMP_SELECT_FALLBACK_POLL_MS: u64 = 1500;

/// Adaptive fallback polling interval: 1.5s during Champ Select / active session, 4s otherwise.
pub fn resolve_fallback_poll_interval(phase: &str, has_active_session: bool) -> Duration {
    let ms = if phase == "ChampSelect" || has_active_session {
        CHAMP_SELECT_FALLBACK_POLL_MS
    } else {
        IDLE_FALLBACK_POLL_MS
    };
    Duration::from_millis(ms)
}

/// Returns `None` when an immediate 0ms poll is required (e.g. right after LCU transitions to connected),
/// or `Some(Duration)` with the adaptive interval otherwise.
pub fn resolve_next_poll_delay(
    needs_immediate_poll: bool,
    phase: &str,
    has_active_session: bool,
) -> Option<Duration> {
    (!needs_immediate_poll).then(|| resolve_fallback_poll_interval(phase, has_active_session))
}

/// Only query `/lol-champ-select/v1/session` when in Champ Select or cleaning up an active session.
pub fn should_poll_champ_select_session(phase: &str, has_active_session: bool) -> bool {
    phase == "ChampSelect" || has_active_session
}

/// Query `/lol-lobby/v2/lobby` during lobby/matchmaking phases or when cleaning up an active lobby pre-select.
pub fn should_poll_lobby_session(phase: &str, has_active_lobby_session: bool) -> bool {
    matches!(phase, "Lobby" | "Matchmaking" | "ReadyCheck") || has_active_lobby_session
}

/// Gateway abstraction for a single LCU fallback/bootstrap poll cycle (mockable in tests).
pub(crate) trait LcuPollGateway: Send + Sync {
    fn get_gameflow_phase(&self) -> impl std::future::Future<Output = Result<String, String>> + Send;
    fn get_champ_select_session(&self) -> impl std::future::Future<Output = Result<Option<serde_json::Value>, String>> + Send;
    fn get_current_queue_id(&self) -> impl std::future::Future<Output = Result<i32, String>> + Send;
    fn get_lobby_session(&self) -> impl std::future::Future<Output = Result<Option<serde_json::Value>, String>> + Send;
}

/// Side-effect sink for a single LCU poll cycle so tests can assert state and scheduling decisions.
pub(crate) trait LcuPollSink: Send + Sync {
    fn on_gameflow_phase(&self, phase: String) -> impl std::future::Future<Output = ()> + Send;
    fn on_session_update(
        &self,
        transition: WsSessionTransition,
        session_json: &serde_json::Value,
        defer_initial_inject: bool,
    ) -> impl std::future::Future<Output = ()> + Send;
    fn on_session_ended(&self) -> impl std::future::Future<Output = ()> + Send;
    fn on_lobby_update(
        &self,
        event_type: &str,
        lobby_json: &serde_json::Value,
    ) -> impl std::future::Future<Output = ()> + Send;
}

struct LiveLcuPollGateway {
    conn: crate::lcu_connector::LcuConnection,
}

impl LcuPollGateway for LiveLcuPollGateway {
    async fn get_gameflow_phase(&self) -> Result<String, String> {
        self.conn.get_gameflow_phase().await
    }
    async fn get_champ_select_session(&self) -> Result<Option<serde_json::Value>, String> {
        self.conn.get_champ_select_session().await
    }
    async fn get_current_queue_id(&self) -> Result<i32, String> {
        self.conn.get_current_queue_id().await
    }
    async fn get_lobby_session(&self) -> Result<Option<serde_json::Value>, String> {
        self.conn.get_lobby_session().await
    }
}

struct LiveLcuPollSink<'a> {
    ctx: &'a WsListenerCtx,
}

impl LcuPollSink for LiveLcuPollSink<'_> {
    async fn on_gameflow_phase(&self, phase: String) {
        process_gameflow_phase_update(&self.ctx.app_handle, &self.ctx.last_gameflow_phase, phase)
            .await;
    }

    async fn on_session_update(
        &self,
        transition: WsSessionTransition,
        session_json: &serde_json::Value,
        defer_initial_inject: bool,
    ) {
        let is_active = matches!(transition, WsSessionTransition::Active(_));
        handle_session_transition(
            &self.ctx.app_handle, transition, &self.ctx.session_state, &self.ctx.active_queue_id,
            &self.ctx.coordinator, &self.ctx.last_gameflow_phase, defer_initial_inject,
        )
        .await;

        if is_active {
            process_session_timer(
                &self.ctx.app_handle, session_json, &self.ctx.session_state,
                &self.ctx.reconcile_lock, &self.ctx.active_queue_id, &self.ctx.coordinator,
            )
            .await;
        }
    }

    async fn on_session_ended(&self) {
        clear_active_session(self.ctx, true).await;
    }

    async fn on_lobby_update(&self, event_type: &str, lobby_json: &serde_json::Value) {
        super::lobby_handler::handle_lobby_ws_payload(self.ctx, event_type, lobby_json).await;
    }
}

/// Executes a single LCU gameflow + Champ Select / Lobby poll cycle using the supplied gateway and sink.
pub(crate) async fn poll_lcu_session_with_gateway<G, S>(
    gateway: &G,
    sink: &S,
    last_gameflow_phase: &Arc<tokio::sync::Mutex<String>>,
    session_state: &Arc<tokio::sync::Mutex<ChampSelectSessionState>>,
    active_queue_id: &Arc<AtomicI32>,
) where
    G: LcuPollGateway,
    S: LcuPollSink,
{
    let current_phase = last_gameflow_phase.lock().await.clone();
    let (has_active_champ_select, is_lobby_session) = {
        let guard = session_state.lock().await;
        match guard.last_payload.as_ref() {
            Some(p) if p.is_lobby_preselect => (false, true),
            Some(_) => (true, false),
            None => (false, false),
        }
    };

    let mut effective_phase = current_phase;
    if let Ok(phase) = gateway.get_gameflow_phase().await {
        effective_phase = phase.clone();
        sink.on_gameflow_phase(phase).await;
    }

    if !should_poll_champ_select_session(&effective_phase, has_active_champ_select) {
        let active_non_game_lobby =
            is_lobby_session && !super::gameflow::is_in_game_phase(&effective_phase);
        if should_poll_lobby_session(&effective_phase, active_non_game_lobby) {
            match gateway.get_lobby_session().await {
                Ok(Some(lobby_json)) => sink.on_lobby_update("Update", &lobby_json).await,
                Ok(None) if active_non_game_lobby => {
                    sink.on_lobby_update("Delete", &serde_json::Value::Null).await;
                }
                _ => {}
            }
        }
        return;
    }

    let existing_q = active_queue_id.load(Ordering::Relaxed);
    let (session_res, fetched_q_opt) = tokio::join!(gateway.get_champ_select_session(), async {
        if existing_q <= 0 {
            gateway.get_current_queue_id().await.ok()
        } else {
            None
        }
    });

    match session_res {
        Ok(Some(session_json)) => {
            let mut current_q = existing_q;
            if existing_q <= 0
                && let Some(fetched_q) = fetched_q_opt
            {
                active_queue_id.store(fetched_q, Ordering::Relaxed);
                current_q = fetched_q;
            }

            let transition = evaluate_ws_event("Update", &session_json, current_q);
            let defer_initial_inject = matches!(
                &transition,
                WsSessionTransition::Active(payload)
                    if should_defer_initial_inject_to_finalization(payload, &session_json)
            );
            sink.on_session_update(transition, &session_json, defer_initial_inject)
                .await;
        }
        Ok(None) => {
            sink.on_session_ended().await;
        }
        Err(_) => {}
    }
}

async fn poll_lcu_session_once_locked(ctx: &WsListenerCtx) {
    let Ok(conn) = crate::lcu_connector::LcuConnection::connect() else { return };
    let (gateway, sink) = (LiveLcuPollGateway { conn }, LiveLcuPollSink { ctx });
    poll_lcu_session_with_gateway(
        &gateway, &sink, &ctx.last_gameflow_phase, &ctx.session_state, &ctx.active_queue_id,
    )
    .await;
}

/// Runs a single serialized LCU gameflow + Champ Select probe, syncing `last_gameflow_phase`
/// and dispatching any active Champ Select transition + finalization timer.
pub(crate) async fn poll_lcu_session_once(ctx: &WsListenerCtx) {
    let _poll_guard = ctx.poll_lock.lock().await;
    poll_lcu_session_once_locked(ctx).await;
}

/// Returns the consolidated bootstrap snapshot of gameflow phase, active Champ Select payload,
/// and compatible cached build. Probes LCU on demand via `poll_lcu_session_once` if the backend
/// has not yet observed a gameflow phase or is in `ChampSelect` without a cached payload.
pub async fn get_active_champ_select_snapshot(
    app_handle: &tauri::AppHandle,
) -> ActiveChampSelectSnapshot {
    let ctx = WsListenerCtx::for_app(app_handle.clone());

    let needs_probe = {
        let phase = ctx.last_gameflow_phase.lock().await;
        let state = ctx.session_state.lock().await;
        should_probe_lcu_for_snapshot(&phase, &state)
    };

    if needs_probe {
        let _poll_guard = ctx.poll_lock.lock().await;
        let still_needs_probe = {
            let phase = ctx.last_gameflow_phase.lock().await;
            let state = ctx.session_state.lock().await;
            should_probe_lcu_for_snapshot(&phase, &state)
        };
        if still_needs_probe {
            poll_lcu_session_once_locked(&ctx).await;
        }
    }

    let phase = ctx.last_gameflow_phase.lock().await.clone();
    let state = ctx.session_state.lock().await;
    build_active_champ_select_snapshot(&phase, &state)
}

async fn run_fallback_poll_loop(ctx: WsListenerCtx) {
    let mut conn_rx = crate::lcu_connector::subscribe_lcu_connection();
    let mut needs_immediate_poll = true;

    loop {
        if !*conn_rx.borrow() {
            needs_immediate_poll = true;
            clear_active_session(&ctx, false).await;
            ctx.last_gameflow_phase.lock().await.clear();
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

        poll_lcu_session_once(&ctx).await;
    }
}

/// Starts the WebSocket listener and adaptive fallback polling task coordinated with `lcu_monitor`.
pub async fn start_ws_listener(app_handle: tauri::AppHandle) {
    let ctx = WsListenerCtx::for_app(app_handle);

    tokio::spawn(run_fallback_poll_loop(ctx.clone()));

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
            active_ws = Some(spawn_configured_ws(&ctx));
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
#[path = "polling_tests.rs"]
mod tests;

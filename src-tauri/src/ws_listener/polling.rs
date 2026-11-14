use irelia::ws::LcuWebSocket;
use irelia::ws::types::{Event, EventKind};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::time::Duration;
use tauri_specta::Event as SpectaEvent;

use super::extract::*;
use super::reconcile::*;
use crate::lcu_injector::AppliedInjectionState;
use crate::models;

#[derive(Default)]
struct FinalizationTimerSlot {
    deadline: Option<tokio::time::Instant>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

/// Coordinates asynchronous Champ Select build/injection tasks so rapid champion switches,
/// finalization timers, or session exits immediately cancel stale work and prevent out-of-order LCU writes.
#[derive(Clone, Default)]
pub struct ChampSelectCoordinator {
    selection_seq: Arc<AtomicU64>,
    active_inject_task: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
    finalization_timer: Arc<tokio::sync::Mutex<FinalizationTimerSlot>>,
}

impl ChampSelectCoordinator {
    #[cfg(test)]
    pub(crate) fn current_seq(&self) -> u64 {
        self.selection_seq.load(Ordering::SeqCst)
    }

    pub(crate) fn is_current(&self, seq: u64) -> bool {
        self.selection_seq.load(Ordering::SeqCst) == seq
    }

    pub(crate) async fn cancel_pending(&self) {
        self.selection_seq.fetch_add(1, Ordering::SeqCst);
        let mut guard = self.active_inject_task.lock().await;
        if let Some(handle) = guard.take() {
            handle.abort();
        }
    }

    pub(crate) async fn cancel_finalization_timer(&self) {
        let mut slot = self.finalization_timer.lock().await;
        slot.deadline = None;
        if let Some(handle) = slot.handle.take() {
            handle.abort();
        }
    }

    pub(crate) async fn schedule_finalization_timer<F, Fut>(&self, delay: Duration, make_task: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let desired_deadline = tokio::time::Instant::now() + delay;
        let mut slot = self.finalization_timer.lock().await;

        if let (Some(existing_deadline), Some(existing_handle)) =
            (slot.deadline, slot.handle.as_ref())
            && !existing_handle.is_finished()
        {
            let diff = desired_deadline.saturating_duration_since(existing_deadline)
                + existing_deadline.saturating_duration_since(desired_deadline);
            if diff <= Duration::from_millis(1200) {
                return;
            }
        }

        if let Some(prev) = slot.handle.take() {
            prev.abort();
        }

        slot.deadline = Some(desired_deadline);
        let handle = tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            make_task().await;
        });
        slot.handle = Some(handle);
    }

    pub(crate) async fn spawn_latest<F, Fut>(&self, make_task: F) -> u64
    where
        F: FnOnce(u64, ChampSelectCoordinator) -> Fut,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let seq = self.selection_seq.fetch_add(1, Ordering::SeqCst) + 1;
        let mut guard = self.active_inject_task.lock().await;
        if let Some(prev_handle) = guard.take() {
            prev_handle.abort();
        }
        let coordinator = self.clone();
        let handle = tokio::spawn(make_task(seq, coordinator));
        *guard = Some(handle);
        seq
    }
}

/// Returns true when a gameflow phase transition indicates a finished match
/// whose match history / ranked stats / masteries should be refreshed in the Profile tab.
pub fn should_request_profile_refresh(prev_phase: &str, next_phase: &str) -> bool {
    let was_in_game = matches!(prev_phase, "InProgress" | "Reconnect");
    let now_post_game = matches!(
        next_phase,
        "WaitingForStats" | "PreEndOfGame" | "EndOfGame" | "Lobby" | "None"
    );
    was_in_game && now_post_game
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

async fn trigger_live_reconciliation(
    handle: &tauri::AppHandle,
    state_ref: &Arc<tokio::sync::Mutex<ChampSelectSessionState>>,
    reconcile_lock: &Arc<tokio::sync::Mutex<()>>,
    q_ref: &Arc<AtomicI32>,
    coordinator: &ChampSelectCoordinator,
) {
    let Ok(conn) = crate::lcu_connector::LcuConnection::connect() else {
        return;
    };
    let gateway = LiveLcuReconcileGateway {
        conn: Arc::new(conn),
        coordinator: coordinator.clone(),
    };
    let emitter = TauriReconcileEmitter {
        handle: handle.clone(),
    };
    let prefs = crate::lcu_injector::get_user_preferences();
    let _ =
        reconcile_final_session(&gateway, &emitter, state_ref, reconcile_lock, q_ref, &prefs).await;
}

async fn process_session_timer(
    handle: &tauri::AppHandle,
    session_json: &serde_json::Value,
    state_ref: &Arc<tokio::sync::Mutex<ChampSelectSessionState>>,
    reconcile_lock: &Arc<tokio::sync::Mutex<()>>,
    q_ref: &Arc<AtomicI32>,
    coordinator: &ChampSelectCoordinator,
) {
    let Some(timer) = extract_champ_select_timer(session_json) else {
        coordinator.cancel_finalization_timer().await;
        return;
    };

    match evaluate_finalization_window(&timer, FINALIZATION_SAFETY_WINDOW_MS) {
        FinalizationAction::TriggerNow => {
            coordinator.cancel_finalization_timer().await;
            trigger_live_reconciliation(handle, state_ref, reconcile_lock, q_ref, coordinator)
                .await;
        }
        FinalizationAction::ScheduleAfter(delay) => {
            let h = handle.clone();
            let s_ref = Arc::clone(state_ref);
            let r_lock = Arc::clone(reconcile_lock);
            let q = Arc::clone(q_ref);
            let coord = coordinator.clone();
            coordinator
                .schedule_finalization_timer(delay, move || async move {
                    trigger_live_reconciliation(&h, &s_ref, &r_lock, &q, &coord).await;
                })
                .await;
        }
        FinalizationAction::NotApplicable => {
            coordinator.cancel_finalization_timer().await;
            let mut guard = state_ref.lock().await;
            guard.reconciled_champion_id = None;
        }
    }
}

/// Dispatches session state changes and triggers automated build injection when applicable.
async fn handle_session_transition(
    handle: &tauri::AppHandle,
    transition: WsSessionTransition,
    state_ref: &Arc<tokio::sync::Mutex<ChampSelectSessionState>>,
    q_ref: &Arc<AtomicI32>,
    coordinator: &ChampSelectCoordinator,
) {
    match transition {
        WsSessionTransition::Active(payload) => {
            let should_spawn = {
                let mut state_guard = state_ref.lock().await;
                let current_opt = Some(payload.clone());
                if should_emit_champ_select(&state_guard.last_payload, &current_opt) {
                    let champion_changed = state_guard
                        .last_payload
                        .as_ref()
                        .is_some_and(|prev| prev.champion_id != payload.champion_id);
                    if champion_changed {
                        crate::lcu_injector::clear_champion_selections();
                        coordinator.cancel_pending().await;
                    }
                    let _ = models::ChampSelected(payload.clone()).emit(handle);
                    state_guard.record_processed(payload.clone());
                    payload.locked_in || payload.queue_mode == QueueMode::PracticeTool
                } else {
                    false
                }
            };

            if should_spawn {
                let handle_for_inject = handle.clone();
                let state_for_inject = Arc::clone(state_ref);
                let champ_id = payload.champion_id;
                let q_id = resolve_effective_build_queue_id(&payload);
                let assigned_pos = resolve_effective_build_position(&payload, q_id);

                coordinator
                    .spawn_latest(move |my_seq, seq_guard| async move {
                        let conn_opt = crate::lcu_connector::LcuConnection::connect()
                            .ok()
                            .map(Arc::new);
                        let Some(conn) = conn_opt else {
                            return;
                        };

                        let http_client = crate::build_provider::shared_http_client();
                        let (fast_tx, fast_rx) = tokio::sync::oneshot::channel::<
                            crate::build_provider::FastSpellRecommendation,
                        >();

                        let summoner_conn = Arc::clone(&conn);
                        let summoner_fut =
                            async move { summoner_conn.get_current_summoner().await.ok() };

                        let spell_conn = Arc::clone(&conn);
                        let spell_seq_guard = seq_guard.clone();
                        let fast_spell_fut = async move {
                            let Ok(fast) = fast_rx.await else {
                                return false;
                            };
                            if !spell_seq_guard.is_current(my_seq) {
                                return false;
                            }
                            let prefs = crate::lcu_injector::get_user_preferences();
                            if !prefs.auto_inject_spells {
                                return false;
                            }
                            crate::lcu_injector::inject_spells_pair(
                                &spell_conn,
                                fast.champion_id,
                                fast.spell1_id,
                                fast.spell2_id,
                                fast.is_jungle,
                                prefs.flash_key,
                            )
                            .await
                            .is_ok()
                        };

                        let build_fut =
                            crate::build_provider::get_recommended_build_with_filters_and_fast_spells(
                                http_client,
                                champ_id,
                                q_id,
                                assigned_pos.as_deref(),
                                None,
                                None,
                                None,
                                Some(fast_tx),
                            );

                        let (summoner_opt, spells_already_injected, build_res) =
                            tokio::join!(summoner_fut, fast_spell_fut, build_fut);

                        if !seq_guard.is_current(my_seq) {
                            return;
                        }

                        let Ok(build) = build_res else {
                            return;
                        };

                        {
                            let mut guard = state_for_inject.lock().await;
                            if seq_guard.is_current(my_seq) {
                                guard.last_build = Some(build.clone());
                            }
                        }

                        let _ = models::RecommendedBuild(build.clone()).emit(&handle_for_inject);

                        let Some(summoner) = summoner_opt else {
                            return;
                        };

                        if !seq_guard.is_current(my_seq) {
                            return;
                        }

                        let report = crate::lcu_injector::inject_champion_build_with_options(
                            &conn,
                            summoner.summoner_id,
                            &build,
                            spells_already_injected,
                        )
                        .await;

                        if seq_guard.is_current(my_seq) {
                            let applied = AppliedInjectionState::from_report(q_id, &build, &report);
                            {
                                let mut guard = state_for_inject.lock().await;
                                if seq_guard.is_current(my_seq) {
                                    guard.record_build_and_applied(build.clone(), applied);
                                }
                            }
                            let _ = models::BuildInjected(report).emit(&handle_for_inject);
                        }
                    })
                    .await;
            }
        }
        WsSessionTransition::Ended => {
            coordinator.cancel_pending().await;
            coordinator.cancel_finalization_timer().await;
            let mut state_guard = state_ref.lock().await;
            if state_guard.last_payload.is_some() {
                let _ = models::ChampSelectEnded.emit(handle);
            }
            state_guard.reset_session();
            q_ref.store(0, Ordering::Relaxed);
        }
        WsSessionTransition::NoChampionSelected => {}
    }
}

/// Starts the WebSocket listener and fallback polling task to observe Champion Select events in real time.
pub async fn start_ws_listener(app_handle: tauri::AppHandle) {
    let active_queue_id = Arc::new(AtomicI32::new(0));
    let session_state = shared_session_state();
    let reconcile_lock = shared_reconcile_lock();
    let coordinator = ChampSelectCoordinator::default();
    let last_gameflow_phase = Arc::new(tokio::sync::Mutex::new(String::new()));

    // Fallback sync loop: ensures Champion Select and FINALIZATION timer countdowns are observed
    // even if WebSocket does not emit per-second timer ticks or missed an event.
    let fallback_handle = app_handle.clone();
    let fallback_q_ref = Arc::clone(&active_queue_id);
    let fallback_state_ref = Arc::clone(&session_state);
    let fallback_reconcile_lock = Arc::clone(&reconcile_lock);
    let fallback_coord = coordinator.clone();

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
                        &fallback_state_ref,
                        &fallback_q_ref,
                        &fallback_coord,
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
                    let mut state_guard = fallback_state_ref.lock().await;
                    if state_guard.last_payload.is_some() {
                        let _ = models::ChampSelectEnded.emit(&fallback_handle);
                    }
                    state_guard.reset_session();
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
    let ws_state_ref = Arc::clone(&session_state);
    let ws_reconcile_lock = Arc::clone(&reconcile_lock);
    let ws_coord = coordinator.clone();

    ws.subscribe(
        EventKind::json_api_event_callback_str("/lol-champ-select/v1/session"),
        move |event: &Event| {
            let handle = ws_handle.clone();
            let q_ref = Arc::clone(&ws_q_ref);
            let state_ref = Arc::clone(&ws_state_ref);
            let r_lock = Arc::clone(&ws_reconcile_lock);
            let coord = ws_coord.clone();

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
                handle_session_transition(&handle, transition, &state_ref, &q_ref, &coord).await;

                if is_active {
                    process_session_timer(&handle, &data, &state_ref, &r_lock, &q_ref, &coord)
                        .await;
                }
            });
        },
    );

    // Subscribe to gameflow phase changes to automatically refresh player profile after a match
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
                let should_refresh = {
                    let mut guard = state.lock().await;
                    let trigger = should_request_profile_refresh(&guard, &next_phase);
                    *guard = next_phase;
                    trigger
                };

                if should_refresh {
                    tokio::time::sleep(Duration::from_millis(1500)).await;
                    let _ = models::ProfileRefreshRequested.emit(&handle);
                }
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
    fn test_should_request_profile_refresh_on_post_game_transition() {
        assert!(should_request_profile_refresh(
            "InProgress",
            "WaitingForStats"
        ));
        assert!(should_request_profile_refresh("InProgress", "EndOfGame"));
        assert!(should_request_profile_refresh("Reconnect", "Lobby"));
        assert!(!should_request_profile_refresh("Lobby", "ChampSelect"));
        assert!(!should_request_profile_refresh("ChampSelect", "InProgress"));
        assert!(!should_request_profile_refresh("EndOfGame", "Lobby"));
    }

    #[tokio::test]
    async fn test_coordinator_cancels_stale_task_on_rapid_champion_switch() {
        let coord = ChampSelectCoordinator::default();
        let completed = Arc::new(tokio::sync::Mutex::new(Vec::<i32>::new()));

        let c1 = Arc::clone(&completed);
        let seq1 = coord
            .spawn_latest(move |my_seq, seq_guard| async move {
                tokio::time::sleep(Duration::from_millis(80)).await;
                if seq_guard.is_current(my_seq) {
                    c1.lock().await.push(18);
                }
            })
            .await;

        let c2 = Arc::clone(&completed);
        let seq2 = coord
            .spawn_latest(move |my_seq, seq_guard| async move {
                tokio::time::sleep(Duration::from_millis(15)).await;
                if seq_guard.is_current(my_seq) {
                    c2.lock().await.push(103);
                }
            })
            .await;

        assert_ne!(seq1, seq2);
        assert!(!coord.is_current(seq1));
        assert!(coord.is_current(seq2));

        tokio::time::sleep(Duration::from_millis(120)).await;
        let finished = completed.lock().await.clone();
        assert_eq!(finished, vec![103]);
    }

    #[tokio::test]
    async fn test_coordinator_cancel_pending_invalidates_sequence_on_champ_select_end() {
        let coord = ChampSelectCoordinator::default();
        let completed = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let c1 = Arc::clone(&completed);
        let seq1 = coord
            .spawn_latest(move |my_seq, seq_guard| async move {
                tokio::time::sleep(Duration::from_millis(60)).await;
                if seq_guard.is_current(my_seq) {
                    c1.store(true, Ordering::SeqCst);
                }
            })
            .await;

        assert!(coord.is_current(seq1));
        coord.cancel_pending().await;
        assert!(!coord.is_current(seq1));
        assert_ne!(coord.current_seq(), seq1);

        tokio::time::sleep(Duration::from_millis(80)).await;
        assert!(!completed.load(Ordering::SeqCst));
    }
}

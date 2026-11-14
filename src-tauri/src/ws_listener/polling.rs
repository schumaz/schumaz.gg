use irelia::ws::LcuWebSocket;
use irelia::ws::types::{Event, EventKind};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::time::Duration;
use tauri_specta::Event as SpectaEvent;

use super::extract::*;
use crate::models;

/// Coordinates asynchronous Champ Select build/injection tasks so rapid champion switches
/// or session exits immediately cancel stale work and prevent out-of-order LCU writes.
#[derive(Clone, Default)]
pub(crate) struct ChampSelectCoordinator {
    selection_seq: Arc<AtomicU64>,
    active_inject_task: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
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
    coordinator: &ChampSelectCoordinator,
) {
    match transition {
        WsSessionTransition::Active(payload) => {
            let mut last_guard = last_payload_ref.lock().await;
            let current_opt = Some(payload.clone());
            if should_emit_champ_select(&last_guard, &current_opt) {
                let champion_changed = last_guard
                    .as_ref()
                    .is_some_and(|prev| prev.champion_id != payload.champion_id);
                if champion_changed {
                    crate::lcu_injector::clear_champion_selections();
                    coordinator.cancel_pending().await;
                }
                let _ = models::ChampSelected(payload.clone()).emit(handle);
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
                    let assigned_pos = if q_id == QUEUE_ARAM || q_id == QUEUE_ARAM_MAYHEM {
                        None
                    } else {
                        payload.assigned_position.clone()
                    };

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

                            // 1. Fetch current summoner concurrently with OP.GG build fetch
                            let summoner_conn = Arc::clone(&conn);
                            let summoner_fut =
                                async move { summoner_conn.get_current_summoner().await.ok() };

                            // 2. Fast-path spell injection: patches LCU as soon as fast OP.GG REST resolves,
                            // without waiting for slow MCP depth items, get_current_summoner, or runes.
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

                            // 3. Full build retrieval (REST + concurrent bounded MCP depth items)
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
                                let _ = models::BuildInjected(report).emit(&handle_for_inject);
                            }
                        })
                        .await;
                }
            }
        }
        WsSessionTransition::Ended => {
            coordinator.cancel_pending().await;
            let mut last_guard = last_payload_ref.lock().await;
            if last_guard.is_some() {
                let _ = models::ChampSelectEnded.emit(handle);
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
    let coordinator = ChampSelectCoordinator::default();

    // Fallback sync loop: Ensures Champion Select (especially Practice Tool) is detected
    // even if the WebSocket missed an event, was mid-reconnection, or client was opened later.
    let fallback_handle = app_handle.clone();
    let fallback_q_ref = Arc::clone(&active_queue_id);
    let fallback_last_ref = Arc::clone(&last_payload);
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
                        &fallback_last_ref,
                        &fallback_q_ref,
                        &fallback_coord,
                    )
                    .await;
                }
                Ok(None) => {
                    fallback_coord.cancel_pending().await;
                    let mut last_guard = fallback_last_ref.lock().await;
                    if last_guard.is_some() {
                        let _ = models::ChampSelectEnded.emit(&fallback_handle);
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
    let ws_coord = coordinator.clone();

    ws.subscribe(
        EventKind::json_api_event_callback_str("/lol-champ-select/v1/session"),
        move |event: &Event| {
            let handle = ws_handle.clone();
            let q_ref = Arc::clone(&ws_q_ref);
            let last_payload_ref = Arc::clone(&ws_last_ref);
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
                handle_session_transition(&handle, transition, &last_payload_ref, &q_ref, &coord)
                    .await;
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

        // Simulate rapid champion switch before task 1 finishes
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

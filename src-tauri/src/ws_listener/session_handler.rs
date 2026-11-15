use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use tauri_specta::Event as SpectaEvent;

use super::coordinator::ChampSelectCoordinator;
use super::extract::{
    ChampSelectedPayload, FINALIZATION_SAFETY_WINDOW_MS, FinalizationAction, QUEUE_ARAM,
    QUEUE_ARAM_MAYHEM, QUEUE_PRACTICE_TOOL, QUEUE_SOLO_DUO, QueueMode, WsSessionTransition,
    evaluate_finalization_window, extract_champ_select_timer, should_emit_champ_select,
    should_request_lock_in_attention,
};
use super::gameflow::sync_gameflow_phase_before_champ_select_end;
use super::reconcile::{LiveLcuReconcileGateway, TauriReconcileEmitter, reconcile_final_session};
use super::reconcile_state::{
    ChampSelectSessionState, record_filter_updated_build, resolve_effective_build_position,
    resolve_effective_build_queue_id,
};
use crate::lcu_injector::AppliedInjectionState;
use crate::models;

pub(crate) async fn trigger_live_reconciliation(
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

pub(crate) async fn process_session_timer(
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

async fn run_champ_select_build_and_inject(
    handle: tauri::AppHandle,
    state_ref: Arc<tokio::sync::Mutex<ChampSelectSessionState>>,
    payload: ChampSelectedPayload,
    my_seq: u64,
    seq_guard: ChampSelectCoordinator,
) {
    let conn_opt = crate::lcu_connector::LcuConnection::connect()
        .ok()
        .map(Arc::new);
    let Some(conn) = conn_opt else {
        return;
    };

    let champ_id = payload.champion_id;
    let q_id = resolve_effective_build_queue_id(&payload);
    let assigned_pos = resolve_effective_build_position(&payload, q_id);
    let http_client = crate::build_provider::shared_http_client();
    let (fast_tx, fast_rx) =
        tokio::sync::oneshot::channel::<crate::build_provider::FastSpellRecommendation>();

    let summoner_conn = Arc::clone(&conn);
    let summoner_fut = async move { summoner_conn.get_current_summoner().await.ok() };

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

    let build_fut = crate::build_provider::get_recommended_build_with_filters_and_fast_spells(
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
        let mut guard = state_ref.lock().await;
        if seq_guard.is_current(my_seq) {
            guard.last_build = Some(build.clone());
        }
    }

    let _ = models::RecommendedBuild(build.clone()).emit(&handle);

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
            let mut guard = state_ref.lock().await;
            if seq_guard.is_current(my_seq) {
                guard.record_build_and_applied(build.clone(), applied);
            }
        }
        let _ = models::BuildInjected(report).emit(&handle);
    }
}

/// Dispatches session state changes and triggers automated build injection when applicable.
pub(crate) async fn handle_session_transition(
    handle: &tauri::AppHandle,
    transition: WsSessionTransition,
    state_ref: &Arc<tokio::sync::Mutex<ChampSelectSessionState>>,
    q_ref: &Arc<AtomicI32>,
    coordinator: &ChampSelectCoordinator,
    phase_state: &Arc<tokio::sync::Mutex<String>>,
) {
    match transition {
        WsSessionTransition::Active(payload) => {
            let (should_spawn, should_notify_lock) = {
                let mut state_guard = state_ref.lock().await;
                let current_opt = Some(payload.clone());
                if should_emit_champ_select(&state_guard.last_payload, &current_opt) {
                    let notify_lock = should_request_lock_in_attention(
                        state_guard.last_payload.as_ref(),
                        &payload,
                    );
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
                    (
                        payload.locked_in || payload.queue_mode == QueueMode::PracticeTool,
                        notify_lock,
                    )
                } else {
                    (false, false)
                }
            };

            if should_notify_lock {
                crate::request_taskbar_attention(handle);
            }

            if should_spawn {
                let handle_for_inject = handle.clone();
                let state_for_inject = Arc::clone(state_ref);
                coordinator
                    .spawn_latest(move |my_seq, seq_guard| {
                        run_champ_select_build_and_inject(
                            handle_for_inject,
                            state_for_inject,
                            payload,
                            my_seq,
                            seq_guard,
                        )
                    })
                    .await;
            }
        }
        WsSessionTransition::Ended => {
            coordinator.cancel_pending().await;
            coordinator.cancel_finalization_timer().await;
            let had_session = {
                let mut state_guard = state_ref.lock().await;
                let was_active = state_guard.last_payload.is_some();
                state_guard.reset_session();
                was_active
            };
            q_ref.store(0, Ordering::Relaxed);
            if had_session {
                sync_gameflow_phase_before_champ_select_end(handle, phase_state).await;
                let _ = models::ChampSelectEnded.emit(handle);
            }
        }
        WsSessionTransition::NoChampionSelected => {}
    }
}

/// Refreshes the build with updated user filters, emits the build event, and re-injects if LCU is active.
pub async fn update_build_filters_and_reinject(
    champion_id: i32,
    queue_id: Option<i32>,
    position: Option<String>,
    tier: Option<String>,
    region: Option<String>,
    patch: Option<String>,
    app_handle: &tauri::AppHandle,
) -> Result<crate::build_provider::ChampionBuild, String> {
    let http_client = crate::build_provider::shared_http_client();
    let q_id = queue_id.unwrap_or(QUEUE_SOLO_DUO);
    let is_aram = q_id == QUEUE_ARAM || q_id == QUEUE_ARAM_MAYHEM;
    let effective_position = if is_aram { None } else { position.as_deref() };

    let build = crate::build_provider::get_recommended_build_with_filters(
        http_client,
        champion_id,
        q_id,
        effective_position,
        tier.as_deref(),
        region.as_deref(),
        patch.as_deref(),
    )
    .await?;

    let _ = models::RecommendedBuild(build.clone()).emit(app_handle);

    if let Ok(conn) = crate::lcu_connector::LcuConnection::connect()
        && let Ok(summoner) = conn.get_current_summoner().await
    {
        let report =
            crate::lcu_injector::inject_champion_build(&conn, summoner.summoner_id, &build).await;
        let effective_q = if q_id == 0 || q_id == QUEUE_PRACTICE_TOOL {
            QUEUE_SOLO_DUO
        } else {
            q_id
        };
        let applied = AppliedInjectionState::from_report(effective_q, &build, &report);
        record_filter_updated_build(build.clone(), applied).await;
        let _ = models::BuildInjected(report).emit(app_handle);
    }

    Ok(build)
}

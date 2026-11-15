use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use tauri_specta::Event as SpectaEvent;

use super::coordinator::ChampSelectCoordinator;
use super::extract::{
    ChampSelectedPayload, QueueMode, WsSessionTransition, evaluate_ws_event,
    should_request_lock_in_attention,
};
use super::reconcile_state::{
    ChampSelectSessionState, ReconciliationDecision, build_matches_payload,
    evaluate_reconciliation_decision, resolve_effective_build_position,
    resolve_effective_build_queue_id,
};
use crate::build_provider::ChampionBuild;
use crate::lcu_injector::{AppliedInjectionState, InjectionReport, UserPreferences};
use crate::models::{self, ChampSelectReconciledPayload};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinalReconcileOutcome {
    SessionEnded,
    SkippedNoLockedChampion,
    AlreadyReconciled,
    Reconciled {
        champion_id: i32,
        champion_diverged: bool,
        cleared_selections: bool,
    },
}

/// Gateway abstraction for querying LCU and applying safety re-injection (mockable in tests).
pub trait FinalReconcileGateway: Send + Sync {
    fn get_champ_select_session(
        &self,
    ) -> impl std::future::Future<Output = Result<Option<serde_json::Value>, String>> + Send;

    fn get_current_queue_id(&self)
    -> impl std::future::Future<Output = Result<i32, String>> + Send;

    fn get_summoner_id(&self) -> impl std::future::Future<Output = Result<u64, String>> + Send;

    fn fetch_build(
        &self,
        champion_id: i32,
        queue_id: i32,
        assigned_position: Option<&str>,
    ) -> impl std::future::Future<Output = Result<ChampionBuild, String>> + Send;

    fn safety_reinject(
        &self,
        summoner_id: u64,
        queue_id: i32,
        build: &ChampionBuild,
    ) -> impl std::future::Future<Output = (InjectionReport, AppliedInjectionState)> + Send;

    fn clear_selections(&self);

    fn cancel_pending_tasks(&self) -> impl std::future::Future<Output = ()> + Send;
}

/// Event emitter abstraction for decoupling Tauri IPC emission from unit tests.
pub trait ReconcileEventEmitter: Send + Sync {
    fn emit_champ_selected(&self, payload: ChampSelectedPayload);
    fn emit_recommended_build(&self, build: ChampionBuild);
    fn emit_champ_select_reconciled(&self, payload: ChampSelectReconciledPayload);
    fn emit_build_injected(&self, report: InjectionReport);
    fn emit_champ_select_ended(&self);
    fn request_lock_in_attention(&self);
}

pub struct TauriReconcileEmitter {
    pub handle: tauri::AppHandle,
}

impl ReconcileEventEmitter for TauriReconcileEmitter {
    fn emit_champ_selected(&self, payload: ChampSelectedPayload) {
        let _ = models::ChampSelected(payload).emit(&self.handle);
    }

    fn emit_recommended_build(&self, build: ChampionBuild) {
        let _ = models::RecommendedBuild(build).emit(&self.handle);
    }

    fn emit_champ_select_reconciled(&self, payload: ChampSelectReconciledPayload) {
        let _ = models::ChampSelectReconciled(payload).emit(&self.handle);
    }

    fn emit_build_injected(&self, report: InjectionReport) {
        let _ = models::BuildInjected(report).emit(&self.handle);
    }

    fn emit_champ_select_ended(&self) {
        let _ = models::ChampSelectEnded.emit(&self.handle);
    }

    fn request_lock_in_attention(&self) {
        crate::request_taskbar_attention(&self.handle);
    }
}

pub struct LiveLcuReconcileGateway {
    pub conn: Arc<crate::lcu_connector::LcuConnection>,
    pub coordinator: ChampSelectCoordinator,
}

impl FinalReconcileGateway for LiveLcuReconcileGateway {
    async fn get_champ_select_session(&self) -> Result<Option<serde_json::Value>, String> {
        self.conn.get_champ_select_session().await
    }

    async fn get_current_queue_id(&self) -> Result<i32, String> {
        self.conn.get_current_queue_id().await
    }

    async fn get_summoner_id(&self) -> Result<u64, String> {
        self.conn
            .get_current_summoner()
            .await
            .map(|s| s.summoner_id)
    }

    async fn fetch_build(
        &self,
        champion_id: i32,
        queue_id: i32,
        assigned_position: Option<&str>,
    ) -> Result<ChampionBuild, String> {
        let http_client = crate::build_provider::shared_http_client();
        crate::build_provider::get_recommended_build_with_filters(
            http_client,
            champion_id,
            queue_id,
            assigned_position,
            None,
            None,
            None,
        )
        .await
    }

    async fn safety_reinject(
        &self,
        summoner_id: u64,
        queue_id: i32,
        build: &ChampionBuild,
    ) -> (InjectionReport, AppliedInjectionState) {
        crate::lcu_injector::safety_reinject_build(&self.conn, summoner_id, queue_id, build).await
    }

    fn clear_selections(&self) {
        crate::lcu_injector::clear_champion_selections();
    }

    async fn cancel_pending_tasks(&self) {
        self.coordinator.cancel_pending().await;
    }
}

/// Re-queries `/lol-champ-select/v1/session` near the end of FINALIZATION, reconciles any
/// backend/UI champion divergence, and performs a safety re-injection before game load.
pub async fn reconcile_final_session<G, E>(
    gateway: &G,
    emitter: &E,
    state_ref: &Arc<tokio::sync::Mutex<ChampSelectSessionState>>,
    reconcile_lock: &Arc<tokio::sync::Mutex<()>>,
    q_ref: &Arc<AtomicI32>,
    prefs: &UserPreferences,
) -> Result<FinalReconcileOutcome, String>
where
    G: FinalReconcileGateway,
    E: ReconcileEventEmitter,
{
    let _reconcile_guard = reconcile_lock.lock().await;

    let session_opt = gateway.get_champ_select_session().await?;
    let Some(session_json) = session_opt else {
        gateway.cancel_pending_tasks().await;
        let had_session = {
            let mut guard = state_ref.lock().await;
            let was_active = guard.last_payload.is_some();
            guard.reset_session();
            was_active
        };
        if had_session {
            emitter.emit_champ_select_ended();
        }
        q_ref.store(0, Ordering::Relaxed);
        return Ok(FinalReconcileOutcome::SessionEnded);
    };

    let mut current_q = q_ref.load(Ordering::Relaxed);
    if current_q <= 0
        && let Ok(fetched_q) = gateway.get_current_queue_id().await
    {
        q_ref.store(fetched_q, Ordering::Relaxed);
        current_q = fetched_q;
    }

    let WsSessionTransition::Active(actual) = evaluate_ws_event("Update", &session_json, current_q)
    else {
        return Ok(FinalReconcileOutcome::SkippedNoLockedChampion);
    };

    if !actual.locked_in && actual.queue_mode != QueueMode::PracticeTool {
        return Ok(FinalReconcileOutcome::SkippedNoLockedChampion);
    }

    let build_q_id = resolve_effective_build_queue_id(&actual);
    let effective_pos = resolve_effective_build_position(&actual, build_q_id);

    let (decision, cached_build) = {
        let guard = state_ref.lock().await;
        let decision = evaluate_reconciliation_decision(&guard, &actual, prefs, build_q_id);
        (decision, guard.last_build.clone())
    };

    let (champion_diverged, clear_previous_selections, need_build_fetch) = match decision {
        ReconciliationDecision::AlreadyReconciled => {
            return Ok(FinalReconcileOutcome::AlreadyReconciled);
        }
        ReconciliationDecision::Diverged {
            clear_previous_selections,
            need_build_fetch,
        } => (true, clear_previous_selections, need_build_fetch),
        ReconciliationDecision::SafetyReinject { need_build_fetch } => {
            (false, false, need_build_fetch)
        }
    };

    if champion_diverged {
        if clear_previous_selections {
            gateway.clear_selections();
        }
        gateway.cancel_pending_tasks().await;
    }

    let build = if need_build_fetch {
        gateway
            .fetch_build(actual.champion_id, build_q_id, effective_pos.as_deref())
            .await?
    } else if let Some(existing) = cached_build
        && build_matches_payload(&existing, &actual, build_q_id)
    {
        existing
    } else {
        gateway
            .fetch_build(actual.champion_id, build_q_id, effective_pos.as_deref())
            .await?
    };

    let summoner_id = gateway.get_summoner_id().await?;
    let (report, applied) = gateway
        .safety_reinject(summoner_id, build_q_id, &build)
        .await;

    let should_notify_lock = {
        let mut guard = state_ref.lock().await;
        let notify = should_request_lock_in_attention(guard.last_payload.as_ref(), &actual);
        guard.last_payload = Some(actual.clone());
        guard.last_build = Some(build.clone());
        guard.last_applied = Some(applied);
        guard.reconciled_champion_id = Some(actual.champion_id);
        notify
    };

    if champion_diverged {
        emitter.emit_champ_selected(actual.clone());
        emitter.emit_recommended_build(build.clone());
    }
    if should_notify_lock {
        emitter.request_lock_in_attention();
    }
    emitter.emit_champ_select_reconciled(ChampSelectReconciledPayload {
        champ_selected: actual.clone(),
        build: Some(build),
        champion_diverged,
    });
    emitter.emit_build_injected(report);

    Ok(FinalReconcileOutcome::Reconciled {
        champion_id: actual.champion_id,
        champion_diverged,
        cleared_selections: clear_previous_selections,
    })
}

#[cfg(test)]
#[path = "reconcile_tests.rs"]
mod tests;

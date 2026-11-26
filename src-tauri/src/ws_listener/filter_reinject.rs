use std::sync::atomic::Ordering;
use tauri_specta::Event as SpectaEvent;

use super::extract::{QUEUE_ARAM, QUEUE_ARAM_MAYHEM, QUEUE_PRACTICE_TOOL, QUEUE_SOLO_DUO, is_arena_queue};
use super::gameflow::is_in_game_phase;
use super::reconcile_state::{
    ChampSelectSessionState, record_filter_updated_build, shared_coordinator,
    shared_last_gameflow_phase, shared_session_state,
};
use crate::build_provider::ChampionBuild;
use crate::lcu_injector::{AppliedInjectionState, InjectionReport};
use crate::models::{self, canonical_build_queue_id};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActiveFilterTarget {
    StandardChampSelect,
    LobbyPreselect,
}

pub(crate) trait FilterReinjectGateway: Send + Sync {
    fn fetch_build(
        &self,
        champion_id: i32,
        queue_id: i32,
        effective_position: Option<&str>,
        tier: Option<&str>,
        region: Option<&str>,
        patch: Option<&str>,
    ) -> impl std::future::Future<Output = Result<ChampionBuild, String>> + Send;

    fn clear_champion_selection(&self, champion_id: i32);

    fn emit_recommended_build(&self, build: ChampionBuild);

    fn reinject_lobby_filter_update(
        &self,
        champion_id: i32,
        build: &ChampionBuild,
    ) -> impl std::future::Future<Output = bool> + Send;

    fn inject_standard_build(
        &self,
        queue_id: i32,
        build: &ChampionBuild,
    ) -> impl std::future::Future<Output = Option<(InjectionReport, AppliedInjectionState)>> + Send;

    fn emit_build_injected(&self, report: InjectionReport);
    fn set_claimed_seq(&self, _seq: Option<u64>) {}
}

struct LiveFilterReinjectGateway<'a> {
    app_handle: &'a tauri::AppHandle,
    coordinator: &'a super::coordinator::ChampSelectCoordinator,
    claimed_seq: std::sync::atomic::AtomicU64,
    state_ref: &'a std::sync::Arc<tokio::sync::Mutex<ChampSelectSessionState>>,
    phase_ref: &'a std::sync::Arc<tokio::sync::Mutex<String>>,
}

impl FilterReinjectGateway for LiveFilterReinjectGateway<'_> {
    fn set_claimed_seq(&self, seq: Option<u64>) {
        self.claimed_seq.store(seq.unwrap_or(0), Ordering::SeqCst);
    }
    async fn fetch_build(
        &self,
        champion_id: i32,
        queue_id: i32,
        effective_position: Option<&str>,
        tier: Option<&str>,
        region: Option<&str>,
        patch: Option<&str>,
    ) -> Result<ChampionBuild, String> {
        let http_client = crate::build_provider::shared_http_client();
        crate::build_provider::get_recommended_build_with_filters(
            http_client,
            champion_id,
            queue_id,
            effective_position,
            tier,
            region,
            patch,
        )
        .await
    }

    fn clear_champion_selection(&self, champion_id: i32) {
        crate::lcu_injector::clear_champion_selection_for_champion(champion_id);
    }

    fn emit_recommended_build(&self, build: ChampionBuild) {
        let _ = models::RecommendedBuild(build).emit(self.app_handle);
    }

    async fn reinject_lobby_filter_update(&self, champion_id: i32, build: &ChampionBuild) -> bool {
        super::lobby_actions::reinject_active_lobby_filter_update(
            champion_id,
            build,
            self.app_handle,
        )
        .await
    }

    async fn inject_standard_build(
        &self,
        queue_id: i32,
        build: &ChampionBuild,
    ) -> Option<(InjectionReport, AppliedInjectionState)> {
        let conn = crate::lcu_connector::LcuConnection::connect().ok()?;
        let summoner = conn.get_current_summoner().await.ok()?;
        let seq = self.claimed_seq.load(Ordering::SeqCst);
        if seq != 0 && !self.coordinator.is_current(seq) {
            return None;
        }
        if !is_standard_target_still_active(self.state_ref, self.phase_ref, build.champion_id)
            .await
        {
            return None;
        }
        let effective_q = if queue_id == 0 || queue_id == QUEUE_PRACTICE_TOOL {
            QUEUE_SOLO_DUO
        } else {
            queue_id
        };
        let report = crate::lcu_injector::inject_champion_build_with_options_for_queue(
            &conn,
            summoner.summoner_id,
            Some(effective_q),
            build,
            false,
        )
        .await;
        let applied = AppliedInjectionState::from_report(effective_q, build, &report);
        record_filter_updated_build(build.clone(), applied.clone()).await;
        Some((report, applied))
    }

    fn emit_build_injected(&self, report: InjectionReport) {
        let _ = models::BuildInjected(report).emit(self.app_handle);
    }
}

pub(crate) fn resolve_active_filter_target(
    state: &ChampSelectSessionState,
    phase: &str,
    champion_id: i32,
) -> Option<ActiveFilterTarget> {
    if is_in_game_phase(phase) {
        return None;
    }
    let payload = state.last_payload.as_ref()?;
    if payload.is_lobby_preselect {
        let matches_lobby = payload.champion_id == champion_id
            || payload
                .lobby_slots
                .iter()
                .any(|slot| slot.champion_id == champion_id);
        matches_lobby.then_some(ActiveFilterTarget::LobbyPreselect)
    } else if payload.champion_id == champion_id {
        Some(ActiveFilterTarget::StandardChampSelect)
    } else {
        None
    }
}

fn state_position_differs(
    state: &ChampSelectSessionState,
    champion_id: i32,
    effective_position: Option<&str>,
) -> bool {
    let Some(pos) = effective_position.map(str::trim).filter(|p| !p.is_empty()) else {
        return false;
    };
    if let Some(build) = state
        .last_build
        .as_ref()
        .filter(|b| b.champion_id == champion_id)
    {
        let build_pos = build.active_position.trim();
        if !build_pos.is_empty() {
            return !build_pos.eq_ignore_ascii_case(pos);
        }
    }
    if let Some(payload) = state
        .last_payload
        .as_ref()
        .filter(|p| p.champion_id == champion_id)
        && let Some(assigned) = payload
            .assigned_position
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty())
    {
        return !assigned.eq_ignore_ascii_case(pos);
    }
    false
}

pub(crate) fn should_clear_selections_for_filter_update(
    state: &ChampSelectSessionState,
    champion_id: i32,
    effective_position: Option<&str>,
    tier: Option<&str>,
    region: Option<&str>,
    patch: Option<&str>,
) -> bool {
    tier.is_some()
        || region.is_some()
        || patch.is_some()
        || state_position_differs(state, champion_id, effective_position)
}

async fn read_active_target(
    state_ref: &std::sync::Arc<tokio::sync::Mutex<ChampSelectSessionState>>,
    phase_ref: &std::sync::Arc<tokio::sync::Mutex<String>>,
    champion_id: i32,
) -> Option<ActiveFilterTarget> {
    let phase = phase_ref.lock().await.clone();
    let state = state_ref.lock().await;
    resolve_active_filter_target(&state, &phase, champion_id)
}

async fn is_standard_target_still_active(
    state_ref: &std::sync::Arc<tokio::sync::Mutex<ChampSelectSessionState>>,
    phase_ref: &std::sync::Arc<tokio::sync::Mutex<String>>,
    champion_id: i32,
) -> bool {
    read_active_target(state_ref, phase_ref, champion_id).await
        == Some(ActiveFilterTarget::StandardChampSelect)
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_filter_reinject_with_gateway<G: FilterReinjectGateway>(
    gateway: &G,
    champion_id: i32,
    queue_id: Option<i32>,
    position: Option<&str>,
    tier: Option<&str>,
    region: Option<&str>,
    patch: Option<&str>,
    state_ref: &std::sync::Arc<tokio::sync::Mutex<ChampSelectSessionState>>,
    phase_ref: &std::sync::Arc<tokio::sync::Mutex<String>>,
    coordinator: &super::coordinator::ChampSelectCoordinator,
) -> Result<ChampionBuild, String> {
    let q_id = canonical_build_queue_id(queue_id.unwrap_or(QUEUE_SOLO_DUO));
    let no_position_mode = q_id == QUEUE_ARAM || q_id == QUEUE_ARAM_MAYHEM || is_arena_queue(q_id);
    let effective_position = if no_position_mode { None } else { position };

    let (initial_target, filters_changed) = {
        let phase = phase_ref.lock().await.clone();
        let state = state_ref.lock().await;
        let target = resolve_active_filter_target(&state, &phase, champion_id);
        let changed = should_clear_selections_for_filter_update(
            &state,
            champion_id,
            effective_position,
            tier,
            region,
            patch,
        );
        (target, changed)
    };

    let claimed_seq = if initial_target.is_some() {
        let seq = coordinator.claim_next_seq().await;
        gateway.set_claimed_seq(Some(seq));
        if filters_changed {
            gateway.clear_champion_selection(champion_id);
        }
        Some(seq)
    } else {
        None
    };

    let build = gateway
        .fetch_build(champion_id, q_id, effective_position, tier, region, patch)
        .await?;

    let Some(my_seq) = claimed_seq else {
        return Ok(build);
    };

    let _inject_guard = coordinator.lock_injection().await;
    if !coordinator.is_current(my_seq) {
        return Ok(build);
    }

    let Some(active_target) = read_active_target(state_ref, phase_ref, champion_id).await else {
        return Ok(build);
    };

    if !coordinator.is_current(my_seq) {
        return Ok(build);
    }

    gateway.emit_recommended_build(build.clone());

    match active_target {
        ActiveFilterTarget::LobbyPreselect => {
            let _ = gateway.reinject_lobby_filter_update(champion_id, &build).await;
        }
        ActiveFilterTarget::StandardChampSelect => {
            if let Some((report, _applied)) = gateway.inject_standard_build(q_id, &build).await {
                gateway.emit_build_injected(report);
            }
        }
    }

    Ok(build)
}

/// Refreshes the build with updated user filters, emits the build event, and re-injects
/// serialized through `ChampSelectCoordinator` only while the session remains active and current.
pub async fn update_build_filters_and_reinject(
    champion_id: i32,
    queue_id: Option<i32>,
    position: Option<String>,
    tier: Option<String>,
    region: Option<String>,
    patch: Option<String>,
    app_handle: &tauri::AppHandle,
) -> Result<ChampionBuild, String> {
    let state_ref = shared_session_state();
    let phase_ref = shared_last_gameflow_phase();
    let coordinator = shared_coordinator();
    let gateway = LiveFilterReinjectGateway {
        app_handle,
        coordinator: &coordinator,
        claimed_seq: std::sync::atomic::AtomicU64::new(0),
        state_ref: &state_ref,
        phase_ref: &phase_ref,
    };

    run_filter_reinject_with_gateway(
        &gateway,
        champion_id,
        queue_id,
        position.as_deref(),
        tier.as_deref(),
        region.as_deref(),
        patch.as_deref(),
        &state_ref,
        &phase_ref,
        &coordinator,
    )
    .await
}

#[cfg(test)]
#[path = "filter_reinject_tests.rs"]
mod tests;

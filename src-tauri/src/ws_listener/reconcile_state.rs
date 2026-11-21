use std::sync::atomic::AtomicI32;
use std::sync::{Arc, OnceLock};

use super::coordinator::ChampSelectCoordinator;
use super::extract::{
    ChampSelectedPayload, QUEUE_ARAM, QUEUE_ARAM_MAYHEM, QUEUE_PRACTICE_TOOL, QUEUE_SOLO_DUO,
    is_arena_queue, queue_supports_runes, queue_supports_spells,
};
use crate::build_provider::ChampionBuild;
use crate::lcu_injector::{AppliedInjectionState, UserPreferences};
use crate::models::ActiveChampSelectSnapshot;

/// Holds the backend's latest processed, built, injected, and UI-displayed state for Champ Select.
#[derive(Debug, Clone, Default)]
pub struct ChampSelectSessionState {
    pub last_payload: Option<ChampSelectedPayload>,
    pub last_build: Option<ChampionBuild>,
    pub last_applied: Option<AppliedInjectionState>,
    pub ui_displayed_champion_id: Option<i32>,
    pub reconciled_champion_id: Option<i32>,
}

impl ChampSelectSessionState {
    pub fn reset_session(&mut self) {
        self.last_payload = None;
        self.last_build = None;
        self.last_applied = None;
        self.reconciled_champion_id = None;
    }

    pub fn record_processed(&mut self, payload: ChampSelectedPayload) {
        let champ_changed = self
            .last_payload
            .as_ref()
            .is_some_and(|prev| prev.champion_id != payload.champion_id);
        if champ_changed {
            self.last_build = None;
            self.last_applied = None;
            self.reconciled_champion_id = None;
        }
        self.last_payload = Some(payload);
    }

    pub fn record_build_and_applied(
        &mut self,
        build: ChampionBuild,
        applied: AppliedInjectionState,
    ) {
        self.last_build = Some(build);
        self.last_applied = Some(applied);
    }
}

static SHARED_SESSION_STATE: OnceLock<Arc<tokio::sync::Mutex<ChampSelectSessionState>>> =
    OnceLock::new();
static RECONCILE_SERIALIZE_LOCK: OnceLock<Arc<tokio::sync::Mutex<()>>> = OnceLock::new();
static SHARED_ACTIVE_QUEUE_ID: OnceLock<Arc<AtomicI32>> = OnceLock::new();
static SHARED_COORDINATOR: OnceLock<ChampSelectCoordinator> = OnceLock::new();
static SHARED_LAST_GAMEFLOW_PHASE: OnceLock<Arc<tokio::sync::Mutex<String>>> = OnceLock::new();
static SHARED_POLL_LOCK: OnceLock<Arc<tokio::sync::Mutex<()>>> = OnceLock::new();

pub fn shared_session_state() -> Arc<tokio::sync::Mutex<ChampSelectSessionState>> {
    Arc::clone(
        SHARED_SESSION_STATE
            .get_or_init(|| Arc::new(tokio::sync::Mutex::new(ChampSelectSessionState::default()))),
    )
}

pub fn shared_reconcile_lock() -> Arc<tokio::sync::Mutex<()>> {
    Arc::clone(RECONCILE_SERIALIZE_LOCK.get_or_init(|| Arc::new(tokio::sync::Mutex::new(()))))
}

pub fn shared_active_queue_id() -> Arc<AtomicI32> {
    Arc::clone(SHARED_ACTIVE_QUEUE_ID.get_or_init(|| Arc::new(AtomicI32::new(0))))
}

pub fn shared_coordinator() -> ChampSelectCoordinator {
    SHARED_COORDINATOR
        .get_or_init(ChampSelectCoordinator::default)
        .clone()
}

pub fn shared_last_gameflow_phase() -> Arc<tokio::sync::Mutex<String>> {
    Arc::clone(
        SHARED_LAST_GAMEFLOW_PHASE.get_or_init(|| Arc::new(tokio::sync::Mutex::new(String::new()))),
    )
}

pub fn shared_poll_lock() -> Arc<tokio::sync::Mutex<()>> {
    Arc::clone(SHARED_POLL_LOCK.get_or_init(|| Arc::new(tokio::sync::Mutex::new(()))))
}

/// Returns `true` when an IPC bootstrap snapshot request should probe LCU on demand
/// because no gameflow phase has been recorded yet or `ChampSelect` has no cached payload yet.
pub fn should_probe_lcu_for_snapshot(phase: &str, state: &ChampSelectSessionState) -> bool {
    let trimmed = phase.trim();
    trimmed.is_empty() || (trimmed == "ChampSelect" && state.last_payload.is_none())
}

/// Builds an `ActiveChampSelectSnapshot` from in-memory session state and gameflow phase,
/// including `last_build` only when compatible with `last_payload` via `build_matches_payload`.
pub fn build_active_champ_select_snapshot(
    phase: &str,
    state: &ChampSelectSessionState,
) -> ActiveChampSelectSnapshot {
    let trimmed = phase.trim();
    let gameflow_phase = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    };
    let champ_selected = state.last_payload.clone();
    let build = champ_selected.as_ref().and_then(|payload| {
        let build_q_id = resolve_effective_build_queue_id(payload);
        state
            .last_build
            .as_ref()
            .filter(|b| build_matches_payload(b, payload, build_q_id))
            .cloned()
    });

    ActiveChampSelectSnapshot {
        gameflow_phase,
        champ_selected,
        build,
    }
}

/// Updates the champion ID currently displayed in the frontend UI.
pub async fn set_ui_displayed_champion(champion_id: Option<i32>) {
    let state = shared_session_state();
    let mut guard = state.lock().await;
    guard.ui_displayed_champion_id = champion_id;
}

/// Updates the cached build and applied injection state when filters change via IPC command.
pub async fn record_filter_updated_build(build: ChampionBuild, applied: AppliedInjectionState) {
    let state = shared_session_state();
    let mut guard = state.lock().await;
    if guard
        .last_payload
        .as_ref()
        .is_some_and(|p| p.champion_id == build.champion_id)
    {
        guard.last_build = Some(build);
        guard.last_applied = Some(applied);
    }
}

pub fn resolve_effective_build_queue_id(payload: &ChampSelectedPayload) -> i32 {
    if payload.queue_id == 0 || payload.queue_id == QUEUE_PRACTICE_TOOL {
        QUEUE_SOLO_DUO
    } else {
        payload.queue_id
    }
}

pub fn resolve_effective_build_position(
    payload: &ChampSelectedPayload,
    build_queue_id: i32,
) -> Option<String> {
    if build_queue_id == QUEUE_ARAM
        || build_queue_id == QUEUE_ARAM_MAYHEM
        || is_arena_queue(build_queue_id)
    {
        None
    } else {
        payload.assigned_position.clone()
    }
}

pub(crate) fn build_matches_payload(
    build: &ChampionBuild,
    actual: &ChampSelectedPayload,
    build_queue_id: i32,
) -> bool {
    build.champion_id == actual.champion_id
        && build.supports_runes == queue_supports_runes(build_queue_id)
        && build.supports_spells == queue_supports_spells(build_queue_id)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconciliationDecision {
    /// Champion differs from backend processed/applied or UI displayed state.
    Diverged {
        clear_previous_selections: bool,
        need_build_fetch: bool,
    },
    /// Same champion across backend and UI, but safety re-injection is required before match load.
    SafetyReinject { need_build_fetch: bool },
    /// Already reconciled for this exact champion and applied state in the current FINALIZATION window.
    AlreadyReconciled,
}

/// Pure decision function comparing the real LCU champion against backend and UI state.
pub fn evaluate_reconciliation_decision(
    state: &ChampSelectSessionState,
    actual: &ChampSelectedPayload,
    prefs: &UserPreferences,
    build_queue_id: i32,
) -> ReconciliationDecision {
    let backend_champ_diverged = state.last_payload.as_ref().is_none_or(|p| {
        p.champion_id != actual.champion_id || resolve_effective_build_queue_id(p) != build_queue_id
    });
    let ui_champ_diverged = state
        .ui_displayed_champion_id
        .is_some_and(|displayed_id| displayed_id != actual.champion_id);

    let has_matching_build = state
        .last_build
        .as_ref()
        .is_some_and(|b| build_matches_payload(b, actual, build_queue_id));

    let applied_matches = match (&state.last_build, &state.last_applied) {
        (Some(b), Some(a)) if build_matches_payload(b, actual, build_queue_id) => {
            a.matches_expected(actual.champion_id, build_queue_id, b, prefs)
        }
        _ => false,
    };

    if backend_champ_diverged || ui_champ_diverged {
        let clear_previous_selections = state
            .last_payload
            .as_ref()
            .is_some_and(|prev| prev.champion_id != actual.champion_id);
        return ReconciliationDecision::Diverged {
            clear_previous_selections,
            need_build_fetch: !has_matching_build,
        };
    }

    if state.reconciled_champion_id == Some(actual.champion_id) && applied_matches {
        return ReconciliationDecision::AlreadyReconciled;
    }

    ReconciliationDecision::SafetyReinject {
        need_build_fetch: !has_matching_build,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{QUEUE_ARENA, QueueMode};

    #[test]
    fn test_arena_reconciliation_recognizes_item_only_injection() {
        let payload = ChampSelectedPayload {
            champion_id: 350,
            queue_id: QUEUE_ARENA,
            queue_mode: QueueMode::Arena,
            locked_in: true,
            assigned_position: Some("support".to_string()),
        };
        assert_eq!(resolve_effective_build_queue_id(&payload), QUEUE_ARENA);
        assert_eq!(
            resolve_effective_build_position(&payload, QUEUE_ARENA),
            None
        );

        let arena_build = ChampionBuild {
            champion_id: 350,
            champion_name: "Yuumi".to_string(),
            supports_runes: false,
            supports_spells: false,
            ..Default::default()
        };
        assert!(build_matches_payload(&arena_build, &payload, QUEUE_ARENA));

        let mut wrong_flags_build = arena_build.clone();
        wrong_flags_build.supports_spells = true;
        assert!(!build_matches_payload(
            &wrong_flags_build,
            &payload,
            QUEUE_ARENA
        ));

        let state = ChampSelectSessionState {
            last_payload: Some(payload.clone()),
            last_build: Some(arena_build),
            last_applied: Some(AppliedInjectionState {
                champion_id: 350,
                queue_id: QUEUE_ARENA,
                supports_runes: false,
                supports_spells: false,
                runes_injected: false,
                spells_injected: false,
                items_injected: true,
            }),
            ui_displayed_champion_id: Some(350),
            reconciled_champion_id: Some(350),
        };
        let prefs = UserPreferences::default();
        assert_eq!(
            evaluate_reconciliation_decision(&state, &payload, &prefs, QUEUE_ARENA),
            ReconciliationDecision::AlreadyReconciled
        );
    }
}

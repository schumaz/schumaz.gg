use super::*;
use crate::models::{ChampSelectedPayload, QUEUE_SOLO_DUO, QueueMode};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct MockFilterGateway {
    delay_ms: u64,
    bump_coordinator: Option<(super::super::coordinator::ChampSelectCoordinator, u64)>,
    clear_session_on_fetch: Option<Arc<tokio::sync::Mutex<ChampSelectSessionState>>>,
    cleared: AtomicUsize,
    emitted_builds: tokio::sync::Mutex<Vec<i32>>,
    standard_injects: AtomicUsize,
    lobby_injects: AtomicUsize,
    emitted_reports: AtomicUsize,
}

impl MockFilterGateway {
    fn new(delay_ms: u64) -> Self {
        Self {
            delay_ms,
            bump_coordinator: None,
            clear_session_on_fetch: None,
            cleared: AtomicUsize::new(0),
            emitted_builds: tokio::sync::Mutex::new(Vec::new()),
            standard_injects: AtomicUsize::new(0),
            lobby_injects: AtomicUsize::new(0),
            emitted_reports: AtomicUsize::new(0),
        }
    }
}

impl FilterReinjectGateway for MockFilterGateway {
    async fn fetch_build(
        &self,
        champion_id: i32,
        _queue_id: i32,
        _effective_position: Option<&str>,
        _tier: Option<&str>,
        _region: Option<&str>,
        _patch: Option<&str>,
    ) -> Result<ChampionBuild, String> {
        if self.delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
        }
        if let Some((coord, _)) = &self.bump_coordinator {
            coord.cancel_pending().await;
        }
        if let Some(state_ref) = &self.clear_session_on_fetch {
            state_ref.lock().await.reset_session();
        }
        Ok(ChampionBuild {
            champion_id,
            champion_name: "Ahri".to_string(),
            ..Default::default()
        })
    }

    fn clear_champion_selection(&self, _champion_id: i32) {
        self.cleared.fetch_add(1, Ordering::SeqCst);
    }

    fn emit_recommended_build(&self, build: ChampionBuild) {
        if let Ok(mut guard) = self.emitted_builds.try_lock() {
            guard.push(build.champion_id);
        }
    }

    async fn reinject_lobby_filter_update(&self, _champion_id: i32, _build: &ChampionBuild) -> bool {
        self.lobby_injects.fetch_add(1, Ordering::SeqCst);
        true
    }

    async fn inject_standard_build(
        &self,
        _queue_id: i32,
        build: &ChampionBuild,
    ) -> Option<(InjectionReport, AppliedInjectionState)> {
        self.standard_injects.fetch_add(1, Ordering::SeqCst);
        let report = InjectionReport {
            champion_name: build.champion_name.clone(),
            runes_injected: true,
            spells_injected: true,
            items_injected: true,
            message: "OK".to_string(),
        };
        let applied = AppliedInjectionState::from_report(QUEUE_SOLO_DUO, build, &report);
        Some((report, applied))
    }

    fn emit_build_injected(&self, _report: InjectionReport) {
        self.emitted_reports.fetch_add(1, Ordering::SeqCst);
    }
}

fn active_ahri_state() -> Arc<tokio::sync::Mutex<ChampSelectSessionState>> {
    Arc::new(tokio::sync::Mutex::new(ChampSelectSessionState {
        last_payload: Some(ChampSelectedPayload {
            champion_id: 103,
            queue_id: QUEUE_SOLO_DUO,
            queue_mode: QueueMode::SummonersRift,
            locked_in: true,
            assigned_position: Some("mid".to_string()),
            ..Default::default()
        }),
        last_build: Some(ChampionBuild {
            champion_id: 103,
            champion_name: "Ahri".to_string(),
            active_position: "mid".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    }))
}

#[tokio::test]
async fn test_filter_reinject_skips_superseded_and_ended_sessions() {
    let coordinator = super::super::coordinator::ChampSelectCoordinator::default();
    let phase_ref = Arc::new(tokio::sync::Mutex::new("ChampSelect".to_string()));
    let state_ref = active_ahri_state();

    let mut gw_superseded = MockFilterGateway::new(10);
    gw_superseded.bump_coordinator = Some((coordinator.clone(), 1));

    let res = run_filter_reinject_with_gateway(
        &gw_superseded,
        103,
        Some(QUEUE_SOLO_DUO),
        Some("mid"),
        Some("diamond_plus"),
        None,
        None,
        &state_ref,
        &phase_ref,
        &coordinator,
    )
    .await
    .unwrap();
    assert_eq!(res.champion_id, 103);
    assert_eq!(gw_superseded.cleared.load(Ordering::SeqCst), 1);
    assert!(gw_superseded.emitted_builds.lock().await.is_empty());
    assert_eq!(gw_superseded.standard_injects.load(Ordering::SeqCst), 0);

    // Dodged/ended session while fetching -> must NOT emit or inject
    let mut gw_ended = MockFilterGateway::new(10);
    gw_ended.clear_session_on_fetch = Some(Arc::clone(&state_ref));
    let _ = run_filter_reinject_with_gateway(
        &gw_ended,
        103,
        Some(QUEUE_SOLO_DUO),
        Some("mid"),
        Some("master_plus"),
        None,
        None,
        &state_ref,
        &phase_ref,
        &coordinator,
    )
    .await
    .unwrap();
    assert!(gw_ended.emitted_builds.lock().await.is_empty());
    assert_eq!(gw_ended.standard_injects.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn test_filter_reinject_preserves_hover_selections_on_lock_and_injects_active_session() {
    let coordinator = super::super::coordinator::ChampSelectCoordinator::default();
    let phase_ref = Arc::new(tokio::sync::Mutex::new("ChampSelect".to_string()));
    let state_ref = active_ahri_state();
    let gw = MockFilterGateway::new(0);

    // Same position with no explicit tier/region/patch (e.g. draft lock-in fetch) preserves selections
    let res = run_filter_reinject_with_gateway(
        &gw,
        103,
        Some(QUEUE_SOLO_DUO),
        Some("mid"),
        None,
        None,
        None,
        &state_ref,
        &phase_ref,
        &coordinator,
    )
    .await
    .unwrap();
    assert_eq!(res.champion_id, 103);
    assert_eq!(gw.cleared.load(Ordering::SeqCst), 0);
    assert_eq!(gw.emitted_builds.lock().await.as_slice(), &[103]);
    assert_eq!(gw.standard_injects.load(Ordering::SeqCst), 1);
    assert_eq!(gw.emitted_reports.load(Ordering::SeqCst), 1);

    // InProgress phase prevents injection even if payload exists
    *phase_ref.lock().await = "InProgress".to_string();
    let gw_ingame = MockFilterGateway::new(0);
    let _ = run_filter_reinject_with_gateway(
        &gw_ingame,
        103,
        Some(QUEUE_SOLO_DUO),
        Some("top"),
        Some("emerald_plus"),
        None,
        None,
        &state_ref,
        &phase_ref,
        &coordinator,
    )
    .await
    .unwrap();
    assert_eq!(gw_ingame.standard_injects.load(Ordering::SeqCst), 0);
}

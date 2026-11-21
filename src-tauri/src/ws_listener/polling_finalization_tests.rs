use super::*;
use crate::lcu_injector::{AppliedInjectionState, InjectionReport, UserPreferences};
use crate::models::{ChampSelectReconciledPayload, ChampSelectedPayload, QUEUE_ARAM};
use crate::ws_listener::reconcile::{
    FinalReconcileGateway, FinalReconcileOutcome, ReconcileEventEmitter, reconcile_final_session,
};

struct SingleFetchReconcileGateway {
    session: serde_json::Value,
    queue_id: i32,
    build: ChampionBuild,
    fetches: AtomicUsize,
    reinjects: AtomicUsize,
}

impl FinalReconcileGateway for SingleFetchReconcileGateway {
    async fn get_champ_select_session(&self) -> Result<Option<serde_json::Value>, String> {
        Ok(Some(self.session.clone()))
    }
    async fn get_current_queue_id(&self) -> Result<i32, String> {
        Ok(self.queue_id)
    }
    async fn get_summoner_id(&self) -> Result<u64, String> {
        Ok(99)
    }
    async fn fetch_build(&self, _: i32, _: i32, _: Option<&str>) -> Result<ChampionBuild, String> {
        self.fetches.fetch_add(1, Ordering::SeqCst);
        Ok(self.build.clone())
    }
    async fn safety_reinject(
        &self,
        _: u64,
        queue_id: i32,
        build: &ChampionBuild,
    ) -> (InjectionReport, AppliedInjectionState) {
        self.reinjects.fetch_add(1, Ordering::SeqCst);
        let report = InjectionReport {
            champion_name: build.champion_name.clone(),
            runes_injected: build.supports_runes,
            spells_injected: build.supports_spells,
            items_injected: true,
            message: "OK".to_string(),
        };
        let state = AppliedInjectionState::from_report(queue_id, build, &report);
        (report, state)
    }
    fn clear_selections(&self) {}
    async fn cancel_pending_tasks(&self) {}
}

#[derive(Default)]
struct RecordingReconcileEmitter {
    events: tokio::sync::Mutex<Vec<String>>,
}

impl RecordingReconcileEmitter {
    fn push(&self, entry: String) {
        if let Ok(mut guard) = self.events.try_lock() {
            guard.push(entry);
        }
    }
}

impl ReconcileEventEmitter for RecordingReconcileEmitter {
    fn emit_champ_selected(&self, payload: ChampSelectedPayload) {
        self.push(format!("champ-selected:{}", payload.champion_id));
    }
    fn emit_recommended_build(&self, build: ChampionBuild) {
        self.push(format!("recommended-build:{}", build.champion_id));
    }
    fn emit_champ_select_reconciled(&self, payload: ChampSelectReconciledPayload) {
        let id = payload.champ_selected.champion_id;
        self.push(format!("reconciled:{}:{}", id, payload.champion_diverged));
    }
    fn emit_build_injected(&self, report: InjectionReport) {
        self.push(format!("injected:{}", report.champion_name));
    }
    fn emit_champ_select_ended(&self) {}
    fn request_lock_in_attention(&self) {}
}

#[tokio::test]
async fn test_opening_in_last_seconds_of_finalization_coalesces_to_single_fetch_and_inject() {
    let session = make_session(222, None, 2100);
    let h = PollHarness::new(session.clone(), QUEUE_ARAM);
    h.poll_once().await;
    assert_eq!(h.counts(), (1, 1, 0, 1));

    let jinx_build = make_build(222, "Jinx", true, true);
    let gw = SingleFetchReconcileGateway {
        session,
        queue_id: QUEUE_ARAM,
        build: jinx_build.clone(),
        fetches: AtomicUsize::new(0),
        reinjects: AtomicUsize::new(0),
    };
    let emitter = RecordingReconcileEmitter::default();
    let lock = Arc::new(tokio::sync::Mutex::new(()));
    let prefs = UserPreferences::default();

    let first = reconcile_final_session(&gw, &emitter, &h.session, &lock, &h.queue_id, &prefs)
        .await
        .unwrap();
    assert_eq!(
        first,
        FinalReconcileOutcome::Reconciled {
            champion_id: 222,
            champion_diverged: false,
            cleared_selections: false,
        }
    );
    assert_eq!(gw.fetches.load(Ordering::SeqCst), 1);
    assert_eq!(gw.reinjects.load(Ordering::SeqCst), 1);
    assert_eq!(
        emitter.events.lock().await.as_slice(),
        &[
            "recommended-build:222",
            "reconciled:222:false",
            "injected:Jinx"
        ]
    );
    assert_eq!(h.snapshot().await.build, Some(jinx_build));

    let second = reconcile_final_session(&gw, &emitter, &h.session, &lock, &h.queue_id, &prefs)
        .await
        .unwrap();
    assert_eq!(second, FinalReconcileOutcome::AlreadyReconciled);
    assert_eq!(gw.fetches.load(Ordering::SeqCst), 1);
    assert_eq!(gw.reinjects.load(Ordering::SeqCst), 1);
}

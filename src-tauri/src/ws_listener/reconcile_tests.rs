use super::*;
use std::sync::atomic::AtomicUsize;

struct MockGateway {
    session: serde_json::Value,
    queue_id: i32,
    summoner_id: u64,
    build: ChampionBuild,
    cleared_count: AtomicUsize,
    cancelled_count: AtomicUsize,
    reinject_calls: tokio::sync::Mutex<Vec<(i32, i32, bool)>>,
}

impl FinalReconcileGateway for MockGateway {
    async fn get_champ_select_session(&self) -> Result<Option<serde_json::Value>, String> {
        Ok(Some(self.session.clone()))
    }

    async fn get_current_queue_id(&self) -> Result<i32, String> {
        Ok(self.queue_id)
    }

    async fn get_summoner_id(&self) -> Result<u64, String> {
        Ok(self.summoner_id)
    }

    async fn fetch_build(
        &self,
        _champion_id: i32,
        _queue_id: i32,
        _assigned_position: Option<&str>,
    ) -> Result<ChampionBuild, String> {
        Ok(self.build.clone())
    }

    async fn safety_reinject(
        &self,
        _summoner_id: u64,
        queue_id: i32,
        build: &ChampionBuild,
    ) -> (InjectionReport, AppliedInjectionState) {
        self.reinject_calls
            .lock()
            .await
            .push((build.champion_id, queue_id, build.supports_runes));
        let report = InjectionReport {
            champion_name: build.champion_name.clone(),
            runes_injected: build.supports_runes,
            spells_injected: true,
            items_injected: true,
            message: "OK".to_string(),
        };
        let applied = AppliedInjectionState::from_report(queue_id, build, &report);
        (report, applied)
    }

    fn clear_selections(&self) {
        self.cleared_count.fetch_add(1, Ordering::SeqCst);
    }

    async fn cancel_pending_tasks(&self) {
        self.cancelled_count.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Default)]
struct MockEmitter {
    events: tokio::sync::Mutex<Vec<String>>,
    reconciled_payloads: tokio::sync::Mutex<Vec<ChampSelectReconciledPayload>>,
    attention_requests: AtomicUsize,
}

impl ReconcileEventEmitter for MockEmitter {
    fn emit_champ_selected(&self, payload: ChampSelectedPayload) {
        if let Ok(mut guard) = self.events.try_lock() {
            guard.push(format!("champ-selected:{}", payload.champion_id));
        }
    }

    fn emit_recommended_build(&self, build: ChampionBuild) {
        if let Ok(mut guard) = self.events.try_lock() {
            guard.push(format!("recommended-build:{}", build.champion_id));
        }
    }

    fn emit_champ_select_reconciled(&self, payload: ChampSelectReconciledPayload) {
        if let Ok(mut guard) = self.events.try_lock() {
            guard.push(format!(
                "champ-select-reconciled:{}:{}",
                payload.champ_selected.champion_id, payload.champion_diverged
            ));
        }
        if let Ok(mut guard) = self.reconciled_payloads.try_lock() {
            guard.push(payload);
        }
    }

    fn emit_build_injected(&self, report: InjectionReport) {
        if let Ok(mut guard) = self.events.try_lock() {
            guard.push(format!("build-injected:{}", report.champion_name));
        }
    }

    fn emit_champ_select_ended(&self) {
        if let Ok(mut guard) = self.events.try_lock() {
            guard.push("champ-select-ended".to_string());
        }
    }

    fn request_lock_in_attention(&self) {
        self.attention_requests.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn test_reconcile_detects_champion_divergence_after_rapid_aram_bench_swap() {
    let session = serde_json::json!({
        "localPlayerCellId": 0,
        "actions": [],
        "myTeam": [{ "cellId": 0, "championId": 222 }],
        "timer": {
            "phase": "FINALIZATION",
            "adjustedTimeLeftInPhase": 2200,
            "isInfinite": false
        }
    });

    let jinx_build = ChampionBuild {
        champion_id: 222,
        champion_name: "Jinx".to_string(),
        supports_runes: true,
        ..Default::default()
    };

    let gateway = MockGateway {
        session,
        queue_id: QUEUE_ARAM,
        summoner_id: 42,
        build: jinx_build,
        cleared_count: AtomicUsize::new(0),
        cancelled_count: AtomicUsize::new(0),
        reinject_calls: tokio::sync::Mutex::new(Vec::new()),
    };
    let emitter = MockEmitter::default();

    // Backend and UI were still stuck on Tristana (18) before FINALIZATION ended
    let state = Arc::new(tokio::sync::Mutex::new(ChampSelectSessionState {
        last_payload: Some(ChampSelectedPayload {
            champion_id: 18,
            queue_id: QUEUE_ARAM,
            queue_mode: QueueMode::Aram,
            locked_in: true,
            assigned_position: None,
        }),
        ui_displayed_champion_id: Some(18),
        ..Default::default()
    }));
    let lock = Arc::new(tokio::sync::Mutex::new(()));
    let q_ref = Arc::new(AtomicI32::new(QUEUE_ARAM));
    let prefs = UserPreferences::default();

    let outcome = reconcile_final_session(&gateway, &emitter, &state, &lock, &q_ref, &prefs)
        .await
        .unwrap();

    assert_eq!(
        outcome,
        FinalReconcileOutcome::Reconciled {
            champion_id: 222,
            champion_diverged: true,
            cleared_selections: true,
        }
    );
    assert_eq!(gateway.cleared_count.load(Ordering::SeqCst), 1);
    assert_eq!(gateway.cancelled_count.load(Ordering::SeqCst), 1);
    assert_eq!(emitter.attention_requests.load(Ordering::SeqCst), 1);
    assert_eq!(
        gateway.reinject_calls.lock().await.as_slice(),
        &[(222, QUEUE_ARAM, true)]
    );

    let emitted = emitter.events.lock().await.clone();
    assert_eq!(
        emitted,
        vec![
            "champ-selected:222",
            "recommended-build:222",
            "champ-select-reconciled:222:true",
            "build-injected:Jinx"
        ]
    );
}

#[tokio::test]
async fn test_reconcile_same_champion_preserves_selections_and_respects_aram_mayhem_mode() {
    let session = serde_json::json!({
        "localPlayerCellId": 0,
        "actions": [],
        "myTeam": [{ "cellId": 0, "championId": 103 }],
        "timer": {
            "phase": "FINALIZATION",
            "adjustedTimeLeftInPhase": 1800,
            "isInfinite": false
        }
    });

    let ahri_mayhem_build = ChampionBuild {
        champion_id: 103,
        champion_name: "Ahri".to_string(),
        supports_runes: false,
        ..Default::default()
    };

    let gateway = MockGateway {
        session,
        queue_id: QUEUE_ARAM_MAYHEM,
        summoner_id: 42,
        build: ahri_mayhem_build.clone(),
        cleared_count: AtomicUsize::new(0),
        cancelled_count: AtomicUsize::new(0),
        reinject_calls: tokio::sync::Mutex::new(Vec::new()),
    };
    let emitter = MockEmitter::default();

    let state = Arc::new(tokio::sync::Mutex::new(ChampSelectSessionState {
        last_payload: Some(ChampSelectedPayload {
            champion_id: 103,
            queue_id: QUEUE_ARAM_MAYHEM,
            queue_mode: QueueMode::AramMayhem,
            locked_in: true,
            assigned_position: None,
        }),
        last_build: Some(ahri_mayhem_build.clone()),
        last_applied: Some(AppliedInjectionState {
            champion_id: 103,
            queue_id: QUEUE_ARAM_MAYHEM,
            supports_runes: false,
            runes_injected: false,
            spells_injected: true,
            items_injected: true,
        }),
        ui_displayed_champion_id: Some(103),
        reconciled_champion_id: None,
    }));
    let lock = Arc::new(tokio::sync::Mutex::new(()));
    let q_ref = Arc::new(AtomicI32::new(QUEUE_ARAM_MAYHEM));
    let prefs = UserPreferences::default();

    let first = reconcile_final_session(&gateway, &emitter, &state, &lock, &q_ref, &prefs)
        .await
        .unwrap();
    assert_eq!(
        first,
        FinalReconcileOutcome::Reconciled {
            champion_id: 103,
            champion_diverged: false,
            cleared_selections: false,
        }
    );
    // Must preserve manual selections when champion did not diverge and must not re-flash taskbar
    assert_eq!(gateway.cleared_count.load(Ordering::SeqCst), 0);
    assert_eq!(emitter.attention_requests.load(Ordering::SeqCst), 0);
    assert_eq!(
        gateway.reinject_calls.lock().await.as_slice(),
        &[(103, QUEUE_ARAM_MAYHEM, false)]
    );

    // Second trigger within the same FINALIZATION window must be a no-op
    let second = reconcile_final_session(&gateway, &emitter, &state, &lock, &q_ref, &prefs)
        .await
        .unwrap();
    assert_eq!(second, FinalReconcileOutcome::AlreadyReconciled);
    assert_eq!(gateway.reinject_calls.lock().await.len(), 1);
    assert_eq!(emitter.attention_requests.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn test_reconcile_detects_ui_only_divergence_and_resyncs_without_clearing_selections() {
    let session = serde_json::json!({
        "localPlayerCellId": 1,
        "actions": [[{ "actorCellId": 1, "type": "pick", "championId": 157, "completed": true }]],
        "myTeam": [{ "cellId": 1, "championId": 157, "assignedPosition": "middle" }],
        "timer": {
            "phase": "FINALIZATION",
            "adjustedTimeLeftInPhase": 2900,
            "isInfinite": false
        }
    });

    let yasuo_build = ChampionBuild {
        champion_id: 157,
        champion_name: "Yasuo".to_string(),
        supports_runes: true,
        ..Default::default()
    };

    let gateway = MockGateway {
        session,
        queue_id: QUEUE_SOLO_DUO,
        summoner_id: 77,
        build: yasuo_build.clone(),
        cleared_count: AtomicUsize::new(0),
        cancelled_count: AtomicUsize::new(0),
        reinject_calls: tokio::sync::Mutex::new(Vec::new()),
    };
    let emitter = MockEmitter::default();

    // Backend processed Yasuo (157), but UI is still showing Ahri (103)
    let state = Arc::new(tokio::sync::Mutex::new(ChampSelectSessionState {
        last_payload: Some(ChampSelectedPayload {
            champion_id: 157,
            queue_id: QUEUE_SOLO_DUO,
            queue_mode: QueueMode::SummonersRift,
            locked_in: true,
            assigned_position: Some("mid".to_string()),
        }),
        last_build: Some(yasuo_build),
        ui_displayed_champion_id: Some(103),
        ..Default::default()
    }));
    let lock = Arc::new(tokio::sync::Mutex::new(()));
    let q_ref = Arc::new(AtomicI32::new(QUEUE_SOLO_DUO));
    let prefs = UserPreferences::default();

    let outcome = reconcile_final_session(&gateway, &emitter, &state, &lock, &q_ref, &prefs)
        .await
        .unwrap();

    assert_eq!(
        outcome,
        FinalReconcileOutcome::Reconciled {
            champion_id: 157,
            champion_diverged: true,
            cleared_selections: false,
        }
    );
    // Selections belong to Yasuo (backend already moved to Yasuo), so don't wipe them or re-flash taskbar
    assert_eq!(gateway.cleared_count.load(Ordering::SeqCst), 0);
    assert_eq!(emitter.attention_requests.load(Ordering::SeqCst), 0);
    assert_eq!(
        gateway.reinject_calls.lock().await.as_slice(),
        &[(157, QUEUE_SOLO_DUO, true)]
    );
}

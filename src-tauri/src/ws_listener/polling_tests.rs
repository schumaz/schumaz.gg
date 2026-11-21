use super::*;
use crate::build_provider::ChampionBuild;
use crate::models::{
    ChampSelectedPayload, QUEUE_ARAM, QUEUE_ARAM_MAYHEM, QUEUE_ARENA, QUEUE_SOLO_DUO, QueueMode,
};
use std::sync::atomic::AtomicUsize;

#[path = "polling_finalization_tests.rs"]
mod finalization_tests;

struct MockPollGateway {
    session: Option<serde_json::Value>,
    queue_id: i32,
    session_calls: AtomicUsize,
    queue_calls: AtomicUsize,
}

impl LcuPollGateway for MockPollGateway {
    async fn get_gameflow_phase(&self) -> Result<String, String> {
        Ok("ChampSelect".to_string())
    }
    async fn get_champ_select_session(&self) -> Result<Option<serde_json::Value>, String> {
        self.session_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.session.clone())
    }
    async fn get_current_queue_id(&self) -> Result<i32, String> {
        self.queue_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.queue_id)
    }
}

struct RecordingPollSink {
    phase: Arc<tokio::sync::Mutex<String>>,
    session: Arc<tokio::sync::Mutex<ChampSelectSessionState>>,
    spawns: AtomicUsize,
    deferred: AtomicUsize,
}

impl LcuPollSink for RecordingPollSink {
    async fn on_gameflow_phase(&self, phase: String) {
        *self.phase.lock().await = phase;
    }
    async fn on_session_update(
        &self,
        transition: WsSessionTransition,
        _: &serde_json::Value,
        defer_initial_inject: bool,
    ) {
        if let WsSessionTransition::Active(payload) = transition {
            let spawn = !defer_initial_inject
                && (payload.locked_in || payload.queue_mode == QueueMode::PracticeTool);
            self.session.lock().await.record_processed(payload);
            if spawn {
                self.spawns.fetch_add(1, Ordering::SeqCst);
            } else if defer_initial_inject {
                self.deferred.fetch_add(1, Ordering::SeqCst);
            }
        }
    }
    async fn on_session_ended(&self) {
        self.session.lock().await.reset_session();
    }
}

struct PollHarness {
    gw: MockPollGateway,
    sink: RecordingPollSink,
    phase: Arc<tokio::sync::Mutex<String>>,
    session: Arc<tokio::sync::Mutex<ChampSelectSessionState>>,
    queue_id: Arc<AtomicI32>,
}

impl PollHarness {
    fn new(session_json: serde_json::Value, queue_id: i32) -> Self {
        let phase = Arc::new(tokio::sync::Mutex::new(String::new()));
        let session = Arc::new(tokio::sync::Mutex::new(ChampSelectSessionState::default()));
        Self {
            gw: MockPollGateway {
                session: Some(session_json),
                queue_id,
                session_calls: AtomicUsize::new(0),
                queue_calls: AtomicUsize::new(0),
            },
            sink: RecordingPollSink {
                phase: Arc::clone(&phase),
                session: Arc::clone(&session),
                spawns: AtomicUsize::new(0),
                deferred: AtomicUsize::new(0),
            },
            phase,
            session,
            queue_id: Arc::new(AtomicI32::new(0)),
        }
    }

    async fn poll_once(&self) {
        poll_lcu_session_with_gateway(
            &self.gw,
            &self.sink,
            &self.phase,
            &self.session,
            &self.queue_id,
        )
        .await;
    }

    fn counts(&self) -> (usize, usize, usize, usize) {
        (
            self.gw.session_calls.load(Ordering::SeqCst),
            self.gw.queue_calls.load(Ordering::SeqCst),
            self.sink.spawns.load(Ordering::SeqCst),
            self.sink.deferred.load(Ordering::SeqCst),
        )
    }

    async fn snapshot(&self) -> ActiveChampSelectSnapshot {
        let phase = self.phase.lock().await.clone();
        let state = self.session.lock().await;
        build_active_champ_select_snapshot(&phase, &state)
    }
}

fn make_build(id: i32, name: &str, runes: bool, spells: bool) -> ChampionBuild {
    ChampionBuild {
        champion_id: id,
        champion_name: name.to_string(),
        supports_runes: runes,
        supports_spells: spells,
        ..Default::default()
    }
}

fn make_session(champ_id: i32, pos: Option<&str>, time_left_ms: i64) -> serde_json::Value {
    let mut member = serde_json::json!({ "cellId": 0, "championId": champ_id });
    if let Some(p) = pos {
        member["assignedPosition"] = serde_json::json!(p);
    }
    serde_json::json!({
        "localPlayerCellId": 0,
        "actions": [],
        "myTeam": [member],
        "timer": {
            "phase": "FINALIZATION",
            "adjustedTimeLeftInPhase": time_left_ms,
            "isInfinite": false
        }
    })
}

#[test]
fn test_resolve_fallback_poll_interval_and_immediate_connect_skip() {
    let fast = Duration::from_millis(1500);
    let slow = Duration::from_millis(4000);
    assert_eq!(resolve_next_poll_delay(true, "", false), None);
    assert_eq!(resolve_next_poll_delay(true, "Lobby", false), None);
    assert_eq!(
        resolve_next_poll_delay(false, "ChampSelect", false),
        Some(fast)
    );
    assert_eq!(resolve_fallback_poll_interval("Lobby", true), fast);
    assert_eq!(resolve_fallback_poll_interval("Lobby", false), slow);
    assert_eq!(resolve_fallback_poll_interval("InProgress", false), slow);
}

#[test]
fn test_should_poll_champ_select_session_and_snapshot_probe_rules() {
    assert!(should_poll_champ_select_session("ChampSelect", false));
    assert!(should_poll_champ_select_session("Lobby", true));
    assert!(!should_poll_champ_select_session("Lobby", false));

    let empty = ChampSelectSessionState::default();
    assert!(should_probe_lcu_for_snapshot("", &empty));
    assert!(should_probe_lcu_for_snapshot("ChampSelect", &empty));
    assert!(!should_probe_lcu_for_snapshot("Lobby", &empty));

    let cached = ChampSelectSessionState {
        last_payload: Some(ChampSelectedPayload {
            champion_id: 103,
            queue_id: QUEUE_SOLO_DUO,
            queue_mode: QueueMode::SummonersRift,
            locked_in: true,
            assigned_position: Some("mid".to_string()),
        }),
        ..Default::default()
    };
    assert!(!should_probe_lcu_for_snapshot("ChampSelect", &cached));
}

#[tokio::test]
async fn test_mid_champ_select_bootstrap_poll_across_sr_aram_mayhem_and_arena() {
    let cases = [
        (
            QUEUE_SOLO_DUO,
            QueueMode::SummonersRift,
            Some("middle"),
            Some("mid"),
            true,
            true,
        ),
        (QUEUE_ARAM, QueueMode::Aram, None, None, true, true),
        (
            QUEUE_ARAM_MAYHEM,
            QueueMode::AramMayhem,
            None,
            None,
            false,
            true,
        ),
        (QUEUE_ARENA, QueueMode::Arena, None, None, false, false),
    ];

    for (queue_id, mode, raw_pos, expected_pos, runes, spells) in cases {
        let h = PollHarness::new(make_session(103, raw_pos, 18000), queue_id);
        h.poll_once().await;

        assert_eq!(*h.phase.lock().await, "ChampSelect");
        assert_eq!(h.counts(), (1, 1, 1, 0));
        assert_eq!(h.queue_id.load(Ordering::SeqCst), queue_id);
        assert_eq!(
            resolve_fallback_poll_interval(&h.phase.lock().await, true),
            Duration::from_millis(1500)
        );

        let compatible = make_build(103, "Ahri", runes, spells);
        h.session.lock().await.last_build = Some(compatible.clone());

        let snap = h.snapshot().await;
        let sel = snap.champ_selected.expect("payload should be hydrated");
        assert_eq!(snap.gameflow_phase.as_deref(), Some("ChampSelect"));
        assert_eq!(
            (sel.champion_id, sel.queue_id, sel.queue_mode),
            (103, queue_id, mode)
        );
        assert_eq!(sel.assigned_position.as_deref(), expected_pos);
        assert_eq!(snap.build, Some(compatible.clone()));

        let mut incompatible = compatible;
        incompatible.supports_runes = !runes;
        h.session.lock().await.last_build = Some(incompatible);
        assert!(h.snapshot().await.build.is_none());
    }
}

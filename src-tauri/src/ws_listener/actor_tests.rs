use super::*;
use crate::lcu_injector::{
    TEST_SELECTIONS_LOCK, clear_champion_selections, get_champion_selections,
    set_selected_spell_pair,
};
use crate::models::QUEUE_SOLO_DUO;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

struct MockActorDriver {
    queue_delay_ms: u64,
    queue_id: i32,
    active_queue_id: Arc<AtomicI32>,
    last_gameflow_phase: Arc<tokio::sync::Mutex<String>>,
    session_state: Arc<tokio::sync::Mutex<ChampSelectSessionState>>,
    events_order: tokio::sync::Mutex<Vec<String>>,
    poll_calls: AtomicUsize,
}

impl MockActorDriver {
    fn new(queue_delay_ms: u64, queue_id: i32) -> Self {
        Self {
            queue_delay_ms,
            queue_id,
            active_queue_id: Arc::new(AtomicI32::new(0)),
            last_gameflow_phase: Arc::new(tokio::sync::Mutex::new(String::new())),
            session_state: Arc::new(tokio::sync::Mutex::new(ChampSelectSessionState::default())),
            events_order: tokio::sync::Mutex::new(Vec::new()),
            poll_calls: AtomicUsize::new(0),
        }
    }
}

impl WsActorDriver for MockActorDriver {
    async fn fetch_current_queue_id(&self) -> Option<i32> {
        if self.queue_delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.queue_delay_ms)).await;
        }
        Some(self.queue_id)
    }

    async fn on_champ_select_event(
        &self,
        event_type: &str,
        data: &serde_json::Value,
        current_q: i32,
    ) {
        match evaluate_ws_event(event_type, data, current_q) {
            WsSessionTransition::Active(payload) => {
                self.events_order
                    .lock()
                    .await
                    .push(format!("active:{}:{}", payload.champion_id, payload.queue_id));
                self.session_state.lock().await.record_processed(payload);
            }
            WsSessionTransition::Ended => {
                self.events_order.lock().await.push("ended".to_string());
                self.session_state.lock().await.reset_session();
                self.active_queue_id.store(0, Ordering::Relaxed);
                clear_champion_selections();
            }
            WsSessionTransition::NoChampionSelected => {
                self.events_order.lock().await.push("none".to_string());
            }
        }
    }

    async fn on_gameflow_phase(&self, phase: String) {
        self.events_order
            .lock()
            .await
            .push(format!("phase:{phase}"));
        *self.last_gameflow_phase.lock().await = phase;
    }

    async fn on_lobby_event(&self, event_type: &str, _data: &serde_json::Value) {
        self.events_order
            .lock()
            .await
            .push(format!("lobby:{event_type}"));
    }

    async fn on_fallback_poll(&self) {
        self.poll_calls.fetch_add(1, Ordering::SeqCst);
        self.events_order.lock().await.push("poll".to_string());
    }

    async fn on_lcu_disconnected(&self) {
        self.events_order
            .lock()
            .await
            .push("disconnected".to_string());
        self.session_state.lock().await.reset_session();
        self.active_queue_id.store(0, Ordering::Relaxed);
        self.last_gameflow_phase.lock().await.clear();
        clear_champion_selections();
    }
}

#[tokio::test]
async fn test_fifo_actor_prevents_slow_update_from_resurrecting_deleted_session() {
    let _sel_guard = TEST_SELECTIONS_LOCK.lock().await;
    clear_champion_selections();
    set_selected_spell_pair(103, 4, 12);

    let driver = Arc::new(MockActorDriver::new(60, QUEUE_SOLO_DUO));
    let poll_lock = Arc::new(tokio::sync::Mutex::new(()));
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    let driver_task = Arc::clone(&driver);
    let q_ref = Arc::clone(&driver.active_queue_id);
    let p_ref = Arc::clone(&driver.last_gameflow_phase);
    let s_ref = Arc::clone(&driver.session_state);
    let l_ref = Arc::clone(&poll_lock);

    let handle = tokio::spawn(async move {
        run_lcu_event_actor_with_driver(driver_task.as_ref(), &q_ref, &p_ref, &s_ref, &l_ref, rx)
            .await;
    });

    let update_payload = serde_json::json!({
        "localPlayerCellId": 0,
        "actions": [],
        "myTeam": [{ "cellId": 0, "championId": 103 }]
    });

    tx.send(LcuEventActorMsg::WsChampSelect {
        event_type: "Update".to_string(),
        data: update_payload,
    })
    .unwrap();
    tx.send(LcuEventActorMsg::WsChampSelect {
        event_type: "Delete".to_string(),
        data: serde_json::Value::Null,
    })
    .unwrap();

    let (done_tx, done_rx) = oneshot::channel();
    tx.send(LcuEventActorMsg::FallbackPoll { done: done_tx })
        .unwrap();
    done_rx.await.unwrap();
    drop(tx);
    handle.await.unwrap();

    assert_eq!(
        driver.events_order.lock().await.as_slice(),
        &["active:103:420", "ended", "poll"]
    );
    assert!(driver.session_state.lock().await.last_payload.is_none());
    assert_eq!(driver.active_queue_id.load(Ordering::Relaxed), 0);
    assert_eq!(get_champion_selections(103), None);
}

#[tokio::test]
async fn test_fifo_actor_processes_probe_and_disconnect_in_order() {
    let _sel_guard = TEST_SELECTIONS_LOCK.lock().await;
    clear_champion_selections();

    let driver = Arc::new(MockActorDriver::new(0, QUEUE_SOLO_DUO));
    let poll_lock = Arc::new(tokio::sync::Mutex::new(()));
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    let driver_task = Arc::clone(&driver);
    let q_ref = Arc::clone(&driver.active_queue_id);
    let p_ref = Arc::clone(&driver.last_gameflow_phase);
    let s_ref = Arc::clone(&driver.session_state);
    let l_ref = Arc::clone(&poll_lock);

    let handle = tokio::spawn(async move {
        run_lcu_event_actor_with_driver(driver_task.as_ref(), &q_ref, &p_ref, &s_ref, &l_ref, rx)
            .await;
    });

    tx.send(LcuEventActorMsg::WsGameflowPhase {
        phase: "Lobby".to_string(),
    })
    .unwrap();
    let (probe_tx, probe_rx) = oneshot::channel();
    tx.send(LcuEventActorMsg::ProbeSnapshotIfNeeded { done: probe_tx })
        .unwrap();
    probe_rx.await.unwrap();

    let (disc_tx, disc_rx) = oneshot::channel();
    tx.send(LcuEventActorMsg::LcuDisconnected { done: disc_tx })
        .unwrap();
    disc_rx.await.unwrap();
    drop(tx);
    handle.await.unwrap();

    assert_eq!(
        driver.events_order.lock().await.as_slice(),
        &["phase:Lobby", "poll", "disconnected"]
    );
    assert_eq!(driver.poll_calls.load(Ordering::SeqCst), 1);
    assert!(driver.last_gameflow_phase.lock().await.is_empty());
}

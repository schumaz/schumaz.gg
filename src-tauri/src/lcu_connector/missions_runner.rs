use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::lcu_connector::client::LcuConnection;
use crate::lcu_connector::endpoints::missions::{
    JADE_OBJECTIVES_ENDPOINT, LOL_OBJECTIVES_ENDPOINT, TFT_OBJECTIVES_ENDPOINT,
};
use crate::lcu_connector::missions_parser::{
    current_epoch_millis, extract_unread_missions_from_flat_list,
    extract_unread_missions_from_objectives, extract_unread_series_ids,
};
use crate::lcu_connector::missions_tracker::{
    auto_mark_tracker, filter_candidates_with_tracker, record_sent_with_tracker,
};

static AUTO_MARK_SWEEP_ACTIVE: AtomicBool = AtomicBool::new(false);
static AUTO_MARK_WS_DEBOUNCE_SEQ: AtomicU64 = AtomicU64::new(0);
static AUTO_MARK_EXEC_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

fn auto_mark_exec_lock() -> &'static tokio::sync::Mutex<()> {
    AUTO_MARK_EXEC_LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

impl LcuConnection {
    /// Collects all unread, active missions across LoL/TFT/Jade objectives, flat `/lol-missions/v1/missions`,
    /// and `/lol-missions/v1/series`, and marks them as read/viewed to clear the client notification counter completely.
    pub async fn mark_unread_missions_as_read(&self) -> Result<usize, String> {
        let now_ms = current_epoch_millis();

        let (lol_res, tft_res, jade_res, missions_res, series_res) = tokio::join!(
            self.get_objectives_for_endpoint(LOL_OBJECTIVES_ENDPOINT),
            self.get_objectives_for_endpoint(TFT_OBJECTIVES_ENDPOINT),
            self.get_objectives_for_endpoint(JADE_OBJECTIVES_ENDPOINT),
            self.get_missions(),
            self.get_mission_series(),
        );

        let mut combined_missions: Vec<(String, bool)> = Vec::new();
        let mut seen_mission_ids: HashMap<String, usize> = HashMap::new();

        let mut push_mission = |id: String, is_new: bool| {
            if let Some(&idx) = seen_mission_ids.get(&id) {
                if is_new {
                    combined_missions[idx].1 = true;
                }
            } else {
                let idx = combined_missions.len();
                seen_mission_ids.insert(id.clone(), idx);
                combined_missions.push((id, is_new));
            }
        };

        for obj_json in [lol_res, tft_res, jade_res].into_iter().flatten() {
            for (id, is_new) in extract_unread_missions_from_objectives(&obj_json, now_ms) {
                push_mission(id, is_new);
            }
        }

        if let Ok(missions_json) = missions_res {
            for (id, is_new) in extract_unread_missions_from_flat_list(&missions_json, now_ms) {
                push_mission(id, is_new);
            }
        }

        let series_candidates = series_res
            .map(|json| extract_unread_series_ids(&json, now_ms))
            .unwrap_or_default();

        if combined_missions.is_empty() && series_candidates.is_empty() {
            return Ok(0);
        }

        let now_instant = Instant::now();
        let (missions_to_send, series_to_send) = {
            let Ok(mut guard) = auto_mark_tracker().lock() else {
                return Ok(0);
            };
            filter_candidates_with_tracker(
                &mut guard,
                &combined_missions,
                &series_candidates,
                now_instant,
            )
        };

        if missions_to_send.is_empty() && series_to_send.is_empty() {
            return Ok(0);
        }

        let mission_ids: Vec<String> = missions_to_send.iter().map(|(id, _)| id.clone()).collect();
        self.mark_player_items_viewed(&mission_ids, &series_to_send)
            .await?;

        if let Ok(mut guard) = auto_mark_tracker().lock() {
            record_sent_with_tracker(
                &mut guard,
                &missions_to_send,
                &series_to_send,
                Instant::now(),
            );
        }

        Ok(mission_ids.len() + series_to_send.len())
    }
}

async fn run_auto_mark_once() {
    if !crate::lcu_injector::get_user_preferences().auto_mark_daily_missions_read {
        return;
    }

    let _exec_guard = auto_mark_exec_lock().lock().await;
    if !crate::lcu_injector::get_user_preferences().auto_mark_daily_missions_read {
        return;
    }

    let Ok(conn) = LcuConnection::connect() else {
        return;
    };

    let _ = conn.mark_unread_missions_as_read().await;
}

/// Debounced trigger called by the LCU WebSocket listener whenever missions or objectives update.
pub fn schedule_debounced_auto_mark_missions() {
    if !crate::lcu_injector::get_user_preferences().auto_mark_daily_missions_read {
        return;
    }

    let seq = AUTO_MARK_WS_DEBOUNCE_SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(1200)).await;
        if AUTO_MARK_WS_DEBOUNCE_SEQ.load(Ordering::SeqCst) != seq {
            return;
        }
        run_auto_mark_once().await;
    });
}

/// Spawns a non-blocking post-connection sweep (~65s window) to catch delayed mission hydration
/// after opening the League Client, and executes immediately if triggered by Settings toggle.
pub fn spawn_auto_mark_daily_missions_if_enabled() {
    if !crate::lcu_injector::get_user_preferences().auto_mark_daily_missions_read {
        return;
    }

    if AUTO_MARK_SWEEP_ACTIVE
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        // A startup sweep is already active; run an immediate pass in case the user just toggled it ON.
        tokio::spawn(async move {
            run_auto_mark_once().await;
        });
        return;
    }

    tokio::spawn(async move {
        struct ResetGuard;
        impl Drop for ResetGuard {
            fn drop(&mut self) {
                AUTO_MARK_SWEEP_ACTIVE.store(false, Ordering::SeqCst);
            }
        }
        let _guard = ResetGuard;

        // Post-login/startup mission hydration in the League Client can take ~30+ seconds.
        // Sweep immediately and periodically across the first ~65s without spamming PUTs.
        let step_delays_secs: [u64; 13] = [0, 3, 3, 4, 5, 5, 5, 5, 5, 5, 5, 10, 10];
        for delay_secs in step_delays_secs {
            if delay_secs > 0 {
                tokio::time::sleep(Duration::from_secs(delay_secs)).await;
            }
            if !crate::lcu_injector::get_user_preferences().auto_mark_daily_missions_read {
                return;
            }
            run_auto_mark_once().await;
        }
    });
}

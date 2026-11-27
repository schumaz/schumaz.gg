use std::sync::Arc;
use std::time::Duration;
use tauri_specta::Event as SpectaEvent;

pub use super::gameflow_rules::{
    is_in_game_phase, should_emit_match_ended, should_request_profile_refresh,
};
use crate::logging;
use crate::models;

pub(crate) fn log_champ_select_change(
    prev: Option<&crate::models::ChampSelectedPayload>,
    next: &crate::models::ChampSelectedPayload,
) {
    let event = match prev {
        None => "champ_select_started",
        Some(p) if p.champion_id != next.champion_id => "champion_changed",
        Some(p) if p.queue_id != next.queue_id => "queue_changed",
        Some(p) if !p.locked_in && next.locked_in => "champion_locked",
        _ => "champ_select_updated",
    };
    logging::log_info(
        "ws.champ_select",
        format!(
            "event={event} champ_id={} queue_id={} locked_in={}",
            next.champion_id, next.queue_id, next.locked_in
        ),
    );
}

pub(crate) async fn process_gameflow_phase_update(
    handle: &tauri::AppHandle,
    phase_state: &Arc<tokio::sync::Mutex<String>>,
    next_phase: String,
) {
    let next_phase = next_phase.trim().to_string();
    if next_phase.is_empty() {
        return;
    }

    let (changed, prev_phase, emit_match_ended, should_refresh) = {
        let mut guard = phase_state.lock().await;
        if *guard == next_phase {
            (false, String::new(), false, false)
        } else {
            let prev = guard.clone();
            let match_ended = should_emit_match_ended(&guard, &next_phase);
            let refresh = should_request_profile_refresh(&guard, &next_phase);
            *guard = next_phase.clone();
            (true, prev, match_ended, refresh)
        }
    };

    if changed {
        let from_label = if prev_phase.is_empty() {
            "None"
        } else {
            prev_phase.as_str()
        };
        logging::log_info(
            "ws.gameflow",
            format!("event=phase_changed from={from_label} to={next_phase}"),
        );
        let in_game_now = is_in_game_phase(&next_phase);
        let prev_in_game = super::reconcile_state::set_in_game_phase_active(in_game_now);
        if prev_in_game != in_game_now {
            crate::app_bootstrap::sync_main_webview_power_state(handle);
        }
        let _ = models::GameflowPhaseChanged(next_phase).emit(handle);
        if in_game_now {
            tokio::spawn(super::lobby_actions::sync_swiftplay_assigned_champion_on_match_start(
                handle.clone(),
            ));
        }
    }

    if emit_match_ended {
        crate::lcu_injector::clear_champion_selections();
        {
            let state_ref = super::reconcile_state::shared_session_state();
            let mut guard = state_ref.lock().await;
            if guard.last_payload.as_ref().is_some_and(|p| p.is_lobby_preselect) {
                guard.reset_session();
                super::reconcile_state::shared_active_queue_id()
                    .store(0, std::sync::atomic::Ordering::Relaxed);
            }
        }
        let _ = models::MatchEnded.emit(handle);
    }

    if should_refresh {
        let refresh_handle = handle.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            let _ = models::ProfileRefreshRequested.emit(&refresh_handle);
        });
    }
}

pub(crate) async fn sync_gameflow_phase_before_champ_select_end(
    handle: &tauri::AppHandle,
    phase_state: &Arc<tokio::sync::Mutex<String>>,
) {
    let Ok(conn) = crate::lcu_connector::LcuConnection::connect() else {
        return;
    };

    let mut phase_opt = conn.get_gameflow_phase().await.ok();
    if phase_opt.as_deref() == Some("ChampSelect") {
        // LCU sometimes deletes /lol-champ-select/v1/session a fraction of a second
        // before /lol-gameflow/v1/gameflow-phase flips to GameStart or Lobby.
        tokio::time::sleep(Duration::from_millis(250)).await;
        if let Ok(updated) = conn.get_gameflow_phase().await {
            phase_opt = Some(updated);
        }
    }

    if let Some(phase) = phase_opt {
        process_gameflow_phase_update(handle, phase_state, phase).await;
    }
}

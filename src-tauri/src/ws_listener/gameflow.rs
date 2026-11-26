use std::sync::Arc;
use std::time::Duration;
use tauri_specta::Event as SpectaEvent;

use crate::logging;
use crate::models;

/// Returns true when the LCU gameflow phase indicates the match has started or is active.
pub fn is_in_game_phase(phase: &str) -> bool {
    matches!(phase, "GameStart" | "InProgress" | "Reconnect")
}

/// Returns true when a gameflow phase transition indicates a match has ended
/// (either normally post-game or aborted after launch), so the UI should return to Profile.
pub fn should_emit_match_ended(prev_phase: &str, next_phase: &str) -> bool {
    let was_in_match = is_in_game_phase(prev_phase);
    let now_post_game = matches!(
        next_phase,
        "WaitingForStats" | "PreEndOfGame" | "EndOfGame" | "Lobby" | "None" | "TerminatedInError"
    );
    was_in_match && now_post_game
}

/// Returns true when a gameflow phase transition indicates a finished match
/// whose match history / ranked stats / masteries should be refreshed in the Profile tab.
pub fn should_request_profile_refresh(prev_phase: &str, next_phase: &str) -> bool {
    let was_in_game = matches!(prev_phase, "InProgress" | "Reconnect");
    let now_post_game = matches!(
        next_phase,
        "WaitingForStats" | "PreEndOfGame" | "EndOfGame" | "Lobby" | "None" | "TerminatedInError"
    );
    was_in_game && now_post_game
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_in_game_phase() {
        assert!(is_in_game_phase("GameStart"));
        assert!(is_in_game_phase("InProgress"));
        assert!(is_in_game_phase("Reconnect"));
        assert!(!is_in_game_phase("ChampSelect"));
        assert!(!is_in_game_phase("Lobby"));
        assert!(!is_in_game_phase("EndOfGame"));
    }

    #[test]
    fn test_should_emit_match_ended_on_post_game_or_aborted_launch() {
        assert!(should_emit_match_ended("InProgress", "WaitingForStats"));
        assert!(should_emit_match_ended("InProgress", "PreEndOfGame"));
        assert!(should_emit_match_ended("InProgress", "EndOfGame"));
        assert!(should_emit_match_ended("InProgress", "Lobby"));
        assert!(should_emit_match_ended("InProgress", "None"));
        assert!(should_emit_match_ended("InProgress", "TerminatedInError"));
        assert!(should_emit_match_ended("Reconnect", "EndOfGame"));
        assert!(should_emit_match_ended("GameStart", "Lobby"));
        assert!(!should_emit_match_ended("GameStart", "InProgress"));
        assert!(!should_emit_match_ended("ChampSelect", "Lobby"));
        assert!(!should_emit_match_ended("ChampSelect", "InProgress"));
        assert!(!should_emit_match_ended("EndOfGame", "Lobby"));
    }

    #[test]
    fn test_should_request_profile_refresh_on_post_game_transition() {
        assert!(should_request_profile_refresh(
            "InProgress",
            "WaitingForStats"
        ));
        assert!(should_request_profile_refresh("InProgress", "EndOfGame"));
        assert!(should_request_profile_refresh("Reconnect", "Lobby"));
        assert!(should_request_profile_refresh(
            "InProgress",
            "TerminatedInError"
        ));
        assert!(!should_request_profile_refresh("Lobby", "ChampSelect"));
        assert!(!should_request_profile_refresh("ChampSelect", "InProgress"));
        assert!(!should_request_profile_refresh("EndOfGame", "Lobby"));
    }
}

use std::time::Duration;
use tauri_specta::Event;

use crate::lcu_connector::client::LcuConnection;
use crate::lcu_connector::missions_runner::spawn_auto_mark_daily_missions_if_enabled;
use crate::lcu_connector::missions_tracker::reset_auto_mark_session_state;
use crate::models::LcuStatus;

/// Calculates the reconnection backoff duration in seconds.
/// Starts at 2s and caps at 10s to conserve CPU while remaining responsive.
pub fn calculate_backoff_secs(attempt: u32) -> u64 {
    let base = 2u64;
    let max_secs = 10u64;
    let delay = base.saturating_mul(1u64 << attempt.min(3));
    delay.min(max_secs)
}

/// Determines whether an event should be emitted to the frontend based on status transitions.
pub fn should_emit_status_change(prev: &LcuStatus, current: &LcuStatus) -> bool {
    prev != current
}

/// Background task that continuously monitors LCU availability and notifies the UI.
pub async fn start_lcu_monitor(app_handle: tauri::AppHandle) {
    let mut current_status = LcuStatus::Disconnected;
    let mut reconnect_attempt: u32 = 0;

    loop {
        match LcuConnection::connect() {
            Ok(conn) => {
                match conn.get_current_summoner().await {
                    Ok(summoner) => {
                        let new_status = LcuStatus::Connected(summoner.clone());
                        if should_emit_status_change(&current_status, &new_status) {
                            let _ = crate::models::LcuConnected(summoner.clone()).emit(&app_handle);
                            current_status = new_status;
                            reset_auto_mark_session_state();
                            spawn_auto_mark_daily_missions_if_enabled();
                        }
                        reconnect_attempt = 0;
                        // Client is connected; check liveness every 3 seconds
                        tokio::time::sleep(Duration::from_secs(3)).await;
                    }
                    Err(_) => {
                        // Process found but summoner endpoint not ready yet (e.g., logging in)
                        let new_status = LcuStatus::Disconnected;
                        if should_emit_status_change(&current_status, &new_status) {
                            let _ = crate::models::LcuDisconnected.emit(&app_handle);
                            current_status = new_status;
                            reset_auto_mark_session_state();
                        }
                        let delay = calculate_backoff_secs(reconnect_attempt);
                        reconnect_attempt = reconnect_attempt.saturating_add(1);
                        tokio::time::sleep(Duration::from_secs(delay)).await;
                    }
                }
            }
            Err(_) => {
                let new_status = LcuStatus::Disconnected;
                if should_emit_status_change(&current_status, &new_status) {
                    let _ = crate::models::LcuDisconnected.emit(&app_handle);
                    current_status = new_status;
                    reset_auto_mark_session_state();
                }
                let delay = calculate_backoff_secs(reconnect_attempt);
                reconnect_attempt = reconnect_attempt.saturating_add(1);
                tokio::time::sleep(Duration::from_secs(delay)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SummonerInfo;

    #[test]
    fn test_calculate_backoff_secs_caps_at_maximum() {
        assert_eq!(calculate_backoff_secs(0), 2);
        assert_eq!(calculate_backoff_secs(1), 4);
        assert_eq!(calculate_backoff_secs(2), 8);
        assert_eq!(calculate_backoff_secs(3), 10);
        assert_eq!(calculate_backoff_secs(10), 10);
    }

    #[test]
    fn test_should_emit_status_change_transitions() {
        let summoner_a = SummonerInfo {
            summoner_id: 1,
            game_name: "PlayerA".to_string(),
            tag_line: "BR1".to_string(),
            display_name: "PlayerA".to_string(),
            summoner_level: 100,
            profile_icon_id: 10,
        };

        let summoner_b = SummonerInfo {
            summoner_id: 2,
            game_name: "PlayerB".to_string(),
            tag_line: "BR1".to_string(),
            display_name: "PlayerB".to_string(),
            summoner_level: 200,
            profile_icon_id: 20,
        };

        // Transition from Disconnected to Connected
        assert!(should_emit_status_change(
            &LcuStatus::Disconnected,
            &LcuStatus::Connected(summoner_a.clone())
        ));

        // Same status should not re-emit
        assert!(!should_emit_status_change(
            &LcuStatus::Connected(summoner_a.clone()),
            &LcuStatus::Connected(summoner_a.clone())
        ));

        // Account change should emit
        assert!(should_emit_status_change(
            &LcuStatus::Connected(summoner_a.clone()),
            &LcuStatus::Connected(summoner_b.clone())
        ));

        // Transition from Connected to Disconnected
        assert!(should_emit_status_change(
            &LcuStatus::Connected(summoner_a),
            &LcuStatus::Disconnected
        ));

        // Same Disconnected status should not re-emit
        assert!(!should_emit_status_change(
            &LcuStatus::Disconnected,
            &LcuStatus::Disconnected
        ));
    }
}

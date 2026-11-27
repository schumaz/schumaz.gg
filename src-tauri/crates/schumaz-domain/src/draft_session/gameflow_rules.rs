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

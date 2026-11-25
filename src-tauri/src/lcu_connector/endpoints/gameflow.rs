use crate::lcu_connector::client::LcuConnection;

pub const GAMEFLOW_SESSION_ENDPOINT: &str = "/lol-gameflow/v1/session";
pub const GAMEFLOW_PHASE_ENDPOINT: &str = "/lol-gameflow/v1/gameflow-phase";
pub const CHAMP_SELECT_SESSION_ENDPOINT: &str = "/lol-champ-select/v1/session";
pub const LOBBY_ENDPOINT: &str = "/lol-lobby/v2/lobby";
pub const LOBBY_PLAYER_SLOTS_ENDPOINT: &str = "/lol-lobby/v1/lobby/members/localMember/player-slots";

impl LcuConnection {
    /// Fetches the full active gameflow session JSON via GET /lol-gameflow/v1/session.
    pub async fn get_gameflow_session(&self) -> Result<Option<serde_json::Value>, String> {
        self.get_json_opt(GAMEFLOW_SESSION_ENDPOINT, "gameflow session")
            .await
    }

    /// Fetches the current queue ID via GET /lol-gameflow/v1/session.
    pub async fn get_current_queue_id(&self) -> Result<i32, String> {
        let val = self
            .get_json_required(GAMEFLOW_SESSION_ENDPOINT, "gameflow session")
            .await?;

        crate::ws_listener::extract_queue_id(&val)
    }

    /// Fetches the current gameflow phase string via GET /lol-gameflow/v1/gameflow-phase.
    pub async fn get_gameflow_phase(&self) -> Result<String, String> {
        let response = self
            .get_json_opt(GAMEFLOW_PHASE_ENDPOINT, "gameflow phase")
            .await?;

        match response.and_then(|v| v.as_str().map(|s| s.trim().to_string())) {
            Some(phase) if !phase.is_empty() => Ok(phase),
            _ => Err("LCU returned an empty or invalid response for gameflow phase".to_string()),
        }
    }

    /// Fetches the active champion select session via GET /lol-champ-select/v1/session.
    pub async fn get_champ_select_session(&self) -> Result<Option<serde_json::Value>, String> {
        self.get_json_opt(CHAMP_SELECT_SESSION_ENDPOINT, "champ select session")
            .await
    }

    /// Fetches the active lobby state via GET /lol-lobby/v2/lobby.
    pub async fn get_lobby_session(&self) -> Result<Option<serde_json::Value>, String> {
        self.get_json_opt(LOBBY_ENDPOINT, "lobby session").await
    }

    /// Fetches local member pre-selection slots via GET /lol-lobby/v1/lobby/members/localMember/player-slots.
    pub async fn get_lobby_player_slots(&self) -> Result<Option<serde_json::Value>, String> {
        self.get_json_opt(LOBBY_PLAYER_SLOTS_ENDPOINT, "lobby player slots")
            .await
    }

    /// Updates local member pre-selection slots via PUT /lol-lobby/v1/lobby/members/localMember/player-slots.
    pub async fn put_lobby_player_slots(&self, payload: &serde_json::Value) -> Result<(), String> {
        self.put_json_ignoring_eof(
            LOBBY_PLAYER_SLOTS_ENDPOINT,
            payload,
            "Error putting lobby player slots",
        )
        .await
    }
}

/// Convenience helper to query the active gameflow phase if the League Client is currently reachable.
pub async fn fetch_current_gameflow_phase() -> Option<String> {
    let conn = LcuConnection::connect().ok()?;
    conn.get_gameflow_phase().await.ok()
}

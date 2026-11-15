use crate::lcu_connector::client::LcuConnection;

pub const GAMEFLOW_SESSION_ENDPOINT: &str = "/lol-gameflow/v1/session";
pub const GAMEFLOW_PHASE_ENDPOINT: &str = "/lol-gameflow/v1/gameflow-phase";
pub const CHAMP_SELECT_SESSION_ENDPOINT: &str = "/lol-champ-select/v1/session";

impl LcuConnection {
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
}

use crate::lcu_connector::client::LcuConnection;

pub const OBJECTIVES_READY_ENDPOINT: &str = "/lol-objectives/v1/ready";
pub const LOL_OBJECTIVES_ENDPOINT: &str = "/lol-objectives/v1/objectives/lol";
pub const TFT_OBJECTIVES_ENDPOINT: &str = "/lol-objectives/v1/objectives/tft";
pub const JADE_OBJECTIVES_ENDPOINT: &str = "/lol-objectives/v1/objectives/jade";
pub const OBJECTIVES_ENDPOINTS: [&str; 3] = [
    LOL_OBJECTIVES_ENDPOINT,
    TFT_OBJECTIVES_ENDPOINT,
    JADE_OBJECTIVES_ENDPOINT,
];
pub const LOL_MISSIONS_ENDPOINT: &str = "/lol-missions/v1/missions";
pub const LOL_MISSIONS_SERIES_ENDPOINT: &str = "/lol-missions/v1/series";
pub const LOL_MISSIONS_PLAYER_ENDPOINT: &str = "/lol-missions/v1/player";

impl LcuConnection {
    /// Checks if the objectives plugin has finished loading via GET /lol-objectives/v1/ready.
    pub async fn get_objectives_ready(&self) -> Result<bool, String> {
        let response = self
            .get_json_opt(OBJECTIVES_READY_ENDPOINT, "objectives readiness")
            .await?;

        Ok(response.and_then(|v| v.as_bool()).unwrap_or(false))
    }

    /// Fetches objectives groups for a given game tab (`lol`, `tft`, `jade`) via GET /lol-objectives/v1/objectives/{gameType}.
    pub async fn get_objectives_for_endpoint(
        &self,
        endpoint: &str,
    ) -> Result<serde_json::Value, String> {
        let response: Option<serde_json::Value> = self
            .raw_client()
            .get(endpoint)
            .await
            .map_err(|err| format!("Error requesting objectives ({}): {:?}", endpoint, err))?;

        response.ok_or_else(|| format!("LCU returned an empty response for {}", endpoint))
    }

    /// Fetches LoL objectives groups via GET /lol-objectives/v1/objectives/lol.
    pub async fn get_lol_objectives(&self) -> Result<serde_json::Value, String> {
        self.get_objectives_for_endpoint(LOL_OBJECTIVES_ENDPOINT)
            .await
    }

    /// Fetches player missions via GET /lol-missions/v1/missions.
    pub async fn get_missions(&self) -> Result<serde_json::Value, String> {
        self.get_json_required(LOL_MISSIONS_ENDPOINT, "missions")
            .await
    }

    /// Fetches player mission series via GET /lol-missions/v1/series.
    pub async fn get_mission_series(&self) -> Result<serde_json::Value, String> {
        self.get_json_required(LOL_MISSIONS_SERIES_ENDPOINT, "mission series")
            .await
    }

    /// Marks the given mission and series IDs as viewed/read via PUT /lol-missions/v1/player.
    pub async fn mark_player_items_viewed(
        &self,
        mission_ids: &[String],
        series_ids: &[String],
    ) -> Result<(), String> {
        if mission_ids.is_empty() && series_ids.is_empty() {
            return Ok(());
        }

        let payload = serde_json::json!({
            "missionIds": mission_ids,
            "seriesIds": series_ids
        });

        self.put_json_ignoring_eof(
            LOL_MISSIONS_PLAYER_ENDPOINT,
            &payload,
            "Error marking missions viewed",
        )
        .await
    }

    /// Marks the given mission IDs as viewed/read via PUT /lol-missions/v1/player.
    pub async fn mark_missions_viewed(&self, mission_ids: &[String]) -> Result<(), String> {
        self.mark_player_items_viewed(mission_ids, &[]).await
    }
}

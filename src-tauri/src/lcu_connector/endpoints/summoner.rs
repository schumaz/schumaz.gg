use crate::lcu_connector::client::LcuConnection;
use crate::models::SummonerInfo;

pub const CURRENT_SUMMONER_ENDPOINT: &str = "/lol-summoner/v1/current-summoner";
pub const CURRENT_SUMMONER_PROFILE_ENDPOINT: &str =
    "/lol-summoner/v1/current-summoner/summoner-profile";
pub const CHAMPION_MASTERIES_ENDPOINT: &str =
    "/lol-champion-mastery/v1/local-player/champion-masteries";
pub const RANKED_STATS_ENDPOINT: &str = "/lol-ranked/v1/current-ranked-stats";
pub const DEFAULT_MATCH_HISTORY_PAGE_SIZE: u32 = 10;
pub const MAX_MATCH_HISTORY_PAGE_SIZE: u32 = 50;
pub const MATCH_HISTORY_ENDPOINT: &str =
    "/lol-match-history/v1/products/lol/current-summoner/matches?begIndex=0&endIndex=9";
pub const MATCH_HISTORY_FALLBACK_ENDPOINT: &str =
    "/lol-match-history/v1/products/lol/current-summoner/matches";
pub const MATCH_DETAILS_ENDPOINT_PREFIX: &str = "/lol-match-history/v1/games";

/// Builds the LCU match history endpoint query for an inclusive `[beg_index..=end_index]` window.
pub fn build_match_history_page_endpoint(beg_index: u32, end_index: u32) -> String {
    let normalized_end = end_index.max(beg_index);
    format!(
        "{}?begIndex={}&endIndex={}",
        MATCH_HISTORY_FALLBACK_ENDPOINT, beg_index, normalized_end
    )
}

/// Builds the LCU full match details endpoint for a specific `game_id`.
pub fn build_match_details_endpoint(game_id: u64) -> String {
    format!("{MATCH_DETAILS_ENDPOINT_PREFIX}/{game_id}")
}

fn is_lcu_error_payload(val: &serde_json::Value) -> bool {
    val.get("errorCode").is_some()
        || val
            .get("httpStatus")
            .and_then(|s| s.as_i64())
            .is_some_and(|s| s >= 400)
}

/// Pure parser function to extract summoner information from a raw JSON string.
/// Falls back to `displayName` if `gameName` is empty.
pub fn parse_summoner_info(raw_json: &str) -> Result<SummonerInfo, String> {
    let mut info: SummonerInfo = serde_json::from_str(raw_json)
        .map_err(|err| format!("Failed to parse summoner JSON: {}", err))?;

    // Apply fallback if game_name is empty (legacy accounts or edge-case response)
    if info.game_name.is_empty() && !info.display_name.is_empty() {
        info.game_name = info.display_name.clone();
    }

    Ok(info)
}

/// Pure parser function to extract summoner information from a `serde_json::Value`.
/// Useful for deserializing irelia's JSON responses directly.
pub fn parse_summoner_value(value: &serde_json::Value) -> Result<SummonerInfo, String> {
    let mut info: SummonerInfo = serde_json::from_value(value.clone())
        .map_err(|err| format!("Failed to convert serde value to SummonerInfo: {}", err))?;

    if info.game_name.is_empty() && !info.display_name.is_empty() {
        info.game_name = info.display_name.clone();
    }

    Ok(info)
}

/// Builds prioritized LCU mastery endpoints, putting full-catalog endpoints first and `top?limit=3` strictly last.
pub fn build_champion_mastery_endpoints(summoner_id: u64, puuid: Option<&str>) -> Vec<String> {
    let puuid_trimmed = puuid.map(str::trim).filter(|p| !p.is_empty());
    let mut endpoints = vec![CHAMPION_MASTERIES_ENDPOINT.to_string()];

    if let Some(p) = puuid_trimmed {
        endpoints.push(format!("/lol-champion-mastery/v1/{p}/champion-masteries"));
        endpoints.push(format!("/lol-champion-mastery/v1/{p}/champion-mastery"));
    }

    endpoints.push("/lol-champion-mastery/v1/local-player/champion-mastery".to_string());

    if summoner_id > 0 {
        endpoints.push(format!(
            "/lol-collections/v1/inventories/{summoner_id}/champion-mastery"
        ));
    }

    if let Some(p) = puuid_trimmed {
        endpoints.push(format!("/lol-collections/v1/inventories/{p}/champion-mastery"));
    }

    if summoner_id > 0 {
        endpoints.push(format!(
            "/lol-collections/v1/inventories/{summoner_id}/champion-mastery/top?limit=3"
        ));
    }

    endpoints
}

impl LcuConnection {
    /// Fetches the current summoner info via GET /lol-summoner/v1/current-summoner.
    pub async fn get_current_summoner(&self) -> Result<SummonerInfo, String> {
        let (info, _) = self.get_current_summoner_with_puuid().await?;
        Ok(info)
    }

    /// Fetches current summoner info along with PUUID when provided by LCU.
    pub async fn get_current_summoner_with_puuid(
        &self,
    ) -> Result<(SummonerInfo, Option<String>), String> {
        let val = self
            .get_json_required(CURRENT_SUMMONER_ENDPOINT, "current summoner")
            .await?;

        let info = parse_summoner_value(&val)?;
        let puuid = val
            .get("puuid")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        Ok((info, puuid))
    }

    /// Fetches the current summoner profile (including backgroundSkinId) via GET /lol-summoner/v1/current-summoner/summoner-profile.
    pub async fn get_summoner_profile(&self) -> Result<serde_json::Value, String> {
        self.get_json_required(CURRENT_SUMMONER_PROFILE_ENDPOINT, "summoner profile")
            .await
    }

    /// Fetches local player champion masteries across supported LCU inventory/mastery endpoints.
    pub async fn get_champion_masteries(
        &self,
        summoner_id: u64,
        puuid: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let endpoints = build_champion_mastery_endpoints(summoner_id, puuid);
        let mut fallback_valid: Option<serde_json::Value> = None;

        for ep in endpoints {
            if let Ok(Some(val)) = self
                .raw_client()
                .get::<Option<serde_json::Value>>(&ep)
                .await
            {
                if is_lcu_error_payload(&val) {
                    continue;
                }

                let has_entries = val.as_array().is_some_and(|a| !a.is_empty())
                    || val
                        .get("masteries")
                        .and_then(|v| v.as_array())
                        .is_some_and(|a| !a.is_empty())
                    || val
                        .get("championMasteries")
                        .and_then(|v| v.as_array())
                        .is_some_and(|a| !a.is_empty())
                    || val
                        .get("data")
                        .and_then(|v| v.as_array())
                        .is_some_and(|a| !a.is_empty());

                if has_entries {
                    return Ok(val);
                }

                if fallback_valid.is_none() {
                    fallback_valid = Some(val);
                }
            }
        }

        fallback_valid.ok_or_else(|| "No champion mastery endpoint returned valid data".to_string())
    }

    /// Fetches ranked queue stats via GET /lol-ranked/v1/current-ranked-stats.
    pub async fn get_ranked_stats(&self) -> Result<serde_json::Value, String> {
        self.get_json_required(RANKED_STATS_ENDPOINT, "ranked stats")
            .await
    }

    /// Fetches an inclusive page (`begIndex..=endIndex`) of recent match history.
    /// Only falls back to the unparameterized endpoint when `beg_index == 0`.
    pub async fn get_match_history_page(
        &self,
        beg_index: u32,
        end_index: u32,
    ) -> Result<serde_json::Value, String> {
        let endpoint = build_match_history_page_endpoint(beg_index, end_index);
        match self.get_json_opt(&endpoint, "match history page").await {
            Ok(Some(val)) if !is_lcu_error_payload(&val) => return Ok(val),
            Ok(Some(_)) if beg_index > 0 => {
                return Err(format!(
                    "LCU returned an error payload for match history page begIndex={beg_index}"
                ));
            }
            Ok(None) if beg_index > 0 => {
                return Err(format!(
                    "LCU returned an empty response for match history page begIndex={beg_index}"
                ));
            }
            Err(err) if beg_index > 0 => return Err(err),
            Ok(_) | Err(_) => {}
        }

        let fallback = self
            .get_json_required(MATCH_HISTORY_FALLBACK_ENDPOINT, "match history")
            .await?;
        if is_lcu_error_payload(&fallback) {
            return Err("LCU returned an error payload for match history".to_string());
        }
        Ok(fallback)
    }

    /// Fetches the initial page (0..=9) of recent match history via GET /lol-match-history/v1/products/lol/current-summoner/matches.
    pub async fn get_match_history(&self) -> Result<serde_json::Value, String> {
        self.get_match_history_page(0, DEFAULT_MATCH_HISTORY_PAGE_SIZE - 1)
            .await
    }

    /// Fetches full participant and team details for a specific match via GET /lol-match-history/v1/games/{gameId}.
    pub async fn get_match_details(&self, game_id: u64) -> Result<serde_json::Value, String> {
        if game_id == 0 {
            return Err("Invalid match gameId".to_string());
        }
        let endpoint = build_match_details_endpoint(game_id);
        let val = self.get_json_required(&endpoint, "match details").await?;
        if is_lcu_error_payload(&val) {
            return Err(format!("LCU returned an error payload for match {game_id}"));
        }
        Ok(val)
    }
}

/// Connects to the local LCU (when running) and returns the current summoner info if available.
pub async fn fetch_current_summoner_status() -> Option<SummonerInfo> {
    let conn = LcuConnection::connect().ok()?;
    conn.get_current_summoner().await.ok()
}

#[cfg(test)]
#[path = "summoner_tests.rs"]
mod tests;

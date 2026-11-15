use crate::lcu_connector::client::LcuConnection;
use crate::models::SummonerInfo;

pub const CURRENT_SUMMONER_ENDPOINT: &str = "/lol-summoner/v1/current-summoner";
pub const CURRENT_SUMMONER_PROFILE_ENDPOINT: &str =
    "/lol-summoner/v1/current-summoner/summoner-profile";
pub const CHAMPION_MASTERIES_ENDPOINT: &str =
    "/lol-champion-mastery/v1/local-player/champion-masteries";
pub const RANKED_STATS_ENDPOINT: &str = "/lol-ranked/v1/current-ranked-stats";
pub const MATCH_HISTORY_ENDPOINT: &str =
    "/lol-match-history/v1/products/lol/current-summoner/matches?begIndex=0&endIndex=29";
pub const MATCH_HISTORY_FALLBACK_ENDPOINT: &str =
    "/lol-match-history/v1/products/lol/current-summoner/matches";

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
        let mut endpoints = vec![
            format!(
                "/lol-collections/v1/inventories/{}/champion-mastery",
                summoner_id
            ),
            format!(
                "/lol-collections/v1/inventories/{}/champion-mastery/top?limit=3",
                summoner_id
            ),
            "/lol-champion-mastery/v1/local-player/champion-mastery".to_string(),
            CHAMPION_MASTERIES_ENDPOINT.to_string(),
        ];

        if let Some(p) = puuid
            && !p.is_empty()
        {
            endpoints.push(format!(
                "/lol-collections/v1/inventories/{}/champion-mastery",
                p
            ));
            endpoints.push(format!("/lol-champion-mastery/v1/{}/champion-masteries", p));
            endpoints.push(format!("/lol-champion-mastery/v1/{}/champion-mastery", p));
        }

        let mut fallback_valid: Option<serde_json::Value> = None;

        for ep in endpoints {
            if let Ok(Some(val)) = self
                .raw_client()
                .get::<Option<serde_json::Value>>(&ep)
                .await
            {
                let is_lcu_err = val.get("errorCode").is_some()
                    || val
                        .get("httpStatus")
                        .and_then(|s| s.as_i64())
                        .is_some_and(|s| s >= 400);
                if is_lcu_err {
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

    /// Fetches recent match history via GET /lol-match-history/v1/products/lol/current-summoner/matches.
    pub async fn get_match_history(&self) -> Result<serde_json::Value, String> {
        if let Ok(Some(val)) = self
            .get_json_opt(MATCH_HISTORY_ENDPOINT, "match history")
            .await
        {
            return Ok(val);
        }

        self.get_json_required(MATCH_HISTORY_FALLBACK_ENDPOINT, "match history")
            .await
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

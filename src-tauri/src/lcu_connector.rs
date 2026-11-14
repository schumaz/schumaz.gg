//! Module 1: Connection and Authentication with the LCU (LeagueClientUx.exe)
//! Handles locating the client, authenticating via Basic Auth, and performing REST calls.

pub use crate::models::{LcuStatus, SummonerInfo};
use irelia::rest::LcuClient;
use std::time::Duration;
use tauri_specta::Event;

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
pub const GAMEFLOW_SESSION_ENDPOINT: &str = "/lol-gameflow/v1/session";
pub const CHAMP_SELECT_SESSION_ENDPOINT: &str = "/lol-champ-select/v1/session";

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

pub type DefaultRequestClient = hyper_util::client::legacy::Client<
    hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
    http_body_util::Full<bytes::Bytes>,
>;

pub type ConnectedLcuClient = LcuClient<DefaultRequestClient>;

/// High-level struct managing the REST connection to the League Client.
pub struct LcuConnection {
    client: ConnectedLcuClient,
}

impl LcuConnection {
    /// Attempts to locate and connect to the active League Client process (LeagueClientUx.exe).
    pub fn connect() -> Result<Self, String> {
        // Ensure rustls has a default crypto provider installed to avoid runtime ambiguity
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

        LcuClient::connect()
            .map(|client| Self { client })
            .map_err(|err| format!("Failed to connect to LCU process: {:?}", err))
    }

    /// Fetches the current summoner info via GET /lol-summoner/v1/current-summoner.
    pub async fn get_current_summoner(&self) -> Result<SummonerInfo, String> {
        let (info, _) = self.get_current_summoner_with_puuid().await?;
        Ok(info)
    }

    /// Fetches current summoner info along with PUUID when provided by LCU.
    pub async fn get_current_summoner_with_puuid(
        &self,
    ) -> Result<(SummonerInfo, Option<String>), String> {
        let response: Option<serde_json::Value> = self
            .client
            .get(CURRENT_SUMMONER_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting current summoner: {:?}", err))?;

        match response {
            Some(val) => {
                let info = parse_summoner_value(&val)?;
                let puuid = val
                    .get("puuid")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
                Ok((info, puuid))
            }
            None => Err("LCU returned an empty response for current summoner".to_string()),
        }
    }

    /// Fetches the current summoner profile (including backgroundSkinId) via GET /lol-summoner/v1/current-summoner/summoner-profile.
    pub async fn get_summoner_profile(&self) -> Result<serde_json::Value, String> {
        let response: Option<serde_json::Value> = self
            .client
            .get(CURRENT_SUMMONER_PROFILE_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting summoner profile: {:?}", err))?;

        response.ok_or_else(|| "LCU returned an empty response for summoner profile".to_string())
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
            endpoints.push(format!("/lol-collections/v1/inventories/{}/champion-mastery", p));
            endpoints.push(format!("/lol-champion-mastery/v1/{}/champion-masteries", p));
            endpoints.push(format!("/lol-champion-mastery/v1/{}/champion-mastery", p));
        }

        let mut fallback_valid: Option<serde_json::Value> = None;

        for ep in endpoints {
            if let Ok(Some(val)) = self.client.get::<Option<serde_json::Value>>(&ep).await {
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
        let response: Option<serde_json::Value> = self
            .client
            .get(RANKED_STATS_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting ranked stats: {:?}", err))?;

        response.ok_or_else(|| "LCU returned an empty response for ranked stats".to_string())
    }

    /// Fetches recent match history via GET /lol-match-history/v1/products/lol/current-summoner/matches.
    pub async fn get_match_history(&self) -> Result<serde_json::Value, String> {
        if let Ok(Some(val)) = self
            .client
            .get::<Option<serde_json::Value>>(MATCH_HISTORY_ENDPOINT)
            .await
        {
            return Ok(val);
        }

        let response: Option<serde_json::Value> = self
            .client
            .get(MATCH_HISTORY_FALLBACK_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting match history: {:?}", err))?;

        response.ok_or_else(|| "LCU returned an empty response for match history".to_string())
    }

    /// Fetches the current queue ID via GET /lol-gameflow/v1/session.
    pub async fn get_current_queue_id(&self) -> Result<i32, String> {
        let response: Option<serde_json::Value> = self
            .client
            .get(GAMEFLOW_SESSION_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting gameflow session: {:?}", err))?;

        match response {
            Some(val) => crate::ws_listener::extract_queue_id(&val),
            None => Err("LCU returned an empty response for gameflow session".to_string()),
        }
    }

    /// Fetches the active champion select session via GET /lol-champ-select/v1/session.
    pub async fn get_champ_select_session(&self) -> Result<Option<serde_json::Value>, String> {
        self.client
            .get(CHAMP_SELECT_SESSION_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting champ select session: {:?}", err))
    }

    /// Fetches the current rune page via GET /lol-perks/v1/currentpage.
    pub async fn get_current_rune_page(&self) -> Result<serde_json::Value, String> {
        let response: Option<serde_json::Value> = self
            .client
            .get(crate::lcu_injector::PERKS_CURRENT_PAGE_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting current rune page: {:?}", err))?;

        response.ok_or_else(|| "LCU returned an empty response for current rune page".to_string())
    }

    /// Fetches all rune pages via GET /lol-perks/v1/pages.
    pub async fn get_all_rune_pages(&self) -> Result<serde_json::Value, String> {
        let response: Option<serde_json::Value> = self
            .client
            .get(crate::lcu_injector::PERKS_PAGES_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting rune pages: {:?}", err))?;

        response.ok_or_else(|| "LCU returned an empty response for rune pages".to_string())
    }

    /// Updates an existing rune page in-place via PUT /lol-perks/v1/pages/{id}.
    pub async fn put_rune_page(
        &self,
        page_id: i64,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let endpoint = format!("{}/{}", crate::lcu_injector::PERKS_PAGES_ENDPOINT, page_id);
        let response: Option<serde_json::Value> = self
            .client
            .put(&endpoint, payload)
            .await
            .map_err(|err| format!("Error updating rune page {}: {:?}", page_id, err))?;

        response.ok_or_else(|| "LCU returned an empty response when updating rune page".to_string())
    }

    /// Deletes a rune page by ID via DELETE /lol-perks/v1/pages/{id}.
    pub async fn delete_rune_page(&self, page_id: i64) -> Result<(), String> {
        let endpoint = format!("{}/{}", crate::lcu_injector::PERKS_PAGES_ENDPOINT, page_id);
        match self
            .client
            .delete::<Option<serde_json::Value>>(&endpoint)
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => {
                let err_str = format!("{:?}", err);
                if err_str.contains("unexpected end of file")
                    || err_str.contains("Unexpected end of file")
                    || err_str.contains("Eof")
                    || err_str.contains("EOF")
                {
                    Ok(())
                } else {
                    Err(format!("Error deleting rune page {}: {}", page_id, err_str))
                }
            }
        }
    }

    /// Creates a new rune page via POST /lol-perks/v1/pages.
    pub async fn create_rune_page(
        &self,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let response: Option<serde_json::Value> = self
            .client
            .post(crate::lcu_injector::PERKS_PAGES_ENDPOINT, payload)
            .await
            .map_err(|err| format!("Error creating rune page: {:?}", err))?;

        response.ok_or_else(|| "LCU returned an empty response when creating rune page".to_string())
    }

    /// Updates summoner spells via PATCH /lol-champ-select/v1/session/my-selection.
    /// Retries up to 5 times with 200ms backoff if champ select session is transitioning.
    pub async fn patch_summoner_spells(&self, payload: &serde_json::Value) -> Result<(), String> {
        let mut last_err = String::new();
        for attempt in 1..=5 {
            match self
                .client
                .patch::<&serde_json::Value, Option<serde_json::Value>>(
                    crate::lcu_injector::CHAMP_SELECT_MY_SELECTION_ENDPOINT,
                    payload,
                )
                .await
            {
                Ok(_) => return Ok(()),
                Err(err) => {
                    let err_str = format!("{:?}", err);
                    if err_str.contains("unexpected end of file")
                        || err_str.contains("Unexpected end of file")
                        || err_str.contains("Eof")
                        || err_str.contains("EOF")
                    {
                        return Ok(());
                    }
                    last_err = format!("Error patching summoner spells: {}", err_str);
                    if attempt < 5 && (err_str.contains("404") || err_str.contains("NotFound")) {
                        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                        continue;
                    }
                    break;
                }
            }
        }
        Err(last_err)
    }

    /// Updates custom item sets via PUT /lol-item-sets/v1/item-sets/{summoner_id}/sets.
    pub async fn put_item_sets(
        &self,
        summoner_id: u64,
        payload: &serde_json::Value,
    ) -> Result<(), String> {
        let endpoint = format!("/lol-item-sets/v1/item-sets/{}/sets", summoner_id);
        match self
            .client
            .put::<&serde_json::Value, Option<serde_json::Value>>(&endpoint, payload)
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => {
                let err_str = format!("{:?}", err);
                if err_str.contains("unexpected end of file")
                    || err_str.contains("Unexpected end of file")
                    || err_str.contains("Eof")
                    || err_str.contains("EOF")
                {
                    Ok(())
                } else {
                    Err(format!("Error putting item sets: {}", err_str))
                }
            }
        }
    }
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

    #[test]
    fn test_parse_valid_current_summoner_json() {
        // Real fixture from /lol-summoner/v1/current-summoner
        let fixture = r#"{
            "accountId": 123456789,
            "displayName": "Schuma",
            "gameName": "Schuma",
            "internalName": "schuma",
            "nameChangeFlag": false,
            "percentCompleteForNextLevel": 42,
            "profileIconId": 548,
            "puuid": "00000000-0000-0000-0000-000000000000",
            "rerollPoints": {
                "currentPoints": 250,
                "maxRolls": 2,
                "numberOfRolls": 1,
                "pointsCostToRoll": 250,
                "pointsToReroll": 250
            },
            "summonerId": 987654321,
            "summonerLevel": 350,
            "tagLine": "BR1",
            "unnamed": false,
            "xpSinceLastLevel": 1200,
            "xpUntilNextLevel": 2800
        }"#;

        let result = parse_summoner_info(fixture);
        assert!(result.is_ok());

        let summoner = result.unwrap();
        assert_eq!(summoner.summoner_id, 987654321);
        assert_eq!(summoner.game_name, "Schuma");
        assert_eq!(summoner.tag_line, "BR1");
        assert_eq!(summoner.summoner_level, 350);
        assert_eq!(summoner.profile_icon_id, 548);
    }

    #[test]
    fn test_parse_summoner_fallback_to_display_name() {
        let fixture = r#"{
            "accountId": 123456789,
            "displayName": "LegacyPlayer",
            "gameName": "",
            "profileIconId": 29,
            "summonerId": 11223344,
            "summonerLevel": 45,
            "tagLine": "EUW"
        }"#;

        let result = parse_summoner_info(fixture);
        assert!(result.is_ok());

        let summoner = result.unwrap();
        assert_eq!(summoner.game_name, "LegacyPlayer");
        assert_eq!(summoner.summoner_id, 11223344);
        assert_eq!(summoner.summoner_level, 45);
    }

    #[test]
    fn test_parse_invalid_or_incomplete_json_returns_err() {
        // Simulating error response when client is not authenticated / 404
        let fixture = r#"{
            "errorCode": "RESOURCE_NOT_FOUND",
            "httpStatus": 404,
            "message": "Summoner not found"
        }"#;

        let result = parse_summoner_info(fixture);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_summoner_value_from_json_value() {
        let val = serde_json::json!({
            "summonerId": 55667788,
            "gameName": "Taker",
            "tagLine": "KR1",
            "displayName": "Taker",
            "summonerLevel": 600,
            "profileIconId": 123
        });

        let result = parse_summoner_value(&val);
        assert!(result.is_ok());

        let summoner = result.unwrap();
        assert_eq!(summoner.summoner_id, 55667788);
        assert_eq!(summoner.game_name, "Taker");
        assert_eq!(summoner.tag_line, "KR1");
        assert_eq!(summoner.summoner_level, 600);
        assert_eq!(summoner.profile_icon_id, 123);
    }

    #[test]
    fn test_connect_lcu_handles_closed_client_gracefully() {
        // Unless LeagueClientUx.exe is actively running on this machine during test execution,
        // LcuConnection::connect should return an Err result without panicking.
        let connection_res = LcuConnection::connect();
        match connection_res {
            Ok(_) => println!("LCU process was detected during test execution."),
            Err(err) => {
                assert!(!err.is_empty());
                println!(
                    "LCU was not detected (expected when client is closed): {}",
                    err
                );
            }
        }
    }
}

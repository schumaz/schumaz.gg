//! Module 1: Connection and Authentication with the LCU (LeagueClientUx.exe)
//! Handles locating the client, authenticating via Basic Auth, and performing REST calls.

use irelia::rest::LcuClient;
use serde::{Deserialize, Serialize};

pub const CURRENT_SUMMONER_ENDPOINT: &str = "/lol-summoner/v1/current-summoner";

/// Represents basic summoner details parsed from `/lol-summoner/v1/current-summoner`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SummonerInfo {
    #[serde(rename = "summonerId")]
    pub summoner_id: u64,
    #[serde(rename = "gameName", default)]
    pub game_name: String,
    #[serde(rename = "tagLine", default)]
    pub tag_line: String,
    #[serde(rename = "displayName", default)]
    pub display_name: String,
    #[serde(rename = "summonerLevel")]
    pub summoner_level: u32,
    #[serde(rename = "profileIconId")]
    pub profile_icon_id: u32,
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
        let response: Option<serde_json::Value> = self
            .client
            .get(CURRENT_SUMMONER_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting current summoner: {:?}", err))?;

        match response {
            Some(val) => parse_summoner_value(&val),
            None => Err("LCU returned an empty response for current summoner".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

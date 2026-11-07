//! Module 1: Connection and Authentication with the LCU (LeagueClientUx.exe)
//! Handles locating the client, authenticating via Basic Auth, and performing REST calls.

use serde::{Deserialize, Serialize};

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
}

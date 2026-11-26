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
fn test_build_match_history_page_endpoint_builds_inclusive_windows_and_clamps() {
    assert_eq!(
        build_match_history_page_endpoint(0, 9),
        "/lol-match-history/v1/products/lol/current-summoner/matches?begIndex=0&endIndex=9"
    );
    assert_eq!(
        build_match_history_page_endpoint(10, 19),
        "/lol-match-history/v1/products/lol/current-summoner/matches?begIndex=10&endIndex=19"
    );
    assert_eq!(
        build_match_history_page_endpoint(20, 29),
        "/lol-match-history/v1/products/lol/current-summoner/matches?begIndex=20&endIndex=29"
    );
    assert_eq!(
        build_match_history_page_endpoint(15, 5),
        "/lol-match-history/v1/products/lol/current-summoner/matches?begIndex=15&endIndex=15"
    );
}

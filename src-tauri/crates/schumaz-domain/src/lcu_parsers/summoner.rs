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

#[cfg(test)]
#[path = "summoner_tests.rs"]
mod tests;

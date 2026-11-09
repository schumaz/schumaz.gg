use super::client::OPGG_REST_API_BASE;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpggPositionInfo {
    pub name: String,
    pub play: u64,
    pub role_rate: f64,
}

pub type ChampionPositionsMap = HashMap<i32, Vec<OpggPositionInfo>>;
pub type PositionsCache = HashMap<String, ChampionPositionsMap>;

// In-memory cache for OP.GG champion rankings by (region, tier, version)
static CHAMPION_POSITIONS_CACHE: std::sync::LazyLock<std::sync::RwLock<PositionsCache>> =
    std::sync::LazyLock::new(|| std::sync::RwLock::new(HashMap::new()));

/// Normalizes an OP.GG position name ("TOP", "JUNGLE", "MID", "ADC", "SUPPORT") into standard lowercase lane.
pub fn normalize_opgg_lane_name(raw: &str) -> Option<&'static str> {
    match raw.to_uppercase().as_str() {
        "TOP" => Some("top"),
        "JUNGLE" | "JNG" => Some("jungle"),
        "MID" | "MIDDLE" => Some("mid"),
        "ADC" | "BOTTOM" | "BOT" => Some("adc"),
        "SUPPORT" | "SUPP" | "UTILITY" => Some("support"),
        _ => None,
    }
}

/// Given a list of OP.GG positions for a champion, returns the lane with the highest game count (`stats.play`).
pub fn select_most_played_lane(positions: &[OpggPositionInfo]) -> Option<&'static str> {
    positions
        .iter()
        .filter_map(|p| {
            let lane = normalize_opgg_lane_name(&p.name)?;
            Some((lane, p.play))
        })
        .max_by_key(|&(_, play)| play)
        .map(|(lane, _)| lane)
}

/// Extracts champion position statistics from the OP.GG champions/ranked JSON payload.
pub fn parse_opgg_champion_positions_ranking(
    json: &serde_json::Value,
) -> HashMap<i32, Vec<OpggPositionInfo>> {
    let mut map = HashMap::new();
    if let Some(data_arr) = json.get("data").and_then(|d| d.as_array()) {
        for champ in data_arr {
            if let Some(champ_id) = champ.get("id").and_then(|v| v.as_i64()).map(|id| id as i32) {
                let mut positions = Vec::new();
                if let Some(pos_arr) = champ.get("positions").and_then(|v| v.as_array()) {
                    for pos in pos_arr {
                        if let Some(name) = pos.get("name").and_then(|v| v.as_str()) {
                            let play = pos
                                .pointer("/stats/play")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            let role_rate = pos
                                .pointer("/stats/role_rate")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0);
                            positions.push(OpggPositionInfo {
                                name: name.to_string(),
                                play,
                                role_rate,
                            });
                        }
                    }
                }
                map.insert(champ_id, positions);
            }
        }
    }
    map
}

/// Resolves the champion's primary lane dynamically by querying OP.GG's ranking data
/// and picking the position with the highest game count (play count).
pub async fn resolve_champion_most_played_lane(
    client: &reqwest::Client,
    champion_id: i32,
    region: &str,
    tier: &str,
    version: Option<&str>,
) -> Result<String, String> {
    let cache_key = format!("{}:{}:{}", region, tier, version.unwrap_or("latest"));

    // 1. Check cache first
    if let Ok(guard) = CHAMPION_POSITIONS_CACHE.read()
        && let Some(champ_map) = guard.get(&cache_key)
        && let Some(positions) = champ_map.get(&champion_id)
        && let Some(lane) = select_most_played_lane(positions)
    {
        return Ok(lane.to_string());
    }

    // 2. Fetch from OP.GG
    let mut url = format!(
        "{}/{}/champions/ranked?tier={}",
        OPGG_REST_API_BASE, region, tier
    );
    if let Some(v) = version
        && !v.trim().is_empty()
    {
        url.push_str(&format!("&version={}", v.trim()));
    }

    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to request champions/ranked from OP.GG: {}", e))?;

    if !res.status().is_success() {
        return Err(format!(
            "OP.GG champions/ranked returned status: {}",
            res.status()
        ));
    }

    let json_body: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse OP.GG champions/ranked JSON: {}", e))?;

    let parsed_map = parse_opgg_champion_positions_ranking(&json_body);

    // 3. Find lane for requested champion
    let chosen_lane = parsed_map
        .get(&champion_id)
        .and_then(|positions| select_most_played_lane(positions))
        .map(|s| s.to_string());

    // 4. Update cache
    if let Ok(mut guard) = CHAMPION_POSITIONS_CACHE.write() {
        guard.insert(cache_key, parsed_map);
    }

    chosen_lane.ok_or_else(|| {
        format!(
            "Champion ID {} has no active positions in OP.GG ranked stats",
            champion_id
        )
    })
}


#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_opgg_champion_positions_ranking_and_select_most_played_lane() {
        let json = serde_json::json!({
            "data": [
                {
                    "id": 157, // Yasuo
                    "positions": [
                        { "name": "MID", "stats": { "play": 57307, "role_rate": 0.60 } },
                        { "name": "TOP", "stats": { "play": 22898, "role_rate": 0.24 } },
                        { "name": "ADC", "stats": { "play": 14609, "role_rate": 0.15 } }
                    ]
                },
                {
                    "id": 79, // Gragas
                    "positions": [
                        { "name": "MID", "stats": { "play": 3491, "role_rate": 0.12 } },
                        { "name": "TOP", "stats": { "play": 16785, "role_rate": 0.60 } },
                        { "name": "JUNGLE", "stats": { "play": 5731, "role_rate": 0.20 } }
                    ]
                },
                {
                    "id": 147, // Seraphine
                    "positions": [
                        { "name": "SUPPORT", "stats": { "play": 44180, "role_rate": 0.91 } }
                    ]
                }
            ]
        });

        let map = parse_opgg_champion_positions_ranking(&json);
        assert_eq!(map.len(), 3);

        // Yasuo should pick Mid (57k > 22k > 14k)
        let yasuo_pos = map.get(&157).unwrap();
        assert_eq!(select_most_played_lane(yasuo_pos), Some("mid"));

        // Gragas should pick Top (16k > 5k > 3k)
        let gragas_pos = map.get(&79).unwrap();
        assert_eq!(select_most_played_lane(gragas_pos), Some("top"));

        // Seraphine should pick Support
        let sera_pos = map.get(&147).unwrap();
        assert_eq!(select_most_played_lane(sera_pos), Some("support"));
    }

    #[tokio::test]
    async fn test_resolve_champion_most_played_lane_cached() {
        let champ_id = 99999;
        let positions = vec![
            OpggPositionInfo {
                name: "TOP".to_string(),
                play: 100,
                role_rate: 0.1,
            },
            OpggPositionInfo {
                name: "ADC".to_string(),
                play: 9500,
                role_rate: 0.9,
            },
        ];

        let mut inner_map = HashMap::new();
        inner_map.insert(champ_id, positions);

        let cache_key = "global:emerald_plus:test_v1".to_string();
        if let Ok(mut guard) = CHAMPION_POSITIONS_CACHE.write() {
            guard.insert(cache_key, inner_map);
        }

        let dummy_client = reqwest::Client::new();
        let resolved = resolve_champion_most_played_lane(
            &dummy_client,
            champ_id,
            "global",
            "emerald_plus",
            Some("test_v1"),
        )
        .await;

        assert_eq!(resolved, Ok("adc".to_string()));
    }
}

use super::client::OPGG_REST_API_BASE;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

pub fn positions_file_path(dir: &Path, region: &str, tier: &str, version: Option<&str>) -> PathBuf {
    let reg = crate::storage::sanitize_key_component(region);
    let tr = crate::storage::sanitize_key_component(tier);
    let ver = crate::storage::sanitize_key_component(version.unwrap_or("latest"));
    dir.join(format!("positions_{reg}_{tr}_{ver}.lz4"))
}

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

pub async fn resolve_champion_most_played_lane_with_base_and_dir(
    client: &reqwest::Client,
    rest_base: &str,
    champion_id: i32,
    region: &str,
    tier: &str,
    version: Option<&str>,
    positions_dir: Option<&Path>,
) -> Result<String, String> {
    let cache_key = format!(
        "{}:{}:{}:{}",
        rest_base,
        region,
        tier,
        version.unwrap_or("latest")
    );
    let legacy_cache_key = format!("{}:{}:{}", region, tier, version.unwrap_or("latest"));

    // 1. Check L1 RAM cache first
    if let Ok(guard) = CHAMPION_POSITIONS_CACHE.read()
        && let Some(champ_map) = guard
            .get(&cache_key)
            .or_else(|| guard.get(&legacy_cache_key))
        && let Some(positions) = champ_map.get(&champion_id)
        && let Some(lane) = select_most_played_lane(positions)
    {
        return Ok(lane.to_string());
    }

    // 2. Check L2 disk .lz4 cache (24h TTL + patch check)
    let disk_env = positions_dir.and_then(|dir| {
        let path = positions_file_path(dir, region, tier, version);
        crate::storage::read_lz4_envelope::<ChampionPositionsMap>(&path)
    });

    if let Some(ref env) = disk_env
        && crate::storage::is_envelope_fresh(
            env.created_at_unix,
            crate::storage::OPGG_CACHE_TTL_SECS,
            crate::storage::now_unix_secs(),
        )
    {
        let patch_ok = match version.map(str::trim).filter(|v| !v.is_empty()) {
            Some(v) => env.patch == v,
            None => true,
        };
        if patch_ok {
            let chosen = env
                .payload
                .get(&champion_id)
                .and_then(|positions| select_most_played_lane(positions))
                .map(|s| s.to_string());
            if let Ok(mut guard) = CHAMPION_POSITIONS_CACHE.write() {
                guard.insert(cache_key.clone(), env.payload.clone());
            }
            if let Some(lane) = chosen {
                return Ok(lane);
            }
        }
    }

    // 3. Fetch from OP.GG REST API
    let mut url = format!(
        "{}/{}/champions/ranked?tier={}",
        rest_base.trim_end_matches('/'),
        region,
        tier
    );
    if let Some(v) = version
        && !v.trim().is_empty()
    {
        url.push_str(&format!("&version={}", v.trim()));
    }

    let fetch_res = async {
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

        Ok::<serde_json::Value, String>(json_body)
    }
    .await;

    let json_body = match fetch_res {
        Ok(json) => json,
        Err(err) => {
            // Stale-While-Error offline fallback
            if let Some(env) = disk_env
                && let Some(lane) = env
                    .payload
                    .get(&champion_id)
                    .and_then(|positions| select_most_played_lane(positions))
                    .map(|s| s.to_string())
            {
                if let Ok(mut guard) = CHAMPION_POSITIONS_CACHE.write() {
                    guard.insert(cache_key, env.payload);
                }
                return Ok(lane);
            }
            return Err(err);
        }
    };

    let parsed_map = parse_opgg_champion_positions_ranking(&json_body);

    let chosen_lane = parsed_map
        .get(&champion_id)
        .and_then(|positions| select_most_played_lane(positions))
        .map(|s| s.to_string());

    if let Some(dir) = positions_dir
        && !parsed_map.is_empty()
    {
        let path = positions_file_path(dir, region, tier, version);
        let patch_str = version.unwrap_or("").trim();
        let _ = crate::storage::write_lz4_envelope(&path, patch_str, &parsed_map);
    }

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

/// Resolves the champion's primary lane dynamically by querying OP.GG's ranking data
/// and picking the position with the highest game count (play count).
pub async fn resolve_champion_most_played_lane(
    client: &reqwest::Client,
    champion_id: i32,
    region: &str,
    tier: &str,
    version: Option<&str>,
) -> Result<String, String> {
    let dir = crate::storage::global_paths().map(|p| p.opgg_positions_dir.as_path());
    resolve_champion_most_played_lane_with_base_and_dir(
        client,
        OPGG_REST_API_BASE,
        champion_id,
        region,
        tier,
        version,
        dir,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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

        let yasuo_pos = map.get(&157).unwrap();
        assert_eq!(select_most_played_lane(yasuo_pos), Some("mid"));

        let gragas_pos = map.get(&79).unwrap();
        assert_eq!(select_most_played_lane(gragas_pos), Some("top"));

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

    #[tokio::test]
    async fn test_positions_disk_cache_and_stale_while_error() {
        let dir =
            std::env::temp_dir().join(format!("schumazgg_positions_cache_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/global/champions/ranked"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    { "id": 103, "positions": [{ "name": "MID", "stats": { "play": 50000, "role_rate": 0.9 } }] }
                ]
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let lane = resolve_champion_most_played_lane_with_base_and_dir(
            &client,
            &mock_server.uri(),
            103,
            "global",
            "emerald_plus",
            Some("16.25"),
            Some(&dir),
        )
        .await
        .unwrap();
        assert_eq!(lane, "mid");
        assert!(positions_file_path(&dir, "global", "emerald_plus", Some("16.25")).exists());

        // Clear RAM and test expired disk envelope with offline server -> Stale-While-Error rescues!
        if let Ok(mut g) = CHAMPION_POSITIONS_CACHE.write() {
            g.clear();
        }
        let disk_file = positions_file_path(&dir, "global", "emerald_plus", Some("16.25"));
        let env = crate::storage::read_lz4_envelope::<ChampionPositionsMap>(&disk_file).unwrap();
        crate::storage::write_lz4_envelope_at(&disk_file, "16.25", &env.payload, 1).unwrap();

        let offline = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/global/champions/ranked"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&offline)
            .await;

        let rescued = resolve_champion_most_played_lane_with_base_and_dir(
            &client,
            &offline.uri(),
            103,
            "global",
            "emerald_plus",
            Some("16.25"),
            Some(&dir),
        )
        .await
        .unwrap();
        assert_eq!(rescued, "mid");

        let _ = std::fs::remove_dir_all(&dir);
    }
}

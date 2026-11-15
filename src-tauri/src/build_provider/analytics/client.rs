use super::super::ddragon::enrich_build_with_ddragon;
use super::super::fallback::{
    build_fallback_champion_build, champion_key_to_opgg_name, opgg_queue_param_from_id,
};
use super::super::models::{ChampionBuild, DataDragonCache, ItemStatEntry};
use super::mcp_extractors::extract_mcp_grouped_item_lists;
use super::rest_parser::parse_opgg_analysis_payload_with_options;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

pub const OPGG_MCP_ENDPOINT: &str = "https://mcp-api.op.gg/mcp";
pub const OPGG_REST_API_BASE: &str = "https://lol-api-champion.op.gg/api";

type VersionsCacheMap = HashMap<String, (Instant, Vec<String>)>;

static SHARED_HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static OPGG_VERSIONS_CACHE: OnceLock<RwLock<VersionsCacheMap>> = OnceLock::new();

const VERSIONS_CACHE_TTL: Duration = Duration::from_secs(600);

/// Returns a shared, connection-pooled `reqwest::Client` with bounded timeouts.
pub fn shared_http_client() -> &'static reqwest::Client {
    SHARED_HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .pool_max_idle_per_host(8)
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(10))
            .tcp_keepalive(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

fn get_versions_cache() -> &'static RwLock<VersionsCacheMap> {
    OPGG_VERSIONS_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn opgg_versions_file_path(dir: &Path, region: &str) -> PathBuf {
    let reg = crate::storage::sanitize_key_component(region);
    dir.join(format!("versions_{reg}.lz4"))
}

/// Non-blocking peek at the latest known OP.GG patch for `region` from RAM or disk `.lz4`.
pub fn peek_latest_cached_opgg_version(region: &str) -> Option<String> {
    let cache_key = format!("{}:{}", OPGG_REST_API_BASE, region);
    if let Ok(guard) = get_versions_cache().read()
        && let Some((_, vers)) = guard.get(&cache_key)
        && let Some(first) = vers.first()
    {
        return Some(first.clone());
    }
    if let Some(paths) = crate::storage::global_paths() {
        let path = opgg_versions_file_path(&paths.opgg_versions_dir, region);
        if let Some(env) = crate::storage::read_lz4_envelope::<Vec<String>>(&path) {
            return env.payload.first().cloned();
        }
    }
    None
}

pub async fn fetch_opgg_champion_analysis(
    client: &reqwest::Client,
    endpoint_url: &str,
    champion_name: &str,
    queue_name: &str,
) -> Result<serde_json::Value, String> {
    let formatted_champ = champion_key_to_opgg_name(champion_name);
    // NOTE: OP.GG MCP `lol_get_champion_analysis` schema strictly requires `position`
    // (`top|mid|jungle|adc|support`) even when `game_mode == "aram"` (omitting it or passing
    // "none" fails with JSON-RPC -32600).
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "lol_get_champion_analysis",
            "arguments": {
                "game_mode": queue_name,
                "champion": formatted_champ,
                "position": "mid",
                "desired_output_fields": [
                    "data.damage_type",
                    "data.summary.average_stats.{win_rate,pick_rate,play}",
                    "data.runes[].{primary_page_id,primary_rune_ids[],secondary_page_id,secondary_rune_ids[],stat_mod_ids[],play,win}",
                    "data.summoner_spells.{ids[],play,win}",
                    "data.starter_items.{ids[],play,win,pick_rate}",
                    "data.core_items.{ids[],play,win,pick_rate}",
                    "data.boots[].{ids[],play,win,pick_rate}",
                    "data.last_items[].{ids[],play,win,pick_rate}",
                    "data.fourth_items[].{ids[],play,win,pick_rate}",
                    "data.fifth_items[].{ids[],play,win,pick_rate}",
                    "data.sixth_items[].{ids[],play,win,pick_rate}"
                ]
            }
        }
    });

    let res = client
        .post(endpoint_url)
        .json(&payload)
        .send()
        .await
        .map_err(|err| format!("HTTP request to OP.GG failed: {}", err))?;

    if !res.status().is_success() {
        return Err(format!("OP.GG responded with HTTP error: {}", res.status()));
    }

    let json_body: serde_json::Value = res
        .json()
        .await
        .map_err(|err| format!("Failed to parse JSON response from OP.GG: {}", err))?;

    Ok(json_body)
}

/// Helper function to fetch champion analysis and return JSON body directly.
pub async fn fetch_opgg_analysis_json(
    client: &reqwest::Client,
    champion_name: &str,
    queue: &str,
) -> Result<serde_json::Value, String> {
    let json_body =
        fetch_opgg_champion_analysis(client, OPGG_MCP_ENDPOINT, champion_name, queue).await?;
    Ok(json_body)
}

/// High-level function that fetches the recommended build for a champion with automatic resilient fallback.
pub async fn get_recommended_build_with_endpoint(
    client: &reqwest::Client,
    endpoint_url: &str,
    champion_id: i32,
    queue_id: i32,
    ddragon_cache: &DataDragonCache,
) -> ChampionBuild {
    let champ_name = ddragon_cache
        .champions
        .get(&champion_id)
        .map(|c| c.key_name.as_str())
        .unwrap_or("Unknown");

    let queue_param = opgg_queue_param_from_id(queue_id);
    let supports_runes = queue_id != crate::ws_listener::QUEUE_ARAM_MAYHEM;

    let mut build =
        match fetch_opgg_champion_analysis(client, endpoint_url, champ_name, queue_param).await {
            Ok(json_res) => match parse_opgg_analysis_payload_with_options(
                &json_res,
                champion_id,
                champ_name,
                supports_runes,
            ) {
                Ok(build) => build,
                Err(_) => build_fallback_champion_build(champion_id, champ_name, queue_id),
            },
            Err(_) => build_fallback_champion_build(champion_id, champ_name, queue_id),
        };

    if queue_param == "aram" {
        build.active_position.clear();
    }
    build.supports_runes = supports_runes;
    enrich_build_with_ddragon(&mut build, ddragon_cache);
    build
}

/// Fetches available patch versions from the OP.GG REST API with L1 RAM + L2 `.lz4` disk cache (48h TTL)
/// and Stale-While-Error offline fallback.
pub async fn fetch_opgg_versions_with_cache_dir(
    client: &reqwest::Client,
    rest_base: &str,
    region: &str,
    versions_dir: Option<&Path>,
) -> Vec<String> {
    let cache_key = format!("{}:{}", rest_base, region);
    if let Ok(guard) = get_versions_cache().read()
        && let Some((ts, cached)) = guard.get(&cache_key)
        && ts.elapsed() < VERSIONS_CACHE_TTL
    {
        return cached.clone();
    }

    let disk_env = versions_dir.and_then(|dir| {
        let path = opgg_versions_file_path(dir, region);
        crate::storage::read_lz4_envelope::<Vec<String>>(&path)
    });

    if let Some(ref env) = disk_env
        && !env.payload.is_empty()
        && crate::storage::is_envelope_fresh(
            env.created_at_unix,
            crate::storage::OPGG_CACHE_TTL_SECS,
            crate::storage::now_unix_secs(),
        )
    {
        if let Ok(mut guard) = get_versions_cache().write() {
            guard.insert(cache_key.clone(), (Instant::now(), env.payload.clone()));
        }
        return env.payload.clone();
    }

    let url = format!("{}/{}/champions/ranked/versions", rest_base, region);
    if let Ok(res) = client.get(&url).send().await
        && let Ok(json) = res.json::<serde_json::Value>().await
        && let Some(arr) = json.get("data").and_then(|d| d.as_array())
    {
        let vers: Vec<String> = arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        if !vers.is_empty() {
            if let Ok(mut guard) = get_versions_cache().write() {
                guard.insert(cache_key, (Instant::now(), vers.clone()));
            }
            if let Some(dir) = versions_dir {
                let path = opgg_versions_file_path(dir, region);
                let latest = vers.first().map(String::as_str).unwrap_or("");
                let _ = crate::storage::write_lz4_envelope(&path, latest, &vers);
            }
            return vers;
        }
    }

    // Stale-While-Error offline fallback: return expired `.lz4` versions if present
    if let Some(env) = disk_env
        && !env.payload.is_empty()
    {
        if let Ok(mut guard) = get_versions_cache().write() {
            guard.insert(cache_key, (Instant::now(), env.payload.clone()));
        }
        return env.payload;
    }

    vec!["16.22".to_string(), "16.21".to_string()]
}

/// Fetches available patch versions from the OP.GG REST API with in-memory TTL + disk caching.
pub async fn fetch_opgg_versions_from_base(
    client: &reqwest::Client,
    rest_base: &str,
    region: &str,
) -> Vec<String> {
    let dir = if rest_base == OPGG_REST_API_BASE {
        crate::storage::global_paths().map(|p| p.opgg_versions_dir.as_path())
    } else {
        None
    };
    fetch_opgg_versions_with_cache_dir(client, rest_base, region, dir).await
}

/// Fetches available patch versions from the OP.GG REST API.
pub async fn fetch_opgg_versions(client: &reqwest::Client, region: &str) -> Vec<String> {
    fetch_opgg_versions_from_base(client, OPGG_REST_API_BASE, region).await
}

/// Fetches champion build analysis from the OP.GG REST API with full filter parameters.
/// When `mode == "aram"` or `position` is `None`/empty, uses `"none"` because the OP.GG REST
/// route `/{region}/champions/{mode}/{champion_id}/{position}` requires a path segment.
pub async fn fetch_opgg_rest_champion_analysis(
    client: &reqwest::Client,
    region: &str,
    mode: &str,
    champion_id: i32,
    position: Option<&str>,
    tier: &str,
    version: Option<&str>,
) -> Result<serde_json::Value, String> {
    fetch_opgg_rest_champion_analysis_from_base(
        client,
        OPGG_REST_API_BASE,
        region,
        mode,
        champion_id,
        position,
        tier,
        version,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn fetch_opgg_rest_champion_analysis_from_base(
    client: &reqwest::Client,
    rest_base: &str,
    region: &str,
    mode: &str,
    champion_id: i32,
    position: Option<&str>,
    tier: &str,
    version: Option<&str>,
) -> Result<serde_json::Value, String> {
    let pos_segment = if mode.eq_ignore_ascii_case("aram") {
        "none"
    } else {
        match position {
            Some(p) if !p.trim().is_empty() => p.trim(),
            _ => "none",
        }
    };

    let mut url = format!(
        "{}/{}/champions/{}/{}/{}?tier={}",
        rest_base, region, mode, champion_id, pos_segment, tier
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
        .map_err(|e| format!("REST request to OP.GG failed: {}", e))?;

    if !res.status().is_success() {
        return Err(format!(
            "OP.GG REST responded with HTTP error: {}",
            res.status()
        ));
    }

    let json_body: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse JSON response from OP.GG REST: {}", e))?;

    Ok(json_body)
}

/// Fetches separate `fourth_items`, `fifth_items`, and `sixth_items` lists from OP.GG MCP.
pub async fn fetch_opgg_mcp_depth_items(
    client: &reqwest::Client,
    endpoint_url: &str,
    champion_name: &str,
    game_mode: &str,
    position: Option<&str>,
    tier: &str,
) -> Result<(Vec<ItemStatEntry>, Vec<ItemStatEntry>, Vec<ItemStatEntry>), String> {
    let formatted_champ = champion_key_to_opgg_name(champion_name);
    // OP.GG MCP `lol_get_champion_analysis` strictly requires `position` in its schema
    // (`top|mid|jungle|adc|support`) even for `game_mode == "aram"` (omitting it or passing
    // "none" fails with JSON-RPC -32600, while any valid enum value returns global ARAM data).
    let mcp_position = if game_mode.eq_ignore_ascii_case("aram") {
        "mid"
    } else {
        match position {
            Some(p) if !p.trim().is_empty() => p.trim(),
            _ => "mid",
        }
    };
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "lol_get_champion_analysis",
            "arguments": {
                "game_mode": game_mode,
                "champion": formatted_champ,
                "position": mcp_position,
                "tier": tier,
                "desired_output_fields": [
                    "data.fourth_items[].{ids[],play,win,pick_rate}",
                    "data.fifth_items[].{ids[],play,win,pick_rate}",
                    "data.sixth_items[].{ids[],play,win,pick_rate}"
                ]
            }
        }
    });

    let res = client
        .post(endpoint_url)
        .json(&payload)
        .send()
        .await
        .map_err(|err| format!("MCP depth items request failed: {}", err))?;

    if !res.status().is_success() {
        return Err(format!("MCP depth items HTTP error: {}", res.status()));
    }

    let json_body: serde_json::Value = res
        .json()
        .await
        .map_err(|err| format!("Failed to parse MCP depth items JSON: {}", err))?;

    let text = json_body
        .pointer("/result/content/0/text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing text in MCP depth items response".to_string())?;

    let grouped = extract_mcp_grouped_item_lists(text);
    let mut fourth = Vec::new();
    let mut fifth = Vec::new();
    let mut sixth = Vec::new();
    for (name, items) in grouped {
        match name.as_str() {
            "fourth_items" => fourth = items,
            "fifth_items" => fifth = items,
            "sixth_items" => sixth = items,
            _ => {}
        }
    }
    Ok((fourth, fifth, sixth))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_fetch_opgg_rest_uses_none_for_aram_ignoring_position() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/global/champions/aram/103/none"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": { "ok": true }
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let res = fetch_opgg_rest_champion_analysis_from_base(
            &client,
            &mock_server.uri(),
            "global",
            "aram",
            103,
            Some("top"),
            "emerald_plus",
            Some("16.22"),
        )
        .await;

        assert!(res.is_ok(), "ARAM REST call should hit /aram/103/none");
        assert_eq!(res.unwrap()["data"]["ok"], serde_json::json!(true));
    }

    #[tokio::test]
    async fn test_fetch_opgg_versions_disk_cache_and_stale_while_error() {
        let dir =
            std::env::temp_dir().join(format!("schumazgg_versions_cache_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/global/champions/ranked/versions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": ["16.25", "16.24"]
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let v1 =
            fetch_opgg_versions_with_cache_dir(&client, &mock_server.uri(), "global", Some(&dir))
                .await;
        assert_eq!(v1, vec!["16.25", "16.24"]);
        let disk_file = opgg_versions_file_path(&dir, "global");
        assert!(disk_file.exists());

        // Clear RAM cache and verify expired disk envelope still rescues offline failure
        if let Ok(mut g) = get_versions_cache().write() {
            g.clear();
        }
        crate::storage::write_lz4_envelope_at(
            &disk_file,
            "16.25",
            &vec!["16.25".to_string(), "16.24".to_string()],
            1, // very old timestamp -> expired TTL
        )
        .unwrap();

        let offline_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/global/champions/ranked/versions"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&offline_server)
            .await;

        let stale_fallback = fetch_opgg_versions_with_cache_dir(
            &client,
            &offline_server.uri(),
            "global",
            Some(&dir),
        )
        .await;
        assert_eq!(stale_fallback, vec!["16.25", "16.24"]);

        let _ = std::fs::remove_dir_all(&dir);
    }
}

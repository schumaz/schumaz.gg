use super::aram::normalize_opgg_champion_key;
use super::client::{
    OPGG_REST_API_BASE, fetch_opgg_rest_champion_analysis_from_base, fetch_opgg_versions_from_base,
};
use crate::build_provider::models::DataDragonCache;
use crate::build_provider::patch::resolve_effective_patch;
use crate::models::{RankedChampionEntry, RankedLane, RankedTierListResponse};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

const RANKED_CACHE_TTL: Duration = Duration::from_secs(900);
const TREND_PROBE_CHAMPION_ID: i32 = 103; // Ahri (high play-rate reference for patch release timestamp)

type RankedCacheMap = HashMap<String, (Instant, RankedTierListResponse)>;

static RANKED_TIER_CACHE: OnceLock<RwLock<RankedCacheMap>> = OnceLock::new();

fn get_ranked_cache() -> &'static RwLock<RankedCacheMap> {
    RANKED_TIER_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn round_pct(val: f64) -> f64 {
    let pct = if val <= 1.0 { val * 100.0 } else { val };
    (pct * 10.0).round() / 10.0
}

fn normalize_raw_pct(val: Option<f64>) -> f64 {
    match val {
        Some(v) if v.is_finite() => {
            if v <= 1.0 {
                v * 100.0
            } else {
                v
            }
        }
        _ => -1.0,
    }
}

pub fn parse_ranked_lane(raw: &str) -> Option<RankedLane> {
    match raw.trim().to_uppercase().as_str() {
        "TOP" => Some(RankedLane::Top),
        "JUNGLE" | "JNG" => Some(RankedLane::Jungle),
        "MID" | "MIDDLE" => Some(RankedLane::Mid),
        "ADC" | "BOTTOM" | "BOT" => Some(RankedLane::Adc),
        "SUPPORT" | "SUPP" | "UTILITY" => Some(RankedLane::Support),
        _ => None,
    }
}

fn lane_sort_order(lane: RankedLane) -> u8 {
    match lane {
        RankedLane::Top => 0,
        RankedLane::Jungle => 1,
        RankedLane::Mid => 2,
        RankedLane::Adc => 3,
        RankedLane::Support => 4,
    }
}

#[derive(Debug, Deserialize)]
struct RestRankedResponse {
    #[serde(default)]
    data: Vec<RestRankedChampionItem>,
    #[serde(default)]
    meta: Option<RestRankedMeta>,
}

#[derive(Debug, Deserialize)]
struct RestRankedMeta {
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    match_count: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct RestRankedChampionItem {
    id: i32,
    #[serde(default)]
    average_stats: Option<RestRankedTierFallbackStats>,
    #[serde(default)]
    positions: Vec<RestRankedPositionItem>,
}

#[derive(Debug, Deserialize)]
struct RestRankedTierFallbackStats {
    #[serde(default)]
    tier: Option<i32>,
    #[serde(default)]
    play: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct RestRankedPositionItem {
    name: String,
    #[serde(default)]
    stats: Option<RestRankedPositionStats>,
}

#[derive(Debug, Deserialize)]
struct RestRankedPositionStats {
    #[serde(default)]
    win_rate: Option<f64>,
    #[serde(default)]
    pick_rate: Option<f64>,
    #[serde(default)]
    ban_rate: Option<f64>,
    #[serde(default)]
    role_rate: Option<f64>,
    #[serde(default)]
    tier_data: Option<RestRankedTierData>,
}

#[derive(Debug, Deserialize)]
struct RestRankedTierData {
    #[serde(default)]
    tier: Option<i32>,
    #[serde(default)]
    rank: Option<i32>,
}

struct ParsedEntryWithPrecision {
    entry: RankedChampionEntry,
    raw_win_rate: f64,
    raw_pick_rate: f64,
}

pub fn parse_ranked_tier_list(
    json: &serde_json::Value,
    ddragon: &DataDragonCache,
) -> Result<Vec<RankedChampionEntry>, String> {
    let parsed: RestRankedResponse = serde_json::from_value(json.clone())
        .map_err(|e| format!("Failed to parse ranked tier list response: {}", e))?;

    let mut items: Vec<ParsedEntryWithPrecision> = Vec::new();

    for champ in parsed.data {
        let dd_meta = ddragon.champions.get(&champ.id);
        let name = dd_meta
            .map(|m| m.display_name.clone())
            .unwrap_or_else(|| format!("Champion {}", champ.id));
        let key = dd_meta
            .map(|m| normalize_opgg_champion_key(&m.key_name))
            .unwrap_or_else(|| champ.id.to_string());
        let icon_url = dd_meta.map(|m| m.icon_url.clone()).unwrap_or_default();
        let fallback_tier = champ.average_stats.as_ref().and_then(|s| s.tier);

        for pos in champ.positions {
            let Some(lane) = parse_ranked_lane(&pos.name) else {
                continue;
            };
            let Some(stats) = pos.stats else {
                continue;
            };

            let raw_tier = stats
                .tier_data
                .as_ref()
                .and_then(|td| td.tier)
                .or(fallback_tier)
                .unwrap_or(6);
            // OP.GG ranked tier starts at 1 (1 = OP, 2 = Tier 1, 3 = Tier 2, ...).
            // Normalize to 0-based canonical tiers (0 = OP, 1 = Tier 1, ..., 5 = Tier 5).
            let normalized_tier = raw_tier.saturating_sub(1);
            let rank = stats
                .tier_data
                .as_ref()
                .and_then(|td| td.rank)
                .unwrap_or(999);

            let raw_win_rate = normalize_raw_pct(stats.win_rate);
            let raw_pick_rate = normalize_raw_pct(stats.pick_rate);

            items.push(ParsedEntryWithPrecision {
                entry: RankedChampionEntry {
                    id: champ.id,
                    key: key.clone(),
                    name: name.clone(),
                    icon_url: icon_url.clone(),
                    lane,
                    tier: normalized_tier,
                    rank,
                    overall_rank: 0,
                    win_rate: stats.win_rate.map(round_pct),
                    pick_rate: stats.pick_rate.map(round_pct),
                    ban_rate: stats.ban_rate.map(round_pct),
                    role_rate: stats.role_rate.map(round_pct),
                },
                raw_win_rate,
                raw_pick_rate,
            });
        }
    }

    // Global cross-lane OP ranking (matches OP.GG's "All Roles" tier list ordering):
    // 1. Canonical tier ascending (OP -> Tier 1 -> Tier 2 ...)
    // 2. Full-precision win rate descending
    // 3. Full-precision pick rate descending
    // 4. Per-lane rank ascending
    // 5. Champion name & lane order for deterministic tie-breaking
    items.sort_by(|a, b| {
        let a_valid_rank = if a.entry.rank > 0 { 0 } else { 1 };
        let b_valid_rank = if b.entry.rank > 0 { 0 } else { 1 };
        a.entry
            .tier
            .cmp(&b.entry.tier)
            .then_with(|| b.raw_win_rate.total_cmp(&a.raw_win_rate))
            .then_with(|| b.raw_pick_rate.total_cmp(&a.raw_pick_rate))
            .then_with(|| a_valid_rank.cmp(&b_valid_rank))
            .then_with(|| a.entry.rank.cmp(&b.entry.rank))
            .then_with(|| a.entry.name.cmp(&b.entry.name))
            .then_with(|| lane_sort_order(a.entry.lane).cmp(&lane_sort_order(b.entry.lane)))
    });

    let entries = items
        .into_iter()
        .enumerate()
        .map(|(idx, mut item)| {
            item.entry.overall_rank = (idx + 1) as i32;
            item.entry
        })
        .collect();

    Ok(entries)
}

fn extract_ranked_meta_info(json: &serde_json::Value) -> (Option<String>, Option<u64>) {
    let parsed: Result<RestRankedResponse, _> = serde_json::from_value(json.clone());
    match parsed {
        Ok(resp) => {
            let version = resp
                .meta
                .as_ref()
                .and_then(|m| m.version.clone())
                .filter(|v| !v.trim().is_empty());
            let match_count = resp.meta.as_ref().and_then(|m| m.match_count).or_else(|| {
                let max_play = resp
                    .data
                    .iter()
                    .filter_map(|c| c.average_stats.as_ref().and_then(|s| s.play))
                    .max();
                max_play
            });
            (version, match_count)
        }
        Err(_) => (None, None),
    }
}

async fn fetch_ranked_json(
    client: &reqwest::Client,
    rest_base: &str,
    region: &str,
    tier: &str,
    version: Option<&str>,
) -> Result<serde_json::Value, String> {
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

    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch ranked tier list: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Ranked tier list HTTP error: {}", res.status()));
    }

    res.json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse ranked tier list JSON: {}", e))
}

#[allow(clippy::too_many_arguments)]
pub async fn fetch_ranked_tier_list_with_base(
    client: &reqwest::Client,
    rest_base: &str,
    region: Option<&str>,
    tier: Option<&str>,
    patch: Option<&str>,
    ddragon: &DataDragonCache,
    patch_48h_protection: bool,
    now_unix: u64,
) -> Result<RankedTierListResponse, String> {
    let target_region = region
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("global");
    let target_tier = tier
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("emerald_plus");
    let manual_patch = patch.map(str::trim).filter(|s| !s.is_empty());

    let cache_key = format!(
        "{}:{}:{}:{}:{}",
        rest_base,
        target_region,
        target_tier,
        manual_patch.unwrap_or(""),
        patch_48h_protection
    );

    if let Ok(guard) = get_ranked_cache().read()
        && let Some((ts, cached)) = guard.get(&cache_key)
        && ts.elapsed() < RANKED_CACHE_TTL
    {
        return Ok(cached.clone());
    }

    let available_versions = fetch_opgg_versions_from_base(client, rest_base, target_region).await;

    let mut effective_patch = manual_patch
        .map(str::to_string)
        .unwrap_or_else(|| available_versions.first().cloned().unwrap_or_default());
    let mut patch_fallback_applied = false;
    let mut patch_protection_remaining_hours: Option<u32> = None;

    let final_json = if let Some(user_patch) = manual_patch {
        effective_patch = user_patch.to_string();
        fetch_ranked_json(
            client,
            rest_base,
            target_region,
            target_tier,
            Some(user_patch),
        )
        .await?
    } else {
        let probe_version = (!effective_patch.is_empty()).then_some(effective_patch.as_str());
        let probe_json =
            fetch_ranked_json(client, rest_base, target_region, target_tier, probe_version).await?;

        let (meta_version, match_count) = extract_ranked_meta_info(&probe_json);
        if effective_patch.is_empty()
            && let Some(v) = meta_version
        {
            effective_patch = v;
        }

        if patch_48h_protection && available_versions.len() >= 2 && match_count.unwrap_or(0) < 1000
        {
            let probe_analysis = fetch_opgg_rest_champion_analysis_from_base(
                client,
                rest_base,
                target_region,
                "ranked",
                TREND_PROBE_CHAMPION_ID,
                Some("mid"),
                target_tier,
                Some(&effective_patch),
            )
            .await;

            if let Ok(analysis_json) = probe_analysis {
                let created_at_iso = analysis_json
                    .pointer("/data/trends/win/0/created_at")
                    .and_then(|v| v.as_str());
                let prev_patch_created_at_iso = analysis_json
                    .pointer("/data/trends/win/1/created_at")
                    .and_then(|v| v.as_str());

                let selection = resolve_effective_patch(
                    &available_versions,
                    created_at_iso,
                    prev_patch_created_at_iso,
                    match_count,
                    now_unix,
                    None,
                    patch_48h_protection,
                );
                effective_patch = selection.effective_patch;
                patch_fallback_applied = selection.fallback_applied;
                patch_protection_remaining_hours = selection.remaining_hours;

                if patch_fallback_applied {
                    fetch_ranked_json(
                        client,
                        rest_base,
                        target_region,
                        target_tier,
                        Some(&effective_patch),
                    )
                    .await?
                } else {
                    probe_json
                }
            } else {
                probe_json
            }
        } else {
            probe_json
        }
    };

    let champions = parse_ranked_tier_list(&final_json, ddragon)?;
    let response = RankedTierListResponse {
        region: target_region.to_string(),
        tier: target_tier.to_string(),
        active_patch: effective_patch,
        available_patches: available_versions,
        patch_fallback_applied,
        patch_protection_remaining_hours,
        champions,
    };

    if !response.champions.is_empty()
        && let Ok(mut guard) = get_ranked_cache().write()
    {
        guard.insert(cache_key, (Instant::now(), response.clone()));
    }

    Ok(response)
}

pub async fn fetch_ranked_tier_list(
    client: &reqwest::Client,
    region: Option<&str>,
    tier: Option<&str>,
    patch: Option<&str>,
    ddragon: &DataDragonCache,
) -> Result<RankedTierListResponse, String> {
    let prefs = crate::lcu_injector::get_user_preferences();
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    fetch_ranked_tier_list_with_base(
        client,
        OPGG_REST_API_BASE,
        region,
        tier,
        patch,
        ddragon,
        prefs.patch_48h_protection,
        now_unix,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_provider::patch::parse_iso8601_to_unix;
    use crate::models::ChampionMetadata;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_ddragon() -> DataDragonCache {
        let mut champions = HashMap::new();
        champions.insert(
            103,
            ChampionMetadata {
                champion_id: 103,
                key_name: "Ahri".to_string(),
                display_name: "Ahri".to_string(),
                icon_url: "https://ddragon.example/Ahri.png".to_string(),
            },
        );
        champions.insert(
            245,
            ChampionMetadata {
                champion_id: 245,
                key_name: "Ekko".to_string(),
                display_name: "Ekko".to_string(),
                icon_url: "https://ddragon.example/Ekko.png".to_string(),
            },
        );
        champions.insert(
            86,
            ChampionMetadata {
                champion_id: 86,
                key_name: "Garen".to_string(),
                display_name: "Garen".to_string(),
                icon_url: "https://ddragon.example/Garen.png".to_string(),
            },
        );
        DataDragonCache {
            version: "16.22.1".to_string(),
            champions,
            runes: HashMap::new(),
            spells: HashMap::new(),
        }
    }

    #[test]
    fn test_parse_ranked_tier_list_normalizes_tiers_and_assigns_cross_lane_overall_rank() {
        let json = serde_json::json!({
            "data": [
                {
                    "id": 103,
                    "positions": [
                        {
                            "name": "MID",
                            "stats": {
                                "win_rate": 0.512,
                                "pick_rate": 0.115,
                                "ban_rate": 0.036,
                                "role_rate": 0.94,
                                "tier_data": { "tier": 1, "rank": 1 }
                            }
                        }
                    ]
                },
                {
                    "id": 245,
                    "positions": [
                        {
                            "name": "MID",
                            "stats": {
                                "win_rate": 0.522,
                                "pick_rate": 0.053,
                                "ban_rate": 0.034,
                                "role_rate": 0.45,
                                "tier_data": { "tier": 2, "rank": 4 }
                            }
                        },
                        {
                            "name": "JUNGLE",
                            "stats": {
                                "win_rate": 0.508,
                                "pick_rate": 0.061,
                                "ban_rate": 0.034,
                                "role_rate": 0.55,
                                "tier_data": { "tier": 3, "rank": 12 }
                            }
                        }
                    ]
                },
                {
                    "id": 86,
                    "positions": [
                        {
                            "name": "TOP",
                            "stats": {
                                "win_rate": 0.507,
                                "pick_rate": 0.066,
                                "ban_rate": 0.044,
                                "role_rate": 0.89,
                                "tier_data": { "tier": 1, "rank": 3 }
                            }
                        }
                    ]
                }
            ]
        });

        let list = parse_ranked_tier_list(&json, &test_ddragon()).unwrap();
        assert_eq!(list.len(), 4);

        // Both OP (tier 0) entries come first across lanes, sorted by win rate descending
        assert_eq!(list[0].lane, RankedLane::Mid);
        assert_eq!(list[0].name, "Ahri");
        assert_eq!(list[0].tier, 0); // raw 1 -> OP
        assert_eq!(list[0].rank, 1);
        assert_eq!(list[0].overall_rank, 1);
        assert_eq!(list[0].pick_rate, Some(11.5));
        assert_eq!(list[0].ban_rate, Some(3.6));

        assert_eq!(list[1].lane, RankedLane::Top);
        assert_eq!(list[1].name, "Garen");
        assert_eq!(list[1].tier, 0); // raw 1 -> OP
        assert_eq!(list[1].rank, 3);
        assert_eq!(list[1].overall_rank, 2);
        assert_eq!(list[1].win_rate, Some(50.7));

        // Tier 1 (raw 2 -> 1)
        assert_eq!(list[2].lane, RankedLane::Mid);
        assert_eq!(list[2].name, "Ekko");
        assert_eq!(list[2].tier, 1);
        assert_eq!(list[2].overall_rank, 3);

        // Tier 2 (raw 3 -> 2)
        assert_eq!(list[3].lane, RankedLane::Jungle);
        assert_eq!(list[3].name, "Ekko");
        assert_eq!(list[3].tier, 2);
        assert_eq!(list[3].overall_rank, 4);
    }

    #[tokio::test]
    async fn test_fetch_ranked_tier_list_applies_48h_patch_protection_and_manual_override() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/global/champions/ranked/versions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": ["16.22", "16.21"]
            })))
            .mount(&mock_server)
            .await;

        // Newest patch 16.22 has low volume (match_count: 150)
        Mock::given(method("GET"))
            .and(path("/global/champions/ranked"))
            .and(query_param("tier", "emerald_plus"))
            .and(query_param("version", "16.22"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "meta": { "version": "16.22", "match_count": 150 },
                "data": [
                    {
                        "id": 103,
                        "positions": [
                            { "name": "MID", "stats": { "win_rate": 0.49, "pick_rate": 0.05, "tier_data": { "tier": 2, "rank": 5 } } }
                        ]
                    }
                ]
            })))
            .mount(&mock_server)
            .await;

        // Trend probe indicates 16.22 was created 10 hours ago (< 48h)
        let created_at = "2026-11-14T08:00:00Z";
        let now_unix = parse_iso8601_to_unix(created_at).unwrap() + 10 * 3600;
        Mock::given(method("GET"))
            .and(path("/global/champions/ranked/103/mid"))
            .and(query_param("tier", "emerald_plus"))
            .and(query_param("version", "16.22"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "trends": {
                        "win": [{ "created_at": created_at }]
                    }
                }
            })))
            .mount(&mock_server)
            .await;

        // Mature fallback patch 16.21
        Mock::given(method("GET"))
            .and(path("/global/champions/ranked"))
            .and(query_param("tier", "emerald_plus"))
            .and(query_param("version", "16.21"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "meta": { "version": "16.21", "match_count": 500000 },
                "data": [
                    {
                        "id": 103,
                        "positions": [
                            { "name": "MID", "stats": { "win_rate": 0.52, "pick_rate": 0.11, "tier_data": { "tier": 1, "rank": 1 } } }
                        ]
                    }
                ]
            })))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let ddragon = test_ddragon();

        // Automatic mode with protection enabled -> falls back to 16.21 with 38h remaining
        let protected = fetch_ranked_tier_list_with_base(
            &client,
            &mock_server.uri(),
            Some("global"),
            Some("emerald_plus"),
            None,
            &ddragon,
            true,
            now_unix,
        )
        .await
        .unwrap();

        assert_eq!(protected.active_patch, "16.21");
        assert!(protected.patch_fallback_applied);
        assert_eq!(protected.patch_protection_remaining_hours, Some(38));
        assert_eq!(protected.champions[0].tier, 0);

        // Manual override to 16.22 -> bypasses 48h lock
        let manual = fetch_ranked_tier_list_with_base(
            &client,
            &mock_server.uri(),
            Some("global"),
            Some("emerald_plus"),
            Some("16.22"),
            &ddragon,
            true,
            now_unix,
        )
        .await
        .unwrap();

        assert_eq!(manual.active_patch, "16.22");
        assert!(!manual.patch_fallback_applied);
        assert_eq!(manual.patch_protection_remaining_hours, None);
        assert_eq!(manual.champions[0].tier, 1);
    }
}

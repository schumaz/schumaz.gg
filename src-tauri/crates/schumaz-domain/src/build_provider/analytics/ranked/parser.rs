use super::super::aram::normalize_opgg_champion_key;
use crate::build_provider::models::DataDragonCache;
use crate::models::{RankedChampionEntry, RankedLane};
use serde::Deserialize;

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
                .unwrap_or(5);
            // OP.GG ranked tier starts at 1 (1 = OP, 2 = Tier 1, ..., 5 = Tier 4).
            // Normalize to 0-based canonical tiers (0 = OP, 1 = Tier 1, ..., 4 = Tier 4).
            let normalized_tier = raw_tier.saturating_sub(1).clamp(0, 4);
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

pub(crate) fn extract_ranked_meta_info(json: &serde_json::Value) -> (Option<String>, Option<u64>) {
    let parsed: Result<RestRankedResponse, _> = serde_json::from_value(json.clone());
    match parsed {
        Ok(resp) => {
            let version = resp
                .meta
                .as_ref()
                .and_then(|m| m.version.clone())
                .filter(|v| !v.trim().is_empty());
            let match_count = resp.meta.as_ref().and_then(|m| m.match_count).or_else(|| {
                resp.data
                    .iter()
                    .filter_map(|c| c.average_stats.as_ref().and_then(|s| s.play))
                    .max()
            });
            (version, match_count)
        }
        Err(_) => (None, None),
    }
}

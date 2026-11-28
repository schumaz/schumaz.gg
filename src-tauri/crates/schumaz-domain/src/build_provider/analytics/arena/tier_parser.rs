use super::super::aram::normalize_opgg_champion_key;
use crate::build_provider::models::DataDragonCache;
use crate::models::ArenaChampionEntry;
use serde::Deserialize;

fn round_1(val: f64) -> f64 {
    (val * 10.0).round() / 10.0
}

fn round_2(val: f64) -> f64 {
    (val * 100.0).round() / 100.0
}

fn round_pct(val: f64) -> f64 {
    let pct = if val <= 1.0 { val * 100.0 } else { val };
    round_1(pct)
}

#[derive(Debug, Deserialize)]
pub(super) struct RestArenaTierListResponse {
    #[serde(default)]
    pub(super) data: Vec<RestArenaChampionItem>,
    #[serde(default)]
    pub(super) meta: Option<RestArenaMeta>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RestArenaMeta {
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    match_count: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RestArenaChampionItem {
    id: i32,
    #[serde(default)]
    is_rip: bool,
    #[serde(default)]
    average_stats: Option<RestArenaAverageStats>,
}

#[derive(Debug, Deserialize)]
struct RestArenaAverageStats {
    #[serde(default)]
    win: i64,
    #[serde(default)]
    play: i64,
    #[serde(default)]
    total_place: i64,
    #[serde(default)]
    first_place: i64,
    #[serde(default)]
    pick_rate: Option<f64>,
    #[serde(default)]
    ban_rate: Option<f64>,
    #[serde(default)]
    tier: Option<i32>,
    #[serde(default)]
    rank: Option<i32>,
    #[serde(default)]
    tier_data: Option<RestArenaTierData>,
}

#[derive(Debug, Deserialize)]
struct RestArenaTierData {
    #[serde(default)]
    tier: Option<i32>,
    #[serde(default)]
    rank: Option<i32>,
}

struct ParsedArenaEntry {
    entry: ArenaChampionEntry,
    raw_win_rate: f64,
}

pub(super) fn parse_arena_tier_list_from_resp(
    parsed: &RestArenaTierListResponse,
    ddragon: &DataDragonCache,
) -> Vec<ArenaChampionEntry> {
    let mut items: Vec<ParsedArenaEntry> = Vec::with_capacity(parsed.data.len());

    for champ in &parsed.data {
        if champ.id <= 0 || champ.is_rip {
            continue;
        }
        let Some(stats) = &champ.average_stats else {
            continue;
        };
        if stats.play <= 0 {
            continue;
        }

        let dd_meta = ddragon.champions.get(&champ.id);
        let name = dd_meta
            .map(|m| m.display_name.clone())
            .unwrap_or_else(|| format!("Champion {}", champ.id));
        let key = dd_meta
            .map(|m| normalize_opgg_champion_key(&m.key_name))
            .unwrap_or_else(|| champ.id.to_string());
        let icon_url = dd_meta.map(|m| m.icon_url.clone()).unwrap_or_default();

        let raw_tier = stats
            .tier_data
            .as_ref()
            .and_then(|td| td.tier)
            .or(stats.tier)
            .unwrap_or(5);
        // OP.GG Arena tiers start at 1 (1 = OP, 2 = Tier 1, ..., 5 = Tier 4).
        // Normalize to canonical 0-based tiers (0 = OP, 1 = Tier 1, ..., 4 = Tier 4).
        let normalized_tier = raw_tier.saturating_sub(1).clamp(0, 4);

        let rank = stats
            .tier_data
            .as_ref()
            .and_then(|td| td.rank)
            .or(stats.rank)
            .unwrap_or(999);

        let play_f = stats.play as f64;
        let raw_win_rate = (stats.win.max(0) as f64 / play_f) * 100.0;
        let raw_first_place_rate = (stats.first_place.max(0) as f64 / play_f) * 100.0;
        let avg_placement =
            (stats.total_place > 0).then(|| round_2(stats.total_place as f64 / play_f));

        items.push(ParsedArenaEntry {
            entry: ArenaChampionEntry {
                id: champ.id,
                key,
                name,
                icon_url,
                tier: normalized_tier,
                rank,
                win_rate: Some(round_1(raw_win_rate)),
                first_place_rate: Some(round_1(raw_first_place_rate)),
                avg_placement,
                pick_rate: stats.pick_rate.filter(|v| v.is_finite()).map(round_pct),
                ban_rate: stats.ban_rate.filter(|v| v.is_finite()).map(round_pct),
                play: stats.play,
            },
            raw_win_rate,
        });
    }

    items.sort_by(|a, b| {
        let a_valid_rank = if a.entry.rank > 0 { 0 } else { 1 };
        let b_valid_rank = if b.entry.rank > 0 { 0 } else { 1 };
        a.entry
            .tier
            .cmp(&b.entry.tier)
            .then_with(|| a_valid_rank.cmp(&b_valid_rank))
            .then_with(|| a.entry.rank.cmp(&b.entry.rank))
            .then_with(|| b.raw_win_rate.total_cmp(&a.raw_win_rate))
            .then_with(|| b.entry.play.cmp(&a.entry.play))
            .then_with(|| a.entry.name.cmp(&b.entry.name))
    });

    items
        .into_iter()
        .enumerate()
        .map(|(idx, mut item)| {
            if item.entry.rank <= 0 || item.entry.rank == 999 {
                item.entry.rank = (idx + 1) as i32;
            }
            item.entry
        })
        .collect()
}

/// Parses OP.GG's `GET /api/global/champions/arena` response into a sorted Arena tier list without cloning `Value`.
pub fn parse_arena_tier_list(
    json: &serde_json::Value,
    ddragon: &DataDragonCache,
) -> Result<Vec<ArenaChampionEntry>, String> {
    let parsed = RestArenaTierListResponse::deserialize(json)
        .map_err(|e| format!("Failed to parse Arena tier list response: {}", e))?;
    Ok(parse_arena_tier_list_from_resp(&parsed, ddragon))
}

/// Extracts `(meta.version, estimated_match_count)` from a typed Arena tier list response.
pub(super) fn extract_arena_meta_from_resp(
    resp: &RestArenaTierListResponse,
) -> (Option<String>, Option<u64>) {
    let version = resp
        .meta
        .as_ref()
        .and_then(|m| m.version.clone())
        .filter(|v| !v.trim().is_empty());
    let match_count = resp.meta.as_ref().and_then(|m| m.match_count).or_else(|| {
        resp.data
            .iter()
            .filter(|c| !c.is_rip)
            .filter_map(|c| c.average_stats.as_ref().map(|s| s.play.max(0) as u64))
            .max()
    });
    (version, match_count)
}

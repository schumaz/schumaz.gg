mod modes;
mod pairs;

use super::models::{SpellMetadata, SpellPairOption};
use pairs::{is_valid_pair_for_mode, unordered_spell_pair_key};
use std::collections::HashMap;

pub use modes::{
    ARAM_FALLBACK_SPELLS, CLASSIC_FALLBACK_SPELLS, JUNGLE_FALLBACK_SPELLS, SPELL_BARRIER_ID,
    SPELL_CLARITY_ID, SPELL_CLEANSE_ID, SPELL_EXHAUST_ID, SPELL_FLASH_ID, SPELL_GHOST_ID,
    SPELL_HEAL_ID, SPELL_IGNITE_ID, SPELL_MARK_ID, SPELL_SMITE_ID, SPELL_TELEPORT_ID,
    ddragon_mode_for_queue, default_spell_modes, fallback_secondary_spells_for_mode,
    is_spell_allowed_in_mode, sanitize_spell_pair_for_mode,
};
pub use pairs::{
    normalize_spell_pair_order, sanitize_spell_pair_options_for_mode,
    sanitize_top_pair_from_options,
};

fn normalize_spell_pick_rate(raw: f64) -> f64 {
    if !raw.is_finite() || raw <= 0.0 {
        return 0.0;
    }
    let pct = if raw <= 1.0 { raw * 100.0 } else { raw };
    (pct * 10.0).round() / 10.0
}

fn extract_raw_flat_spell_pair(analysis: &serde_json::Value) -> Option<(i32, i32)> {
    if let Some(arr) = analysis.get("spells").and_then(|v| v.as_array())
        && arr.len() >= 2
        && let (Some(s1), Some(s2)) = (arr[0].as_i64(), arr[1].as_i64())
        && s1 > 0
        && s2 > 0
    {
        return Some((s1 as i32, s2 as i32));
    }

    if let Some(arr) = analysis
        .pointer("/summoner_spells/ids")
        .and_then(|v| v.as_array())
        && arr.len() >= 2
        && let (Some(s1), Some(s2)) = (arr[0].as_i64(), arr[1].as_i64())
        && s1 > 0
        && s2 > 0
    {
        return Some((s1 as i32, s2 as i32));
    }

    let s1 = analysis
        .pointer("/summoner_spells/0/ids/0")
        .or_else(|| analysis.pointer("/spells/0"))
        .and_then(|v| v.as_i64())?;
    let s2 = analysis
        .pointer("/summoner_spells/0/ids/1")
        .or_else(|| analysis.pointer("/spells/1"))
        .and_then(|v| v.as_i64())?;
    if s1 > 0 && s2 > 0 {
        Some((s1 as i32, s2 as i32))
    } else {
        None
    }
}

fn parse_spell_entry(item: &serde_json::Value) -> Option<(i32, i32, i64, i64, f64)> {
    if let Some(ids) = item.get("ids").and_then(|i| i.as_array())
        && ids.len() >= 2
    {
        let s1 = ids[0].as_i64().unwrap_or(0) as i32;
        let s2 = ids[1].as_i64().unwrap_or(0) as i32;
        let play = item
            .get("play")
            .and_then(|p| p.as_i64().or_else(|| p.as_f64().map(|f| f as i64)))
            .unwrap_or(1);
        let win = item
            .get("win")
            .and_then(|w| w.as_i64().or_else(|| w.as_f64().map(|f| f as i64)))
            .unwrap_or(0)
            .max(0);
        let pick_rate = item
            .get("pick_rate")
            .and_then(|pr| pr.as_f64())
            .unwrap_or(0.0);
        return Some((s1, s2, play, win, pick_rate));
    }
    if let Some(ids) = item.as_array()
        && ids.len() >= 2
    {
        let s1 = ids[0].as_i64().unwrap_or(0) as i32;
        let s2 = ids[1].as_i64().unwrap_or(0) as i32;
        return Some((s1, s2, 1, 0, 0.0));
    }
    None
}

/// Extracts `(top_combo, pair_options)` from OP.GG analysis JSON, dynamically filtering
/// combinations and fallbacks against DataDragon's `modes` for `mode`.
pub fn extract_summoner_spell_pairs_for_mode(
    analysis: &serde_json::Value,
    is_jungle: bool,
    mode: Option<&str>,
    spells_catalog: Option<&HashMap<i32, SpellMetadata>>,
) -> ((i32, i32), Vec<SpellPairOption>) {
    let mut raw_pairs: Vec<SpellPairOption> = Vec::new();
    let mut index_by_key: HashMap<(i32, i32), usize> = HashMap::new();

    if let Some(arr) = analysis
        .get("summoner_spells")
        .or_else(|| analysis.get("spells"))
        .and_then(|v| v.as_array())
    {
        for item in arr {
            let Some((s1, s2, play, win, pick_rate)) = parse_spell_entry(item) else {
                continue;
            };
            if s1 <= 0 || s2 <= 0 || s1 == s2 {
                continue;
            }
            let key = unordered_spell_pair_key(s1, s2);
            let norm_pick = normalize_spell_pick_rate(pick_rate);
            if let Some(&idx) = index_by_key.get(&key) {
                raw_pairs[idx].play += play;
                raw_pairs[idx].win += win;
                raw_pairs[idx].pick_rate =
                    ((raw_pairs[idx].pick_rate + norm_pick).min(100.0) * 10.0).round() / 10.0;
            } else {
                let (ns1, ns2) = normalize_spell_pair_order(s1, s2, is_jungle);
                index_by_key.insert(key, raw_pairs.len());
                raw_pairs.push(SpellPairOption {
                    spell1_id: ns1,
                    spell2_id: ns2,
                    play,
                    win,
                    pick_rate: norm_pick,
                    spells: Vec::new(),
                });
            }
        }
    }

    let mut valid_pairs: Vec<SpellPairOption> = raw_pairs
        .iter()
        .filter(|p| is_valid_pair_for_mode(p.spell1_id, p.spell2_id, is_jungle, mode, spells_catalog))
        .cloned()
        .collect();

    let cmp_pairs = |a: &SpellPairOption, b: &SpellPairOption| {
        b.play
            .cmp(&a.play)
            .then_with(|| b.win.cmp(&a.win))
            .then_with(|| b.pick_rate.total_cmp(&a.pick_rate))
    };
    raw_pairs.sort_by(cmp_pairs);
    valid_pairs.sort_by(cmp_pairs);

    let raw_top = if let Some(first) = valid_pairs.first() {
        (first.spell1_id, first.spell2_id)
    } else if let Some(first) = raw_pairs.first() {
        (first.spell1_id, first.spell2_id)
    } else if let Some(pair) = extract_raw_flat_spell_pair(analysis) {
        pair
    } else if is_jungle {
        (SPELL_FLASH_ID, SPELL_SMITE_ID)
    } else if mode.is_some_and(modes::is_aram_mode) {
        (SPELL_FLASH_ID, SPELL_MARK_ID)
    } else {
        (SPELL_FLASH_ID, SPELL_IGNITE_ID)
    };

    let normalized_top = normalize_spell_pair_order(raw_top.0, raw_top.1, is_jungle);
    let preferred_ids: Vec<i32> = valid_pairs
        .iter()
        .flat_map(|p| [p.spell1_id, p.spell2_id])
        .collect();
    let (san_s1, san_s2) = sanitize_spell_pair_for_mode(
        normalized_top.0,
        normalized_top.1,
        &preferred_ids,
        is_jungle,
        mode,
        spells_catalog,
    );
    let top_combo = normalize_spell_pair_order(san_s1, san_s2, is_jungle);
    let pair_options = sanitize_spell_pair_options_for_mode(
        top_combo.0,
        top_combo.1,
        &valid_pairs,
        is_jungle,
        mode,
        spells_catalog,
    );
    (top_combo, pair_options)
}

/// Extracts `(top_combo, pair_options)` without mode filtering (mode-agnostic callers/tests).
pub fn extract_summoner_spell_pairs(
    analysis: &serde_json::Value,
    is_jungle: bool,
) -> ((i32, i32), Vec<SpellPairOption>) {
    extract_summoner_spell_pairs_for_mode(analysis, is_jungle, None, None)
}

/// Extracts the most used summoner spells from OP.GG analysis JSON.
pub fn extract_most_used_summoner_spells(analysis: &serde_json::Value) -> (i32, i32) {
    extract_summoner_spell_pairs_for_mode(analysis, false, None, None).0
}

#[cfg(test)]
#[path = "spells_tests.rs"]
mod tests;

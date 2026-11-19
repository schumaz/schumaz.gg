mod modes;

use super::models::SpellMetadata;
use std::collections::HashMap;

pub use modes::{
    ARAM_FALLBACK_SPELLS, CLASSIC_FALLBACK_SPELLS, JUNGLE_FALLBACK_SPELLS, SPELL_BARRIER_ID,
    SPELL_CLARITY_ID, SPELL_CLEANSE_ID, SPELL_EXHAUST_ID, SPELL_FLASH_ID, SPELL_GHOST_ID,
    SPELL_HEAL_ID, SPELL_IGNITE_ID, SPELL_MARK_ID, SPELL_SMITE_ID, SPELL_TELEPORT_ID,
    ddragon_mode_for_queue, default_spell_modes, fallback_secondary_spells_for_mode,
    is_spell_allowed_in_mode, sanitize_spell_choices_for_mode, sanitize_spell_pair_for_mode,
};

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

/// Extracts `(top_combo, secondary_choices)` from OP.GG analysis JSON, dynamically filtering
/// combinations and fallbacks against DataDragon's `modes` for `mode`.
pub fn extract_summoner_spell_choices_for_mode(
    analysis: &serde_json::Value,
    is_jungle: bool,
    mode: Option<&str>,
    spells_catalog: Option<&HashMap<i32, SpellMetadata>>,
) -> ((i32, i32), Vec<i32>) {
    let mut raw_combos: Vec<(i32, i32, i64)> = Vec::new();
    let mut valid_combos: Vec<(i32, i32, i64)> = Vec::new();

    if let Some(arr) = analysis
        .get("summoner_spells")
        .or_else(|| analysis.get("spells"))
        .and_then(|v| v.as_array())
    {
        for item in arr {
            let parsed = if let Some(ids) = item.get("ids").and_then(|i| i.as_array())
                && ids.len() >= 2
            {
                let s1 = ids[0].as_i64().unwrap_or(0) as i32;
                let s2 = ids[1].as_i64().unwrap_or(0) as i32;
                let play = item
                    .get("play")
                    .and_then(|p| p.as_i64().or_else(|| p.as_f64().map(|f| f as i64)))
                    .unwrap_or(1);
                Some((s1, s2, play))
            } else if let Some(ids) = item.as_array()
                && ids.len() >= 2
            {
                let s1 = ids[0].as_i64().unwrap_or(0) as i32;
                let s2 = ids[1].as_i64().unwrap_or(0) as i32;
                Some((s1, s2, 1))
            } else {
                None
            };

            if let Some((s1, s2, play)) = parsed
                && s1 > 0
                && s2 > 0
                && s1 != s2
            {
                raw_combos.push((s1, s2, play));
                if is_spell_allowed_in_mode(s1, mode, spells_catalog)
                    && is_spell_allowed_in_mode(s2, mode, spells_catalog)
                {
                    valid_combos.push((s1, s2, play));
                }
            }
        }
    }

    raw_combos.sort_by_key(|e| std::cmp::Reverse(e.2));
    valid_combos.sort_by_key(|e| std::cmp::Reverse(e.2));

    let raw_top = if let Some(&(s1, s2, _)) = valid_combos.first() {
        (s1, s2)
    } else if let Some(&(s1, s2, _)) = raw_combos.first() {
        (s1, s2)
    } else if let Some(pair) = extract_raw_flat_spell_pair(analysis) {
        pair
    } else if is_jungle {
        (SPELL_FLASH_ID, SPELL_SMITE_ID)
    } else if mode.is_some_and(modes::is_aram_mode) {
        (SPELL_FLASH_ID, SPELL_MARK_ID)
    } else {
        (SPELL_FLASH_ID, SPELL_IGNITE_ID)
    };

    // Normalize numerically-ordered OP.GG pairs (e.g. [1, 4], [3, 4], [32, 4]) so Flash (4)
    // defaults to the primary slot before FlashKeyPreference (D vs F) is applied.
    let normalized_top = if !is_jungle && raw_top.1 == SPELL_FLASH_ID && raw_top.0 != SPELL_FLASH_ID
    {
        (SPELL_FLASH_ID, raw_top.0)
    } else {
        raw_top
    };

    let top_combo = sanitize_spell_pair_for_mode(
        normalized_top.0,
        normalized_top.1,
        &[],
        is_jungle,
        mode,
        spells_catalog,
    );

    let mut candidates: Vec<i32> = Vec::new();
    if is_jungle {
        let mut companion_counts: HashMap<i32, i64> = HashMap::new();
        for &(s1, s2, play) in &valid_combos {
            let comp = if s1 == SPELL_SMITE_ID {
                Some(s2)
            } else if s2 == SPELL_SMITE_ID {
                Some(s1)
            } else {
                None
            };
            if let Some(c) = comp
                && c != SPELL_SMITE_ID
            {
                *companion_counts.entry(c).or_insert(0) += play;
            }
        }
        let mut sorted: Vec<(i32, i64)> = companion_counts.into_iter().collect();
        sorted.sort_by_key(|e| std::cmp::Reverse(e.1));
        candidates.extend(sorted.into_iter().map(|(id, _)| id));
    } else {
        let primary_spell = if top_combo.0 == SPELL_FLASH_ID || top_combo.1 == SPELL_FLASH_ID {
            SPELL_FLASH_ID
        } else {
            top_combo.0
        };
        for &(s1, s2, _) in &valid_combos {
            let alt = if s1 == primary_spell {
                s2
            } else if s2 == primary_spell {
                s1
            } else {
                continue;
            };
            if !candidates.contains(&alt) {
                candidates.push(alt);
            }
        }
    }

    let secondary_choices = sanitize_spell_choices_for_mode(
        top_combo.0,
        top_combo.1,
        &candidates,
        is_jungle,
        mode,
        spells_catalog,
    );
    (top_combo, secondary_choices)
}

/// Extracts `(top_combo, secondary_choices)` without mode filtering (mode-agnostic callers/tests).
pub fn extract_summoner_spell_choices(
    analysis: &serde_json::Value,
    is_jungle: bool,
) -> ((i32, i32), Vec<i32>) {
    extract_summoner_spell_choices_for_mode(analysis, is_jungle, None, None)
}

/// Extracts the most used summoner spells from OP.GG analysis JSON.
pub fn extract_most_used_summoner_spells(analysis: &serde_json::Value) -> (i32, i32) {
    extract_summoner_spell_choices_for_mode(analysis, false, None, None).0
}

#[cfg(test)]
#[path = "spells_tests.rs"]
mod tests;

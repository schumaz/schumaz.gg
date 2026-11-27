use super::super::models::{ItemStatEntry, SpellMetadata};
use super::mcp_dsl::{McpDocument, McpTuple, McpValue};
use super::rest_extractors::normalize_opgg_tier;
use std::collections::HashMap;

pub fn is_known_summoner_spell_id(id: i32) -> bool {
    super::super::spells::default_spell_modes(id).is_some()
}

fn normalize_pct(num: f64) -> f64 {
    if num <= 1.0 {
        (num * 1000.0).round() / 10.0
    } else {
        num
    }
}

/// Extracts `(win_rate, pick_rate)` from OP.GG MCP `AverageStats(...)` tuples in a `McpDocument`.
pub fn parse_mcp_average_stats_doc(doc: &McpDocument<'_>) -> (f64, f64) {
    let mut win_rate = 50.0;
    let mut pick_rate = 5.0;

    let Some(avg_tuple) = doc.find_tuples(&["AverageStats"]).into_iter().next() else {
        return (win_rate, pick_rate);
    };

    if let Some(named) = doc.named_args(avg_tuple) {
        for (field_name, val) in named {
            if let Some(num) = val.as_f64() {
                if field_name == "win_rate" {
                    win_rate = normalize_pct(num);
                } else if field_name == "pick_rate" {
                    pick_rate = normalize_pct(num);
                }
            }
        }
    } else {
        // Heuristic fallback: play count is an integer > 100, win_rate and pick_rate are <= 100
        let float_candidates: Vec<f64> = avg_tuple
            .args
            .iter()
            .filter_map(McpValue::as_f64)
            .filter(|&v| v <= 100.0)
            .collect();
        if float_candidates.len() >= 2 {
            win_rate = normalize_pct(float_candidates[0]);
            pick_rate = normalize_pct(float_candidates[1]);
        } else if let Some(&w) = float_candidates.first() {
            win_rate = normalize_pct(w);
        }
    }

    (win_rate, pick_rate)
}

/// Extracts `(win_rate, pick_rate)` from OP.GG MCP `AverageStats(...)` headers and tuples.
pub fn parse_mcp_average_stats(text: &str) -> (f64, f64) {
    let doc = McpDocument::parse(text);
    parse_mcp_average_stats_doc(&doc)
}

/// Extracts and normalizes the canonical champion tier (`0 = OP`, `1..=4 = Tier 1..4`) from a `McpDocument`.
pub fn parse_mcp_tier_doc(doc: &McpDocument<'_>) -> Option<i32> {
    for ctor in ["AverageStats", "TierData"] {
        if let Some(tuple) = doc.find_tuples(&[ctor]).into_iter().next()
            && let Some(raw) = doc
                .field(tuple, "tier")
                .and_then(McpValue::as_i32)
        {
            return normalize_opgg_tier(raw);
        }
    }
    None
}

/// Extracts and normalizes the canonical champion tier (`0 = OP`, `1..=4 = Tier 1..4`) from
/// OP.GG MCP `AverageStats(...)` or `TierData(...)` tuples when explicitly declared.
pub fn parse_mcp_tier(text: &str) -> Option<i32> {
    let doc = McpDocument::parse(text);
    parse_mcp_tier_doc(&doc)
}

fn parse_mcp_spell_tuple(doc: &McpDocument<'_>, tuple: &McpTuple<'_>) -> Option<(Vec<i32>, i64, i64, f64)> {
    let mut play = 1i64;
    let mut win = 0i64;
    let mut pick_rate = 0.0f64;

    if let Some(named) = doc.named_args(tuple) {
        let mut ids: Option<Vec<i32>> = None;
        for (field, val) in named {
            match field {
                "ids" => ids = val.as_i32_list(),
                "play" => {
                    if let Some(p) = val.as_i64() {
                        play = p;
                    }
                }
                "win" => {
                    if let Some(w) = val.as_i64() {
                        win = w.max(0);
                    }
                }
                "pick_rate" => {
                    if let Some(pr) = val.as_f64() {
                        pick_rate = pr;
                    }
                }
                _ => {}
            }
        }
        return ids.filter(|v| v.len() >= 2).map(|v| (v, play, win, pick_rate));
    }

    let ids_idx = tuple.args.iter().position(|a| a.as_i32_list().is_some())?;
    let ids = tuple.args[ids_idx].as_i32_list().filter(|v| v.len() >= 2)?;
    let numeric_args: Vec<&McpValue<'_>> = tuple.args[ids_idx + 1..]
        .iter()
        .filter(|a| matches!(a, McpValue::Int(_) | McpValue::Float(_)))
        .collect();

    let non_id_fields: Vec<&str> = doc
        .fields_for(&[tuple.name])
        .map(|fields| {
            fields
                .iter()
                .copied()
                .filter(|&f| f != "ids" && f != "ids_names")
                .collect()
        })
        .unwrap_or_default();

    if !non_id_fields.is_empty() && non_id_fields.len() == numeric_args.len() {
        for (&field, val) in non_id_fields.iter().zip(numeric_args.iter()) {
            match field {
                "play" => {
                    if let Some(p) = val.as_i64() {
                        play = p;
                    }
                }
                "win" => {
                    if let Some(w) = val.as_i64() {
                        win = w.max(0);
                    }
                }
                "pick_rate" => {
                    if let Some(pr) = val.as_f64() {
                        pick_rate = pr;
                    }
                }
                _ => {}
            }
        }
    } else {
        let mut ints = Vec::new();
        for val in numeric_args {
            match *val {
                McpValue::Int(n) => ints.push(n),
                McpValue::Float(f) if f.is_finite() => pick_rate = f,
                _ => {}
            }
        }
        if ints.len() >= 2 {
            // Without explicit header, OP.GG MCP typically emits (win, play) where play >= win
            play = ints[0].max(ints[1]);
            win = ints[0].min(ints[1]).max(0);
        } else if let Some(&p) = ints.first() {
            play = p;
        }
    }

    Some((ids, play, win, pick_rate))
}

fn is_spell_id_known(id: i32, spells_catalog: Option<&HashMap<i32, SpellMetadata>>) -> bool {
    spells_catalog.is_some_and(|c| c.contains_key(&id)) || is_known_summoner_spell_id(id)
}

/// Extracts all `SummonerSpells(...)` entries from a `McpDocument` into a JSON array
/// compatible with `extract_summoner_spell_pairs_for_mode`.
pub fn extract_mcp_summoner_spell_entries_doc(
    doc: &McpDocument<'_>,
    fallback_entries: &[ItemStatEntry],
    spells_catalog: Option<&HashMap<i32, SpellMetadata>>,
) -> serde_json::Value {
    let mut spell_combos: Vec<serde_json::Value> = Vec::new();
    for tuple in doc.find_tuples(&["SummonerSpells"]) {
        if let Some((ids, play, win, pick_rate)) = parse_mcp_spell_tuple(doc, tuple) {
            spell_combos.push(serde_json::json!({
                "ids": [ids[0], ids[1]],
                "play": play,
                "win": win,
                "pick_rate": pick_rate
            }));
        }
    }

    if spell_combos.is_empty() {
        for entry in fallback_entries {
            if entry.ids.len() == 2
                && is_spell_id_known(entry.ids[0], spells_catalog)
                && is_spell_id_known(entry.ids[1], spells_catalog)
            {
                spell_combos.push(serde_json::json!({
                    "ids": [entry.ids[0], entry.ids[1]],
                    "play": entry.play,
                    "win": entry.win,
                    "pick_rate": entry.pick_rate
                }));
            }
        }
    }

    serde_json::json!({ "summoner_spells": spell_combos })
}

/// Extracts all `SummonerSpells(...)` entries from OP.GG MCP text into a JSON array
/// compatible with `extract_summoner_spell_pairs_for_mode`.
pub fn extract_mcp_summoner_spell_entries(
    text: &str,
    all_entries: &[ItemStatEntry],
) -> serde_json::Value {
    let doc = McpDocument::parse(text);
    extract_mcp_summoner_spell_entries_doc(&doc, all_entries, None)
}

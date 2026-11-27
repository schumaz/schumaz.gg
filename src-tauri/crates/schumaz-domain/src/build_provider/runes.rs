pub mod mcp;
pub use mcp::*;

use super::models::RunePageOption;

pub fn parse_single_rune_entry(entry: &serde_json::Value) -> Option<RunePageOption> {
    let primary_style_id = entry
        .get("primary_style_id")
        .or_else(|| entry.get("primary_page_id"))
        .or_else(|| entry.pointer("/primary_style_id"))
        .or_else(|| entry.pointer("/primary_page_id"))
        .and_then(|v| v.as_i64())? as i32;

    let sub_style_id = entry
        .get("sub_style_id")
        .or_else(|| entry.get("secondary_page_id"))
        .or_else(|| entry.get("sub_page_id"))
        .or_else(|| entry.pointer("/sub_style_id"))
        .or_else(|| entry.pointer("/secondary_page_id"))
        .and_then(|v| v.as_i64())? as i32;

    if primary_style_id <= 0 || sub_style_id <= 0 || primary_style_id == sub_style_id {
        return None;
    }

    let mut selected_perk_ids = Vec::with_capacity(9);

    if let Some(primary_perks) = entry
        .get("primary_perk_ids")
        .or_else(|| entry.get("primary_rune_ids"))
        .or_else(|| entry.pointer("/primary_perk_ids"))
        .or_else(|| entry.pointer("/primary_rune_ids"))
        .and_then(|v| v.as_array())
    {
        for p in primary_perks {
            if let Some(id) = p.as_i64() {
                selected_perk_ids.push(id as i32);
            }
        }
    }

    if let Some(sub_perks) = entry
        .get("sub_perk_ids")
        .or_else(|| entry.get("secondary_rune_ids"))
        .or_else(|| entry.pointer("/sub_perk_ids"))
        .or_else(|| entry.pointer("/secondary_rune_ids"))
        .and_then(|v| v.as_array())
    {
        for p in sub_perks {
            if let Some(id) = p.as_i64() {
                selected_perk_ids.push(id as i32);
            }
        }
    }

    if let Some(stat_mods) = entry
        .get("stat_shards")
        .or_else(|| entry.get("stat_mod_ids"))
        .or_else(|| entry.pointer("/stat_shards"))
        .or_else(|| entry.pointer("/stat_mod_ids"))
        .and_then(|v| v.as_array())
    {
        for s in stat_mods {
            if let Some(id) = s.as_i64() {
                selected_perk_ids.push(id as i32);
            }
        }
    }

    if selected_perk_ids.len() != 9 {
        return None;
    }

    let play = entry.get("play").and_then(|v| v.as_i64()).unwrap_or(0);
    let win = entry.get("win").and_then(|v| v.as_i64()).unwrap_or(0);
    let pick_rate = entry
        .get("pick_rate")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    Some(RunePageOption {
        primary_style_id,
        sub_style_id,
        selected_perk_ids,
        play,
        win,
        pick_rate,
        ..Default::default()
    })
}

/// Extracts up to 3 valid rune pages from OP.GG analysis JSON, sorted by most played (`play` descending).
pub fn extract_top_rune_pages(analysis: &serde_json::Value) -> Vec<RunePageOption> {
    let mut pages: Vec<RunePageOption> = Vec::new();

    let runes_val = analysis
        .get("runes")
        .or_else(|| analysis.get("rune_pages"))
        .or_else(|| analysis.pointer("/data/runes"))
        .or_else(|| analysis.pointer("/data/rune_pages"));

    if let Some(val) = runes_val {
        if let Some(arr) = val.as_array() {
            for entry in arr {
                if let Some(page) = parse_single_rune_entry(entry) {
                    pages.push(page);
                }
            }
        } else if let Some(obj) = val.as_object() {
            let mut has_numeric_keys = false;
            for (k, v) in obj {
                if k.parse::<usize>().is_ok() {
                    has_numeric_keys = true;
                    if let Some(page) = parse_single_rune_entry(v) {
                        pages.push(page);
                    }
                }
            }
            if !has_numeric_keys && let Some(page) = parse_single_rune_entry(val) {
                pages.push(page);
            }
        }
    }

    // Deduplicate identical setups
    let mut unique_pages: Vec<RunePageOption> = Vec::new();
    for p in pages {
        if !unique_pages.iter().any(|existing| {
            existing.primary_style_id == p.primary_style_id
                && existing.sub_style_id == p.sub_style_id
                && existing.selected_perk_ids == p.selected_perk_ids
        }) {
            unique_pages.push(p);
        }
    }

    // Sort by play count descending
    unique_pages.sort_by_key(|p| std::cmp::Reverse(p.play));
    unique_pages.truncate(3);

    unique_pages
}

#[cfg(test)]
#[path = "runes_tests.rs"]
mod tests;

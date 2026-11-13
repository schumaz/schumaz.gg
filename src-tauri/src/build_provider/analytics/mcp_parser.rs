use super::super::items::*;
use super::super::models::{ChampionBuild, ItemStatEntry, RunePageOption, SpellMetadata};
use super::super::runes::extract_all_mcp_runes;
use super::mcp_extractors::{extract_all_core_items_from_text, extract_mcp_grouped_item_lists};

/// Parses OP.GG MCP text output into a ChampionBuild.
pub fn parse_opgg_mcp_text(
    text: &str,
    champion_id: i32,
    champion_name: &str,
) -> Result<ChampionBuild, String> {
    // 1. Extract win_rate and pick_rate: AverageStats(...)
    let mut win_rate = 50.0;
    let mut pick_rate = 5.0;

    let avg_fields: Vec<&str> = if let Some(def_idx) = text.find("class AverageStats:") {
        let line_rest = &text[def_idx + 19..];
        let end_line = line_rest.find('\n').unwrap_or(line_rest.len());
        line_rest[..end_line]
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    };

    if let Some(avg_idx) = text.find("AverageStats(") {
        let after_avg = &text[avg_idx + 13..];
        if let Some(close_idx) = after_avg.find(')') {
            let parts: Vec<&str> = after_avg[..close_idx].split(',').collect();
            if !avg_fields.is_empty() && avg_fields.len() == parts.len() {
                for (field_name, val_str) in avg_fields.iter().zip(parts.iter()) {
                    if let Ok(num) = val_str.trim().parse::<f64>() {
                        if *field_name == "win_rate" {
                            win_rate = if num <= 1.0 {
                                (num * 1000.0).round() / 10.0
                            } else {
                                num
                            };
                        } else if *field_name == "pick_rate" {
                            pick_rate = if num <= 1.0 {
                                (num * 1000.0).round() / 10.0
                            } else {
                                num
                            };
                        }
                    }
                }
            } else {
                // Heuristic fallback: play count is an integer > 100, win_rate and pick_rate are <= 100
                let float_candidates: Vec<f64> = parts
                    .iter()
                    .filter_map(|s| s.trim().parse::<f64>().ok())
                    .filter(|&v| v <= 100.0)
                    .collect();
                if float_candidates.len() >= 2 {
                    let w = float_candidates[0];
                    let p = float_candidates[1];
                    win_rate = if w <= 1.0 {
                        (w * 1000.0).round() / 10.0
                    } else {
                        w
                    };
                    pick_rate = if p <= 1.0 {
                        (p * 1000.0).round() / 10.0
                    } else {
                        p
                    };
                } else if float_candidates.len() == 1 {
                    let w = float_candidates[0];
                    win_rate = if w <= 1.0 {
                        (w * 1000.0).round() / 10.0
                    } else {
                        w
                    };
                }
            }
        }
    }

    // 2. Extract Runes: Runes(8100,[8112,8139,8140,8106],8200,[8210,8226],[5005,5008,5001])
    let runes_idx = text
        .find("Runes(")
        .ok_or_else(|| "Missing Runes block in OP.GG text".to_string())?;
    let after_runes = &text[runes_idx + 6..];

    // Find primary style id
    let comma1 = after_runes
        .find(',')
        .ok_or_else(|| "Invalid Runes format".to_string())?;
    let mut primary_style_id: i32 = after_runes[..comma1]
        .trim()
        .parse()
        .map_err(|e| format!("Invalid primary style id: {}", e))?;

    // Primary perks array: [ ... ]
    let p_open = after_runes
        .find('[')
        .ok_or_else(|| "Missing primary perks [".to_string())?;
    let p_close = after_runes
        .find(']')
        .ok_or_else(|| "Missing primary perks ]".to_string())?;
    let primary_perks: Vec<i32> = after_runes[p_open + 1..p_close]
        .split(',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .collect();

    let after_primary = &after_runes[p_close + 1..];
    let after_primary_trim =
        after_primary.trim_start_matches(|c: char| c == ',' || c.is_whitespace());

    // Sub style id
    let comma2 = after_primary_trim
        .find(',')
        .ok_or_else(|| "Invalid sub style format".to_string())?;
    let mut sub_style_id: i32 = after_primary_trim[..comma2]
        .trim()
        .parse()
        .map_err(|e| format!("Invalid sub style id: {}", e))?;

    // Secondary perks array: [ ... ]
    let s_open = after_primary_trim
        .find('[')
        .ok_or_else(|| "Missing secondary perks [".to_string())?;
    let s_close = after_primary_trim
        .find(']')
        .ok_or_else(|| "Missing secondary perks ]".to_string())?;
    let secondary_perks: Vec<i32> = after_primary_trim[s_open + 1..s_close]
        .split(',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .collect();

    // Stat mods array: [ ... ]
    let after_sec = &after_primary_trim[s_close + 1..];
    let m_open = after_sec
        .find('[')
        .ok_or_else(|| "Missing stat mods [".to_string())?;
    let m_close = after_sec
        .find(']')
        .ok_or_else(|| "Missing stat mods ]".to_string())?;
    let stat_mods: Vec<i32> = after_sec[m_open + 1..m_close]
        .split(',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .collect();

    let mut selected_perk_ids = Vec::with_capacity(9);
    selected_perk_ids.extend(primary_perks);
    selected_perk_ids.extend(secondary_perks);
    selected_perk_ids.extend(stat_mods);

    if selected_perk_ids.len() != 9 {
        return Err(format!(
            "Expected 9 perks, extracted {}",
            selected_perk_ids.len()
        ));
    }

    // 3. Extract all CoreItems entries with stats
    let all_entries = extract_all_core_items_from_text(text);

    // 4. Extract summoner spells
    let mut spell1_id = 4;
    let mut spell2_id = 14;
    if let Some(ss_idx) = text.find("SummonerSpells(") {
        let after_ss = &text[ss_idx + 15..];
        let maybe_brackets = after_ss
            .find('[')
            .zip(after_ss.find(']'))
            .filter(|(open, close)| open < close);
        if let Some((o, c)) = maybe_brackets {
            let ids: Vec<i32> = after_ss[o + 1..c]
                .split(',')
                .filter_map(|s| s.trim().parse::<i32>().ok())
                .collect();
            if ids.len() >= 2 {
                spell1_id = ids[0];
                spell2_id = ids[1];
            }
        }
    } else {
        for entry in &all_entries {
            if entry.ids.len() == 2
                && matches!(entry.ids[0], 4 | 14 | 12 | 7 | 21 | 11 | 6 | 3 | 1 | 32)
            {
                spell1_id = entry.ids[0];
                spell2_id = entry.ids[1];
                break;
            }
        }
    }

    // Filter out spell arrays from items entries
    let item_candidate_entries: Vec<ItemStatEntry> = all_entries
        .into_iter()
        .filter(|entry| {
            !(entry.ids.len() == 2
                && matches!(entry.ids[0], 4 | 14 | 12 | 7 | 21 | 11 | 6 | 3 | 1 | 32))
        })
        .collect();

    let is_ap = text.contains("\"AP\"")
        || (!text.contains("\"AD\"")
            && (champion_name.to_lowercase().contains("ahri")
                || champion_name.to_lowercase().contains("lux")
                || champion_name.to_lowercase().contains("syndra")
                || champion_name.to_lowercase().contains("veigar")));
    let damage_type = if is_ap { "AP" } else { "AD" };

    let starter_entries = if let Some(e) = item_candidate_entries.get(2) {
        vec![e.clone()]
    } else {
        Vec::new()
    };

    let grouped_lists = extract_mcp_grouped_item_lists(text);
    let mut fourth_entries = Vec::new();
    let mut fifth_entries = Vec::new();
    let mut sixth_entries = Vec::new();
    let mut last_entries = Vec::new();
    let mut boots_from_mcp = Vec::new();
    for (name, items) in grouped_lists {
        match name.as_str() {
            "boots" => boots_from_mcp = items,
            "fourth_items" => fourth_entries = items,
            "fifth_items" => fifth_entries = items,
            "sixth_items" => sixth_entries = items,
            "last_items" => last_entries = items,
            _ => last_entries.extend(items),
        }
    }

    let boot_entries = if !boots_from_mcp.is_empty() {
        boots_from_mcp
    } else {
        item_candidate_entries.get(1).cloned().into_iter().collect()
    };

    let mut candidate_entries = Vec::new();
    let core_ids = if let Some(first) = item_candidate_entries.first() {
        let mut core_entry = first.clone();
        if core_entry.play == 1 {
            core_entry.play = 10000;
        }
        let ids = core_entry.ids.clone();
        candidate_entries.push(core_entry);
        ids
    } else {
        Vec::new()
    };

    if !last_entries.is_empty() {
        for entry in &last_entries {
            let filtered_ids: Vec<i32> = entry
                .ids
                .iter()
                .filter(|id| !core_ids.contains(id))
                .copied()
                .collect();
            if !filtered_ids.is_empty() {
                candidate_entries.push(ItemStatEntry {
                    ids: filtered_ids,
                    play: entry.play,
                    win: entry.win,
                    pick_rate: entry.pick_rate,
                });
            }
        }
    } else if item_candidate_entries.len() > 3 {
        for entry in &item_candidate_entries[3..] {
            let filtered_ids: Vec<i32> = entry
                .ids
                .iter()
                .filter(|id| !core_ids.contains(id))
                .copied()
                .collect();
            if !filtered_ids.is_empty() {
                candidate_entries.push(ItemStatEntry {
                    ids: filtered_ids,
                    play: entry.play,
                    win: entry.win,
                    pick_rate: entry.pick_rate,
                });
            }
        }
    }

    let position = if text.contains("\"position\": \"jungle\"") || text.contains("\"jungle\"") {
        "jungle"
    } else if text.contains("\"position\": \"support\"") || text.contains("\"support\"") {
        "support"
    } else {
        ""
    };

    let all_for_tear: Vec<ItemStatEntry> = candidate_entries
        .iter()
        .chain(fourth_entries.iter())
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
        .cloned()
        .collect();
    let tear_upgrade = resolve_tear_to_completed_item(&all_for_tear, position, damage_type);

    let normalize_entries = |entries: &mut [ItemStatEntry]| {
        for entry in entries.iter_mut() {
            entry.ids = resolve_tear_item_ids(&entry.ids, tear_upgrade);
        }
    };
    normalize_entries(&mut candidate_entries);
    normalize_entries(&mut fourth_entries);
    normalize_entries(&mut fifth_entries);
    normalize_entries(&mut sixth_entries);

    let cand_ids: Vec<i32> = candidate_entries
        .iter()
        .flat_map(|e| e.ids.clone())
        .collect();
    let core_build_options =
        extract_top_core_build_options(&candidate_entries, &cand_ids, damage_type);
    let primary_core = if let Some(first_core) = core_build_options.first() {
        first_core.item_ids.clone()
    } else {
        vec![]
    };
    let item_blocks = build_5_item_blocks_with_depth_lists(
        starter_entries,
        boot_entries,
        candidate_entries.clone(),
        fourth_entries.clone(),
        fifth_entries.clone(),
        sixth_entries.clone(),
        damage_type,
        position,
    );
    let general_situational = item_blocks.get(4).map(|b| b.item_ids.clone()).unwrap_or_default();

    let build_options = build_unified_build_options(
        &candidate_entries,
        &fourth_entries,
        &fifth_entries,
        &sixth_entries,
        &candidate_entries,
        &last_entries,
        &general_situational,
        damage_type,
        position,
    );
    let final_build_options = build_3_final_build_options(
        &primary_core,
        &fourth_entries,
        &fifth_entries,
        &sixth_entries,
        &cand_ids,
        damage_type,
    );

    let spell_choices = vec![SpellMetadata {
        id: spell2_id,
        name: String::new(),
        icon_url: String::new(),
    }];

    let mut top_rune_pages = extract_all_mcp_runes(text);
    if top_rune_pages.is_empty() {
        top_rune_pages.push(RunePageOption {
            primary_style_id,
            sub_style_id,
            selected_perk_ids: selected_perk_ids.clone(),
            play: 1,
            win: 1,
            pick_rate: 1.0,
            ..Default::default()
        });
    } else {
        let first = &top_rune_pages[0];
        primary_style_id = first.primary_style_id;
        sub_style_id = first.sub_style_id;
        selected_perk_ids = first.selected_perk_ids.clone();
    }

    Ok(ChampionBuild {
        champion_id,
        champion_name: champion_name.to_string(),
        win_rate,
        pick_rate,
        primary_style_id,
        sub_style_id,
        selected_perk_ids,
        spell1_id,
        spell2_id,
        item_blocks,
        spell_choices,
        rune_pages: top_rune_pages,
        build_options,
        core_build_options,
        final_build_options,
        ..Default::default()
    })
}

/// Extracts the top summoner spells and the 2 most popular choices for the secondary spell slot
#[cfg(test)]
#[path = "mcp_parser_tests.rs"]
mod tests;

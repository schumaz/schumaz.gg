use super::super::items::*;
use super::super::models::{ChampionBuild, ItemStatEntry, SpellMetadata};
use super::super::runes::extract_all_mcp_runes;
use super::super::spells::extract_summoner_spell_choices;
use super::mcp_extractors::{extract_all_core_items_from_text, extract_mcp_grouped_item_lists};

/// Extracts all `SummonerSpells(...)` entries from OP.GG MCP text into a JSON array
/// compatible with `extract_summoner_spell_choices`.
fn extract_mcp_summoner_spell_entries(
    text: &str,
    all_entries: &[ItemStatEntry],
) -> serde_json::Value {
    let spell_fields: Vec<&str> = if let Some(def_idx) = text.find("class SummonerSpells:") {
        let line_rest = &text[def_idx + 21..];
        let end_line = line_rest.find('\n').unwrap_or(line_rest.len());
        line_rest[..end_line]
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    };

    let play_after_ids_idx = spell_fields
        .iter()
        .filter(|&&f| f != "ids" && f != "ids_names")
        .position(|&f| f == "play");

    let mut spell_combos: Vec<serde_json::Value> = Vec::new();
    let mut search_slice = text;
    while let Some(ss_idx) = search_slice.find("SummonerSpells(") {
        let after_ss = &search_slice[ss_idx + 15..];
        let Some((o, c)) = after_ss
            .find('[')
            .zip(after_ss.find(']'))
            .filter(|(open, close)| open < close)
        else {
            break;
        };

        let ids: Vec<i32> = after_ss[o + 1..c]
            .split(',')
            .filter_map(|s| s.trim().parse::<i32>().ok())
            .collect();

        let rest_after_bracket = &after_ss[c + 1..];
        let mut play = 1i64;
        if let Some(close_paren) = rest_after_bracket.find(')') {
            let stats_nums: Vec<i64> = rest_after_bracket[..close_paren]
                .trim_start_matches(',')
                .split(',')
                .filter_map(|s| s.trim().parse::<i64>().ok())
                .collect();
            if let Some(idx) = play_after_ids_idx
                && let Some(&p) = stats_nums.get(idx)
            {
                play = p;
            } else if stats_nums.len() >= 2 {
                // Without explicit header, OP.GG MCP typically emits (win, play) where play >= win
                play = stats_nums[0].max(stats_nums[1]);
            } else if let Some(&p) = stats_nums.first() {
                play = p;
            }
            search_slice = &rest_after_bracket[close_paren + 1..];
        } else {
            search_slice = rest_after_bracket;
        }

        if ids.len() >= 2 {
            spell_combos.push(serde_json::json!({
                "ids": [ids[0], ids[1]],
                "play": play
            }));
        }
    }

    if spell_combos.is_empty() {
        for entry in all_entries {
            if entry.ids.len() == 2
                && is_known_summoner_spell_id(entry.ids[0])
                && is_known_summoner_spell_id(entry.ids[1])
            {
                spell_combos.push(serde_json::json!({
                    "ids": [entry.ids[0], entry.ids[1]],
                    "play": entry.play
                }));
            }
        }
    }

    serde_json::json!({ "summoner_spells": spell_combos })
}

fn is_known_summoner_spell_id(id: i32) -> bool {
    matches!(id, 1 | 3 | 4 | 6 | 7 | 11 | 12 | 13 | 14 | 21 | 32)
}

/// Parses OP.GG MCP text output into a ChampionBuild (requiring valid runes).
pub fn parse_opgg_mcp_text(
    text: &str,
    champion_id: i32,
    champion_name: &str,
) -> Result<ChampionBuild, String> {
    parse_opgg_mcp_text_with_options(text, champion_id, champion_name, true)
}

/// Parses OP.GG MCP text output into a ChampionBuild with configurable rune requirement.
pub fn parse_opgg_mcp_text_with_options(
    text: &str,
    champion_id: i32,
    champion_name: &str,
    supports_runes: bool,
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

    // 2. Extract Runes (supports both legacy `Runes(8100,...)` and OP.GG MCP `Runes(id,8100,"Domination",...)`)
    let top_rune_pages = extract_all_mcp_runes(text);
    let (primary_style_id, sub_style_id, selected_perk_ids) =
        if let Some(first) = top_rune_pages.first() {
            (
                first.primary_style_id,
                first.sub_style_id,
                first.selected_perk_ids.clone(),
            )
        } else if supports_runes {
            if !text.contains("Runes(") {
                return Err("Missing Runes block in OP.GG text".to_string());
            }
            return Err("Invalid Runes format in OP.GG text".to_string());
        } else {
            (0, 0, Vec::new())
        };

    // 3. Extract all CoreItems entries with stats
    let all_entries = extract_all_core_items_from_text(text);

    let position = if text.contains("\"position\": \"jungle\"") || text.contains("\"jungle\"") {
        "jungle"
    } else if text.contains("\"position\": \"support\"") || text.contains("\"support\"") {
        "support"
    } else {
        ""
    };
    let is_jungle = position.eq_ignore_ascii_case("jungle");

    // 4. Extract summoner spells via shared extract_summoner_spell_choices
    let spells_json = extract_mcp_summoner_spell_entries(text, &all_entries);
    let ((spell1_id, spell2_id), spell_choice_ids) =
        extract_summoner_spell_choices(&spells_json, is_jungle);

    // Filter out spell arrays from items entries
    let item_candidate_entries: Vec<ItemStatEntry> = all_entries
        .into_iter()
        .filter(|entry| {
            !(entry.ids.len() == 2
                && is_known_summoner_spell_id(entry.ids[0])
                && is_known_summoner_spell_id(entry.ids[1]))
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
    normalize_entries(&mut last_entries);

    let cand_ids: Vec<i32> = candidate_entries
        .iter()
        .chain(fourth_entries.iter())
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
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
    let general_situational = item_blocks
        .get(4)
        .map(|b| b.item_ids.clone())
        .unwrap_or_default();

    let build_options = build_unified_build_options(
        &candidate_entries,
        &fourth_entries,
        &fifth_entries,
        &sixth_entries,
        &candidate_entries,
        &[],
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

    let spell_choices = spell_choice_ids
        .into_iter()
        .map(|id| SpellMetadata {
            id,
            name: String::new(),
            icon_url: String::new(),
        })
        .collect();

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
        supports_runes,
        ..Default::default()
    })
}

#[cfg(test)]
#[path = "mcp_parser_tests.rs"]
mod tests;

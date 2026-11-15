use super::super::models::ItemStatEntry;

pub fn is_known_summoner_spell_id(id: i32) -> bool {
    matches!(id, 1 | 3 | 4 | 6 | 7 | 11 | 12 | 13 | 14 | 21 | 32)
}

/// Extracts `(win_rate, pick_rate)` from OP.GG MCP `AverageStats(...)` headers and tuples.
pub fn parse_mcp_average_stats(text: &str) -> (f64, f64) {
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

    (win_rate, pick_rate)
}

/// Extracts all `SummonerSpells(...)` entries from OP.GG MCP text into a JSON array
/// compatible with `extract_summoner_spell_choices`.
pub fn extract_mcp_summoner_spell_entries(
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

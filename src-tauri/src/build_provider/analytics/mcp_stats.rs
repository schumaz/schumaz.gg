use super::super::models::ItemStatEntry;
use super::rest_extractors::normalize_opgg_tier;

pub fn is_known_summoner_spell_id(id: i32) -> bool {
    matches!(id, 1 | 3 | 4 | 6 | 7 | 11 | 12 | 13 | 14 | 21 | 32)
}

fn split_top_level_args(input: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (idx, ch) in input.char_indices() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                args.push(input[start..idx].trim());
                start = idx + 1;
            }
            _ => {}
        }
    }
    if start <= input.len() {
        let tail = input[start..].trim();
        if !tail.is_empty() {
            args.push(tail);
        }
    }
    args
}

fn extract_balanced_paren_body<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    let idx = text.find(marker)?;
    let rest = &text[idx + marker.len()..];
    let mut depth = 1usize;
    for (offset, ch) in rest.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&rest[..offset]);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_class_fields<'a>(text: &'a str, header: &str) -> Vec<&'a str> {
    if let Some(def_idx) = text.find(header) {
        let line_rest = &text[def_idx + header.len()..];
        let end_line = line_rest.find('\n').unwrap_or(line_rest.len());
        line_rest[..end_line]
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    }
}

/// Extracts `(win_rate, pick_rate)` from OP.GG MCP `AverageStats(...)` headers and tuples.
pub fn parse_mcp_average_stats(text: &str) -> (f64, f64) {
    let mut win_rate = 50.0;
    let mut pick_rate = 5.0;

    let avg_fields = parse_class_fields(text, "class AverageStats:");

    if let Some(body) = extract_balanced_paren_body(text, "AverageStats(") {
        let parts = split_top_level_args(body);
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

    (win_rate, pick_rate)
}

/// Extracts and normalizes the canonical champion tier (`0 = OP`, `1..=4 = Tier 1..4`) from
/// OP.GG MCP `AverageStats(...)` or `TierData(...)` tuples when explicitly declared.
pub fn parse_mcp_tier(text: &str) -> Option<i32> {
    let avg_fields = parse_class_fields(text, "class AverageStats:");
    if let Some(body) = extract_balanced_paren_body(text, "AverageStats(") {
        let parts = split_top_level_args(body);
        if !avg_fields.is_empty() && avg_fields.len() == parts.len() {
            for (field_name, val_str) in avg_fields.iter().zip(parts.iter()) {
                if *field_name == "tier"
                    && let Ok(raw) = val_str.trim().parse::<i32>()
                {
                    return normalize_opgg_tier(raw);
                }
            }
        }
    }

    let tier_data_fields = parse_class_fields(text, "class TierData:");
    if let Some(td_body) = extract_balanced_paren_body(text, "TierData(") {
        let parts = split_top_level_args(td_body);
        if !tier_data_fields.is_empty() && tier_data_fields.len() == parts.len() {
            for (field_name, val_str) in tier_data_fields.iter().zip(parts.iter()) {
                if *field_name == "tier"
                    && let Ok(raw) = val_str.trim().parse::<i32>()
                {
                    return normalize_opgg_tier(raw);
                }
            }
        }
    }

    None
}

fn parse_mcp_spell_stats(
    after_bracket: &str,
    non_id_fields: &[&str],
) -> (i64, i64, f64, usize) {
    let Some(close_paren) = after_bracket.find(')') else {
        return (1, 0, 0.0, 0);
    };
    let stats_str = after_bracket[..close_paren].trim_start_matches(',');
    let args = split_top_level_args(stats_str);
    let mut play = 1i64;
    let mut win = 0i64;
    let mut pick_rate = 0.0f64;

    if !non_id_fields.is_empty() && non_id_fields.len() == args.len() {
        for (&field, &val_str) in non_id_fields.iter().zip(args.iter()) {
            match field {
                "play" => {
                    if let Ok(p) = val_str.trim().parse::<f64>() {
                        play = p.round() as i64;
                    }
                }
                "win" => {
                    if let Ok(w) = val_str.trim().parse::<f64>() {
                        win = (w.round() as i64).max(0);
                    }
                }
                "pick_rate" => {
                    if let Ok(pr) = val_str.trim().parse::<f64>() {
                        pick_rate = pr;
                    }
                }
                _ => {}
            }
        }
    } else {
        let numeric_args: Vec<&str> = args
            .iter()
            .copied()
            .filter(|s| !s.starts_with('[') && !s.starts_with('"'))
            .collect();
        if !non_id_fields.is_empty() && non_id_fields.len() == numeric_args.len() {
            for (&field, &val_str) in non_id_fields.iter().zip(numeric_args.iter()) {
                match field {
                    "play" => {
                        if let Ok(p) = val_str.parse::<f64>() {
                            play = p.round() as i64;
                        }
                    }
                    "win" => {
                        if let Ok(w) = val_str.parse::<f64>() {
                            win = (w.round() as i64).max(0);
                        }
                    }
                    "pick_rate" => {
                        if let Ok(pr) = val_str.parse::<f64>() {
                            pick_rate = pr;
                        }
                    }
                    _ => {}
                }
            }
        } else {
            let mut ints: Vec<i64> = Vec::new();
            for tok in numeric_args {
                if !tok.contains('.')
                    && let Ok(n) = tok.parse::<i64>()
                {
                    ints.push(n);
                } else if let Ok(f) = tok.parse::<f64>() {
                    pick_rate = f;
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
    }

    (play, win, pick_rate, close_paren + 1)
}

/// Extracts all `SummonerSpells(...)` entries from OP.GG MCP text into a JSON array
/// compatible with `extract_summoner_spell_pairs_for_mode`.
pub fn extract_mcp_summoner_spell_entries(
    text: &str,
    all_entries: &[ItemStatEntry],
) -> serde_json::Value {
    let spell_fields = parse_class_fields(text, "class SummonerSpells:");
    let non_id_fields: Vec<&str> = spell_fields
        .into_iter()
        .filter(|&f| f != "ids" && f != "ids_names")
        .collect();

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
        let (play, win, pick_rate, advance) = parse_mcp_spell_stats(rest_after_bracket, &non_id_fields);
        search_slice = &rest_after_bracket[advance..];

        if ids.len() >= 2 {
            spell_combos.push(serde_json::json!({
                "ids": [ids[0], ids[1]],
                "play": play,
                "win": win,
                "pick_rate": pick_rate
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
                    "play": entry.play,
                    "win": entry.win,
                    "pick_rate": entry.pick_rate
                }));
            }
        }
    }

    serde_json::json!({ "summoner_spells": spell_combos })
}

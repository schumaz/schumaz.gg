use super::super::models::RunePageOption;

fn extract_balanced_paren_content(after_open_paren: &str) -> Option<(&str, usize)> {
    let mut depth = 1usize;
    let mut in_quotes = false;
    let mut escaped = false;

    for (idx, ch) in after_open_paren.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if in_quotes {
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_quotes = false;
            }
            continue;
        }
        match ch {
            '"' => in_quotes = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((&after_open_paren[..idx], idx + 1));
                }
            }
            _ => {}
        }
    }
    None
}

fn split_top_level_args(inside: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut start = 0;
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut in_quotes = false;
    let mut escaped = false;

    for (idx, ch) in inside.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if in_quotes {
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_quotes = false;
            }
            continue;
        }
        match ch {
            '"' => in_quotes = true,
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            ',' if bracket_depth == 0 && paren_depth == 0 => {
                args.push(inside[start..idx].trim());
                start = idx + 1;
            }
            _ => {}
        }
    }
    let tail = inside[start..].trim();
    if !tail.is_empty() || !args.is_empty() {
        args.push(tail);
    }
    args
}

fn parse_int_array(arg: &str) -> Option<Vec<i32>> {
    let trimmed = arg.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return None;
    }
    let inner = trimmed[1..trimmed.len() - 1].trim();
    if inner.is_empty() {
        return None;
    }
    let mut nums = Vec::new();
    for part in inner.split(',') {
        if let Ok(v) = part.trim().parse::<i32>() {
            nums.push(v);
        } else {
            return None;
        }
    }
    Some(nums)
}

fn parse_runes_class_fields(text: &str) -> Vec<&str> {
    if let Some(def_idx) = text.find("class Runes:") {
        let line_rest = &text[def_idx + 12..];
        let end_line = line_rest.find('\n').unwrap_or(line_rest.len());
        line_rest[..end_line]
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    }
}

/// Parses a single `Runes(...)` argument block into a `RunePageOption`.
/// Supports legacy `(8100,[...],8200,[...],[...])` as well as OP.GG MCP schemas
/// containing `id`, tree names (e.g. `"Domination"`, `"Precision"`), rune name arrays,
/// and trailing `play`, `win`, `pick_rate` fields.
pub fn parse_mcp_runes_args(inside: &str, class_fields: &[&str]) -> Option<RunePageOption> {
    let args = split_top_level_args(inside);
    if args.is_empty() {
        return None;
    }

    let mut primary_style_id: Option<i32> = None;
    let mut sub_style_id: Option<i32> = None;
    let mut primary_perks: Option<Vec<i32>> = None;
    let mut secondary_perks: Option<Vec<i32>> = None;
    let mut stat_mods: Option<Vec<i32>> = None;
    let mut play: i64 = 1;
    let mut win: i64 = 1;
    let mut pick_rate: f64 = 1.0;

    if !class_fields.is_empty() && class_fields.len() == args.len() {
        for (field_name, raw_val) in class_fields.iter().zip(args.iter()) {
            match *field_name {
                "primary_page_id" | "primary_style_id" => {
                    primary_style_id = raw_val.parse::<i32>().ok();
                }
                "primary_rune_ids" | "primary_perk_ids" => {
                    primary_perks = parse_int_array(raw_val);
                }
                "secondary_page_id" | "sub_style_id" | "sub_page_id" => {
                    sub_style_id = raw_val.parse::<i32>().ok();
                }
                "secondary_rune_ids" | "sub_perk_ids" => {
                    secondary_perks = parse_int_array(raw_val);
                }
                "stat_mod_ids" | "stat_shards" => {
                    stat_mods = parse_int_array(raw_val);
                }
                "play" => {
                    if let Ok(v) = raw_val.parse::<i64>() {
                        play = v;
                    }
                }
                "win" => {
                    if let Ok(v) = raw_val.parse::<i64>() {
                        win = v;
                    }
                }
                "pick_rate" => {
                    if let Ok(v) = raw_val.parse::<f64>() {
                        pick_rate = v;
                    }
                }
                _ => {}
            }
        }
    } else {
        // Structural fallback when `class Runes:` header is absent or has a different arity
        let p_idx = args
            .iter()
            .position(|a| parse_int_array(a).is_some_and(|v| v.len() == 4))?;
        primary_perks = parse_int_array(args[p_idx]);

        // Last integer before primary_perks is primary_page_id (handles optional leading `id` and trailing tree name)
        primary_style_id = args[..p_idx]
            .iter()
            .filter_map(|a| a.parse::<i32>().ok())
            .next_back();

        let s_style_rel = args[p_idx + 1..]
            .iter()
            .position(|a| a.parse::<i32>().is_ok())?;
        let s_style_idx = p_idx + 1 + s_style_rel;
        sub_style_id = args[s_style_idx].parse::<i32>().ok();

        let s_rel = args[s_style_idx + 1..]
            .iter()
            .position(|a| parse_int_array(a).is_some_and(|v| v.len() == 2))?;
        let s_idx = s_style_idx + 1 + s_rel;
        secondary_perks = parse_int_array(args[s_idx]);

        let m_rel = args[s_idx + 1..]
            .iter()
            .position(|a| parse_int_array(a).is_some_and(|v| v.len() == 3))?;
        let m_idx = s_idx + 1 + m_rel;
        stat_mods = parse_int_array(args[m_idx]);

        let trailing_scalars: Vec<&str> = args[m_idx + 1..]
            .iter()
            .copied()
            .filter(|a| !a.starts_with('[') && !a.starts_with('"'))
            .collect();
        if let Some(Ok(p)) = trailing_scalars.first().map(|s| s.parse::<i64>()) {
            play = p;
        }
        if let Some(Ok(w)) = trailing_scalars.get(1).map(|s| s.parse::<i64>()) {
            win = w;
        }
        if let Some(Ok(pr)) = trailing_scalars.get(2).map(|s| s.parse::<f64>()) {
            pick_rate = pr;
        }
    }

    let p_style = primary_style_id?;
    let s_style = sub_style_id?;
    if p_style <= 0 || s_style <= 0 || p_style == s_style {
        return None;
    }

    let mut selected_perk_ids = Vec::with_capacity(9);
    selected_perk_ids.extend(primary_perks?);
    selected_perk_ids.extend(secondary_perks?);
    selected_perk_ids.extend(stat_mods?);

    if selected_perk_ids.len() != 9 {
        return None;
    }

    Some(RunePageOption {
        primary_style_id: p_style,
        sub_style_id: s_style,
        selected_perk_ids,
        play,
        win,
        pick_rate,
        ..Default::default()
    })
}

/// Extracts all Runes(...) blocks from MCP text output into RunePageOption instances.
pub fn extract_all_mcp_runes(text: &str) -> Vec<RunePageOption> {
    let class_fields = parse_runes_class_fields(text);
    let mut results = Vec::new();
    let mut cursor = 0;

    while let Some(rel_idx) = text[cursor..].find("Runes(") {
        let runes_idx = cursor + rel_idx;
        let after_runes = &text[runes_idx + 6..];

        if let Some((inside, consumed)) = extract_balanced_paren_content(after_runes) {
            if let Some(page) = parse_mcp_runes_args(inside, &class_fields) {
                let is_dup = results.iter().any(|existing: &RunePageOption| {
                    existing.primary_style_id == page.primary_style_id
                        && existing.sub_style_id == page.sub_style_id
                        && existing.selected_perk_ids == page.selected_perk_ids
                });
                if !is_dup {
                    results.push(page);
                }
            }
            cursor = runes_idx + 6 + consumed;
        } else {
            cursor = runes_idx + 6;
        }
    }

    results.truncate(3);
    results
}

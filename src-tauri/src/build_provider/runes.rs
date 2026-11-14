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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_top_rune_pages_multiple_sorted_by_play() {
        let json = serde_json::json!({
            "runes": [
                {
                    "primary_page_id": 8100,
                    "secondary_page_id": 8200,
                    "primary_rune_ids": [8112, 8126, 8138, 8135],
                    "secondary_rune_ids": [8226, 8210],
                    "stat_mod_ids": [5008, 5008, 5001],
                    "play": 12000,
                    "win": 6200
                },
                {
                    "primary_page_id": 8000,
                    "secondary_page_id": 8100,
                    "primary_rune_ids": [8005, 9111, 9104, 8014],
                    "secondary_rune_ids": [8139, 8135],
                    "stat_mod_ids": [5008, 5008, 5003],
                    "play": 45000,
                    "win": 23500
                },
                {
                    "primary_page_id": 8200,
                    "secondary_page_id": 8300,
                    "primary_rune_ids": [8214, 8226, 8210, 8236],
                    "secondary_rune_ids": [8304, 8345],
                    "stat_mod_ids": [5008, 5008, 5001],
                    "play": 8000,
                    "win": 4100
                },
                {
                    "primary_page_id": 8400,
                    "secondary_page_id": 8200,
                    "primary_rune_ids": [8437, 8401, 8473, 8484],
                    "secondary_rune_ids": [8226, 8210],
                    "stat_mod_ids": [5008, 5002, 5001],
                    "play": 2000,
                    "win": 950
                }
            ]
        });

        let pages = extract_top_rune_pages(&json);
        assert_eq!(pages.len(), 3);
        // 1st: 8000 (45,000 games)
        assert_eq!(pages[0].primary_style_id, 8000);
        assert_eq!(pages[0].play, 45000);
        // 2nd: 8100 (12,000 games)
        assert_eq!(pages[1].primary_style_id, 8100);
        assert_eq!(pages[1].play, 12000);
        // 3rd: 8200 (8,000 games)
        assert_eq!(pages[2].primary_style_id, 8200);
        assert_eq!(pages[2].play, 8000);
    }

    #[test]
    fn test_extract_all_mcp_runes_multiple_pages() {
        let text = "LolGetChampionAnalysis(Data(Summary(AverageStats(0.51,0.12)),CoreItems([3118,4645,3157]),CoreItems([3020]),CoreItems([1056,2003]),Runes(8100,[8112,8139,8140,8106],8200,[8210,8226],[5005,5008,5001]),Runes(8000,[8005,9111,9104,8014],8100,[8139,8135],[5008,5008,5001])))";
        let pages = extract_all_mcp_runes(text);
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].primary_style_id, 8100);
        assert_eq!(pages[1].primary_style_id, 8000);
    }

    #[test]
    fn test_extract_all_mcp_runes_with_id_and_tree_names() {
        let text = "class Runes: id,primary_page_id,primary_page_name,primary_rune_ids,primary_rune_names,secondary_page_id,secondary_page_name,secondary_rune_ids,secondary_rune_names,stat_mod_ids,stat_mod_names,play,win,pick_rate\n\nLolGetChampionAnalysis(Data(Runes(8128,8100,\"Domination\",[8128,8126,8140,8106],[\"Dark Harvest\",\"Cheap Shot\",\"Grisly Mementos\",\"Ultimate Hunter\"],8000,\"Precision\",[8009,8014],[\"Presence of Mind\",\"Coup de Grace\"],[5008,5008,5001],[5008,5008,5001],24553,12358,0.32)))";
        let pages = extract_all_mcp_runes(text);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].primary_style_id, 8100);
        assert_eq!(pages[0].sub_style_id, 8000);
        assert_eq!(
            pages[0].selected_perk_ids,
            vec![8128, 8126, 8140, 8106, 8009, 8014, 5008, 5008, 5001]
        );
        assert_eq!(pages[0].play, 24553);
        assert_eq!(pages[0].win, 12358);
    }
}

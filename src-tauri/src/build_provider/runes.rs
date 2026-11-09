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

/// Extracts all Runes(...) blocks from MCP text output into RunePageOption instances.
pub fn extract_all_mcp_runes(text: &str) -> Vec<RunePageOption> {
    let mut results = Vec::new();
    let mut cursor = 0;

    while let Some(rel_idx) = text[cursor..].find("Runes(") {
        let runes_idx = cursor + rel_idx;
        let after_runes = &text[runes_idx + 6..];

        if let Some(comma1) = after_runes.find(',')
            && let Ok(primary_style_id) = after_runes[..comma1].trim().parse::<i32>()
            && let (Some(p_open), Some(p_close)) = (after_runes.find('['), after_runes.find(']'))
            && p_open < p_close
            && p_open > comma1
        {
            let primary_perks: Vec<i32> = after_runes[p_open + 1..p_close]
                .split(',')
                .filter_map(|s| s.trim().parse::<i32>().ok())
                .collect();

            let after_primary = &after_runes[p_close + 1..];
            let after_primary_trim =
                after_primary.trim_start_matches(|c: char| c == ',' || c.is_whitespace());

            if let Some(comma2) = after_primary_trim.find(',')
                && let Ok(sub_style_id) = after_primary_trim[..comma2].trim().parse::<i32>()
                && let (Some(s_open), Some(s_close)) =
                    (after_primary_trim.find('['), after_primary_trim.find(']'))
                && s_open < s_close
                && s_open > comma2
            {
                let secondary_perks: Vec<i32> = after_primary_trim[s_open + 1..s_close]
                    .split(',')
                    .filter_map(|s| s.trim().parse::<i32>().ok())
                    .collect();

                let after_sec = &after_primary_trim[s_close + 1..];
                if let (Some(m_open), Some(m_close)) = (after_sec.find('['), after_sec.find(']'))
                    && m_open < m_close
                {
                    let stat_mods: Vec<i32> = after_sec[m_open + 1..m_close]
                        .split(',')
                        .filter_map(|s| s.trim().parse::<i32>().ok())
                        .collect();

                    let mut selected_perk_ids = Vec::with_capacity(9);
                    selected_perk_ids.extend(primary_perks);
                    selected_perk_ids.extend(secondary_perks);
                    selected_perk_ids.extend(stat_mods);

                    if selected_perk_ids.len() == 9 && primary_style_id != sub_style_id {
                        let is_dup = results.iter().any(|existing: &RunePageOption| {
                            existing.primary_style_id == primary_style_id
                                && existing.sub_style_id == sub_style_id
                                && existing.selected_perk_ids == selected_perk_ids
                        });
                        if !is_dup {
                            results.push(RunePageOption {
                                primary_style_id,
                                sub_style_id,
                                selected_perk_ids,
                                play: 1,
                                win: 1,
                                pick_rate: 1.0,
                                ..Default::default()
                            });
                        }
                    }
                }
            }
        }
        cursor = runes_idx + 6;
    }

    results.truncate(3);
    results
}

/// Helper to extract the most played summoner spells combination from OP.GG analysis JSON.
/// OP.GG REST returns `summoner_spells: [ { "ids": [4, 21], "play": 62087, "win": ... }, ... ]`
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
}

use super::super::models::ItemStatEntry;

pub fn extract_all_core_items_from_text(text: &str) -> Vec<ItemStatEntry> {
    let mut entries = Vec::new();
    let mut search_slice = text;
    let prefixes = [
        "CoreItems(",
        "FourthItem(",
        "FifthItem(",
        "SixthItem(",
        "Boots(",
        "Boot(",
    ];
    loop {
        let next_match = prefixes
            .iter()
            .filter_map(|p| search_slice.find(p).map(|idx| (idx, p.len())))
            .min_by_key(|(idx, _)| *idx);
        let Some((ci_idx, prefix_len)) = next_match else {
            break;
        };
        let after_ci = &search_slice[ci_idx + prefix_len..];
        let maybe_brackets = after_ci
            .find('[')
            .zip(after_ci.find(']'))
            .filter(|(open, close)| open < close);
        if let Some((open_bracket, close_bracket)) = maybe_brackets {
            let inside = &after_ci[open_bracket + 1..close_bracket];
            let ids: Vec<i32> = inside
                .split(',')
                .filter_map(|s| s.trim().parse::<i32>().ok())
                .collect();
            if !ids.is_empty() {
                let rest_after_bracket = &after_ci[close_bracket + 1..];
                let mut play = 1;
                let mut win = 0;
                let mut pick_rate = 0.0;
                if let Some(close_paren) = rest_after_bracket.find(')') {
                    let stats_str = &rest_after_bracket[..close_paren];
                    let parts: Vec<&str> = stats_str
                        .trim_start_matches(',')
                        .split(',')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect();
                    if let Some(Ok(p)) = parts.first().map(|s| s.parse::<i64>()) {
                        play = p;
                    }
                    if let Some(Ok(w)) = parts.get(1).map(|s| s.parse::<i64>()) {
                        win = w;
                    }
                    if let Some(Ok(pr)) = parts.get(2).map(|s| s.parse::<f64>()) {
                        pick_rate = pr;
                    }
                }
                entries.push(ItemStatEntry {
                    ids,
                    play,
                    win,
                    pick_rate,
                });
            }
            search_slice = &after_ci[close_bracket + 1..];
            continue;
        }
        break;
    }
    entries
}

/// Extracts separate item lists (`last_items`, `fourth_items`, `fifth_items`, `sixth_items`) from OP.GG MCP text.
pub fn extract_mcp_grouped_item_lists(text: &str) -> Vec<(String, Vec<ItemStatEntry>)> {
    let data_fields: Vec<String> = if let Some(def_idx) = text.find("class Data:") {
        let line_rest = &text[def_idx + 11..];
        let end_line = line_rest.find('\n').unwrap_or(line_rest.len());
        line_rest[..end_line]
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| {
                matches!(
                    s.as_str(),
                    "boots" | "last_items" | "fourth_items" | "fifth_items" | "sixth_items"
                )
            })
            .collect()
    } else {
        Vec::new()
    };

    let mut groups: Vec<Vec<ItemStatEntry>> = Vec::new();
    let Some(data_idx) = text.find("Data(") else {
        return Vec::new();
    };
    let mut slice = &text[data_idx + 5..];
    if let Some(runes_idx) = slice.find("Runes(") {
        slice = &slice[..runes_idx];
    }

    let bytes = slice.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            let rem = &slice[j..];
            if rem.starts_with("CoreItems(")
                || rem.starts_with("FourthItem(")
                || rem.starts_with("FifthItem(")
                || rem.starts_with("SixthItem(")
                || rem.starts_with("Boots(")
                || rem.starts_with("Boot(")
            {
                let mut depth = 1;
                let mut k = i + 1;
                while k < bytes.len() && depth > 0 {
                    if bytes[k] == b'[' {
                        depth += 1;
                    } else if bytes[k] == b']' {
                        depth -= 1;
                    }
                    k += 1;
                }
                let list_str = &slice[i..k];
                let items = extract_all_core_items_from_text(list_str);
                groups.push(items);
                i = k;
                continue;
            }
        }
        i += 1;
    }

    let mut result = Vec::new();
    if !data_fields.is_empty() && data_fields.len() == groups.len() {
        for (name, items) in data_fields.into_iter().zip(groups) {
            result.push((name, items));
        }
    } else {
        let default_names = ["fourth_items", "fifth_items", "sixth_items", "last_items"];
        for (idx, items) in groups.into_iter().enumerate() {
            let name = default_names
                .get(idx)
                .copied()
                .unwrap_or("other")
                .to_string();
            result.push((name, items));
        }
    }
    result
}

/// Helper to parse JSON item entries (either single object, array of ids, or array of item stat objects).
pub fn parse_json_item_entries(val: Option<&serde_json::Value>) -> Vec<ItemStatEntry> {
    let mut entries = Vec::new();
    let Some(v) = val else {
        return entries;
    };
    if let Some(arr) = v.as_array() {
        for elem in arr {
            if let Some(num) = elem.as_i64() {
                entries.push(ItemStatEntry {
                    ids: vec![num as i32],
                    play: 1,
                    win: 0,
                    pick_rate: 0.0,
                });
            } else if let Some(obj) = elem.as_object() {
                let ids: Vec<i32> = obj
                    .get("ids")
                    .and_then(|ids_val| ids_val.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_i64().map(|n| n as i32))
                            .collect()
                    })
                    .unwrap_or_default();
                let play = obj.get("play").and_then(|p| p.as_i64()).unwrap_or(1);
                let win = obj.get("win").and_then(|w| w.as_i64()).unwrap_or(0);
                let pick_rate = obj
                    .get("pick_rate")
                    .and_then(|pr| pr.as_f64())
                    .unwrap_or(0.0);
                if !ids.is_empty() {
                    entries.push(ItemStatEntry {
                        ids,
                        play,
                        win,
                        pick_rate,
                    });
                }
            }
        }
    } else if let Some(obj) = v.as_object() {
        let ids: Vec<i32> = obj
            .get("ids")
            .and_then(|ids_val| ids_val.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_i64().map(|n| n as i32))
                    .collect()
            })
            .unwrap_or_default();
        let play = obj.get("play").and_then(|p| p.as_i64()).unwrap_or(1);
        let win = obj.get("win").and_then(|w| w.as_i64()).unwrap_or(0);
        let pick_rate = obj
            .get("pick_rate")
            .and_then(|pr| pr.as_f64())
            .unwrap_or(0.0);
        if !ids.is_empty() {
            entries.push(ItemStatEntry {
                ids,
                play,
                win,
                pick_rate,
            });
        }
    }
    entries
}

// Builds the 5 item blocks conforming to the user's requirements:
// 1. Itens Iniciais (Starter + Potion)
// 2. Primeiros 3 Itens (Top most played Core Build combination of 3 non-boot items)
// 3. 3 Botas Mais Feitas (Always strictly ordered by most absolute games played: play descending)
// 4. Build Final (6 items: 3 Core items + #1 most played 4th item + #1 most played 5th item + #1 most played 6th item)
// 5. Itens Situacionais / Úteis (Non-boot items not in core or final build, strictly ordered by play descending)


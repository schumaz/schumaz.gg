use super::super::analytics::mcp_dsl::{McpDocument, McpTuple, McpValue};
use super::super::models::RunePageOption;

fn parse_mcp_runes_tuple(doc: &McpDocument<'_>, tuple: &McpTuple<'_>) -> Option<RunePageOption> {
    let args = &tuple.args;
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

    if let Some(named) = doc.named_args(tuple) {
        for (field_name, val) in named {
            match field_name {
                "primary_page_id" | "primary_style_id" => primary_style_id = val.as_i32(),
                "primary_rune_ids" | "primary_perk_ids" => primary_perks = val.as_i32_list(),
                "secondary_page_id" | "sub_style_id" | "sub_page_id" => {
                    sub_style_id = val.as_i32();
                }
                "secondary_rune_ids" | "sub_perk_ids" => secondary_perks = val.as_i32_list(),
                "stat_mod_ids" | "stat_shards" => stat_mods = val.as_i32_list(),
                "play" => {
                    if let Some(v) = val.as_i64() {
                        play = v;
                    }
                }
                "win" => {
                    if let Some(v) = val.as_i64() {
                        win = v;
                    }
                }
                "pick_rate" => {
                    if let Some(v) = val.as_f64() {
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
            .position(|a| a.as_i32_list().is_some_and(|v| v.len() == 4))?;
        primary_perks = args[p_idx].as_i32_list();
        primary_style_id = args[..p_idx].iter().filter_map(McpValue::as_i32).next_back();

        let s_style_rel = args[p_idx + 1..].iter().position(|a| a.as_i32().is_some())?;
        let s_style_idx = p_idx + 1 + s_style_rel;
        sub_style_id = args[s_style_idx].as_i32();

        let s_rel = args[s_style_idx + 1..]
            .iter()
            .position(|a| a.as_i32_list().is_some_and(|v| v.len() == 2))?;
        let s_idx = s_style_idx + 1 + s_rel;
        secondary_perks = args[s_idx].as_i32_list();

        let m_rel = args[s_idx + 1..]
            .iter()
            .position(|a| a.as_i32_list().is_some_and(|v| v.len() == 3))?;
        let m_idx = s_idx + 1 + m_rel;
        stat_mods = args[m_idx].as_i32_list();

        let trailing_scalars: Vec<&McpValue<'_>> = args[m_idx + 1..]
            .iter()
            .filter(|a| matches!(a, McpValue::Int(_) | McpValue::Float(_)))
            .collect();
        if let Some(p) = trailing_scalars.first().and_then(|v| v.as_i64()) {
            play = p;
        }
        if let Some(w) = trailing_scalars.get(1).and_then(|v| v.as_i64()) {
            win = w;
        }
        if let Some(pr) = trailing_scalars.get(2).and_then(|v| v.as_f64()) {
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

/// Extracts all `Runes(...)` tuples from a pre-parsed `McpDocument` (up to 3 unique setups).
pub fn extract_all_mcp_runes_from_doc(doc: &McpDocument<'_>) -> Vec<RunePageOption> {
    let mut results = Vec::new();
    for tuple in doc.find_tuples(&["Runes"]) {
        if let Some(page) = parse_mcp_runes_tuple(doc, tuple) {
            let is_dup = results.iter().any(|existing: &RunePageOption| {
                existing.primary_style_id == page.primary_style_id
                    && existing.sub_style_id == page.sub_style_id
                    && existing.selected_perk_ids == page.selected_perk_ids
            });
            if !is_dup {
                results.push(page);
            }
        }
        if results.len() >= 3 {
            break;
        }
    }
    results
}

/// Extracts all `Runes(...)` blocks from MCP text output into `RunePageOption` instances.
pub fn extract_all_mcp_runes(text: &str) -> Vec<RunePageOption> {
    let doc = McpDocument::parse(text);
    extract_all_mcp_runes_from_doc(&doc)
}

use super::super::items::{is_boot_in, is_component_in, is_starter_in};
use super::super::models::{ItemClassificationCache, ItemStatEntry, SpellMetadata};
use super::mcp_dsl::{McpDocument, McpTuple, McpValue};
use super::mcp_stats::is_known_summoner_spell_id;
use std::collections::HashMap;

const ITEM_TUPLE_NAMES: &[&str] = &[
    "CoreItems", "FourthItem", "FourthItems", "FifthItem", "FifthItems", "SixthItem", "SixthItems",
    "Boots", "Boot", "StarterItems", "StarterItem", "LastItem", "LastItems",
];

#[derive(Debug, Clone)]
pub struct ParsedMcpItemEntry {
    pub entry: ItemStatEntry,
    pub has_explicit_play: bool,
}

#[derive(Debug, Default)]
pub struct McpExtractedSections {
    pub position: Option<String>,
    pub damage_type: Option<String>,
    pub starter_entries: Vec<ItemStatEntry>,
    pub boot_entries: Vec<ItemStatEntry>,
    pub core_entries: Vec<ItemStatEntry>,
    pub core_has_explicit_play: bool,
    pub fourth_entries: Vec<ItemStatEntry>,
    pub fifth_entries: Vec<ItemStatEntry>,
    pub sixth_entries: Vec<ItemStatEntry>,
    pub last_entries: Vec<ItemStatEntry>,
    pub spell_fallback_entries: Vec<ItemStatEntry>,
}

fn is_item_tuple_name(name: &str) -> bool {
    ITEM_TUPLE_NAMES.contains(&name)
}

fn parse_mcp_item_tuple(doc: &McpDocument<'_>, tuple: &McpTuple<'_>) -> Option<ParsedMcpItemEntry> {
    if let Some(named) = doc.named_args(tuple) {
        let (mut ids, mut play, mut win, mut pick_rate) = (None, None, 0i64, 0.0f64);
        for (field, val) in named {
            match field {
                "ids" => ids = val.as_i32_list(),
                "play" => play = val.as_i64(),
                "win" => win = val.as_i64().unwrap_or(0),
                "pick_rate" => pick_rate = val.as_f64().unwrap_or(0.0),
                _ => {}
            }
        }
        let ids = ids.filter(|v| !v.is_empty())?;
        return Some(ParsedMcpItemEntry {
            has_explicit_play: play.is_some(),
            entry: ItemStatEntry { ids, play: play.unwrap_or(1), win, pick_rate },
        });
    }

    let ids_idx = tuple.args.iter().position(|a| a.as_i32_list().is_some())?;
    let ids = tuple.args[ids_idx].as_i32_list().filter(|v| !v.is_empty())?;
    let scalars: Vec<&McpValue<'_>> = tuple.args[ids_idx + 1..]
        .iter().filter(|v| matches!(v, McpValue::Int(_) | McpValue::Float(_))).collect();
    let play_opt = scalars.first().and_then(|v| v.as_i64());
    let win = scalars.get(1).and_then(|v| v.as_i64()).unwrap_or(0);
    let pick_rate = scalars.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0);
    Some(ParsedMcpItemEntry {
        has_explicit_play: play_opt.is_some(),
        entry: ItemStatEntry { ids, play: play_opt.unwrap_or(1), win, pick_rate },
    })
}

fn collect_parsed_entries(doc: &McpDocument<'_>, val: &McpValue<'_>) -> Vec<ParsedMcpItemEntry> {
    match val {
        McpValue::Tuple(t) if is_item_tuple_name(t.name) => parse_mcp_item_tuple(doc, t).into_iter().collect(),
        McpValue::List(items) => items.iter().filter_map(McpValue::as_tuple)
            .filter(|t| is_item_tuple_name(t.name)).filter_map(|t| parse_mcp_item_tuple(doc, t)).collect(),
        _ => Vec::new(),
    }
}

fn collect_entries(doc: &McpDocument<'_>, val: &McpValue<'_>) -> Vec<ItemStatEntry> {
    collect_parsed_entries(doc, val).into_iter().map(|p| p.entry).collect()
}

fn is_spell_pair_ids(ids: &[i32], spells_catalog: Option<&HashMap<i32, SpellMetadata>>) -> bool {
    let known = |id: i32| spells_catalog.is_some_and(|c| c.contains_key(&id)) || is_known_summoner_spell_id(id);
    ids.len() == 2 && known(ids[0]) && known(ids[1])
}

fn is_starter_entry_ids(cls: &ItemClassificationCache, ids: &[i32]) -> bool {
    !ids.is_empty()
        && ids.iter().any(|&id| is_starter_in(cls, id))
        && ids.iter().all(|&id| is_starter_in(cls, id) || is_component_in(cls, id))
}

/// Extracts all item tuples from MCP text using the unified DSL AST.
pub fn extract_all_core_items_from_text(text: &str) -> Vec<ItemStatEntry> {
    let doc = McpDocument::parse(text);
    doc.find_tuples(ITEM_TUPLE_NAMES)
        .into_iter()
        .filter_map(|t| parse_mcp_item_tuple(&doc, t).map(|p| p.entry))
        .collect()
}

/// Extracts grouped item lists (`boots`, `last_items`, `fourth_items`, `fifth_items`, `sixth_items`)
/// from a pre-parsed `McpDocument`.
pub fn extract_mcp_grouped_item_lists_doc(doc: &McpDocument<'_>) -> Vec<(String, Vec<ItemStatEntry>)> {
    let Some(data_tuple) = doc.find_tuples(&["Data"]).into_iter().next() else {
        return Vec::new();
    };
    let grouped_field_names = ["boots", "last_items", "fourth_items", "fifth_items", "sixth_items"];

    if let Some(named) = doc.named_args(data_tuple) {
        return named
            .into_iter()
            .filter(|(field, _)| grouped_field_names.contains(field))
            .map(|(field, val)| (field.to_string(), collect_entries(doc, val)))
            .collect();
    }

    let groups: Vec<Vec<ItemStatEntry>> = data_tuple
        .args
        .iter()
        .filter_map(|arg| arg.as_list().map(|_| collect_entries(doc, arg)))
        .filter(|items| !items.is_empty())
        .collect();

    let data_fields: Vec<&str> = doc
        .fields_for(&["Data"])
        .map(|fields| fields.iter().copied().filter(|f| grouped_field_names.contains(f)).collect())
        .unwrap_or_default();

    if !data_fields.is_empty() && data_fields.len() == groups.len() {
        return data_fields
            .into_iter()
            .zip(groups)
            .map(|(name, items)| (name.to_string(), items))
            .collect();
    }
    let default_names = ["fourth_items", "fifth_items", "sixth_items", "last_items"];
    groups
        .into_iter()
        .enumerate()
        .map(|(idx, items)| (default_names.get(idx).copied().unwrap_or("other").to_string(), items))
        .collect()
}

/// Extracts separate item lists (`last_items`, `fourth_items`, `fifth_items`, `sixth_items`) from OP.GG MCP text.
pub fn extract_mcp_grouped_item_lists(text: &str) -> Vec<(String, Vec<ItemStatEntry>)> {
    extract_mcp_grouped_item_lists_doc(&McpDocument::parse(text))
}

fn route_named_data_arg(
    doc: &McpDocument<'_>,
    field: &str,
    val: &McpValue<'_>,
    out: &mut McpExtractedSections,
) {
    match field {
        "position" => if let Some(p) = val.as_str().map(str::trim).filter(|s| !s.is_empty()) { out.position = Some(p.to_string()); },
        "damage_type" => if let Some(d) = val.as_str().map(str::trim).filter(|s| !s.is_empty()) { out.damage_type = Some(d.to_string()); },
        "summary" => if out.position.is_none()
            && let Some(summary) = val.as_tuple()
            && let Some(pos) = doc.field(summary, "position").and_then(McpValue::as_str).map(str::trim).filter(|s| !s.is_empty())
        {
            out.position = Some(pos.to_string());
        },
        "boots" => out.boot_entries.extend(collect_entries(doc, val)),
        "starter_items" | "starters" => out.starter_entries.extend(collect_entries(doc, val)),
        "core_items" => for p in collect_parsed_entries(doc, val) {
            out.core_has_explicit_play |= p.has_explicit_play;
            out.core_entries.push(p.entry);
        },
        "fourth_items" => out.fourth_entries.extend(collect_entries(doc, val)),
        "fifth_items" => out.fifth_entries.extend(collect_entries(doc, val)),
        "sixth_items" => out.sixth_entries.extend(collect_entries(doc, val)),
        "last_items" => out.last_entries.extend(collect_entries(doc, val)),
        "summoner_spells" | "spells" => out.spell_fallback_entries.extend(collect_entries(doc, val)),
        _ => {}
    }
}

fn route_semantic_item_entry(
    cls: &ItemClassificationCache,
    spells_catalog: Option<&HashMap<i32, SpellMetadata>>,
    ctor_name: &str,
    parsed: ParsedMcpItemEntry,
    out: &mut McpExtractedSections,
) {
    match ctor_name {
        "Boots" | "Boot" => out.boot_entries.push(parsed.entry),
        "StarterItems" | "StarterItem" => out.starter_entries.push(parsed.entry),
        "FourthItem" | "FourthItems" => out.fourth_entries.push(parsed.entry),
        "FifthItem" | "FifthItems" => out.fifth_entries.push(parsed.entry),
        "SixthItem" | "SixthItems" => out.sixth_entries.push(parsed.entry),
        "LastItem" | "LastItems" => out.last_entries.push(parsed.entry),
        _ => {
            let ids = &parsed.entry.ids;
            if is_spell_pair_ids(ids, spells_catalog) {
                out.spell_fallback_entries.push(parsed.entry);
            } else if ids.iter().all(|&id| is_boot_in(cls, id)) {
                out.boot_entries.push(parsed.entry);
            } else if is_starter_entry_ids(cls, ids) {
                out.starter_entries.push(parsed.entry);
            } else if out.core_entries.is_empty() || ids.len() >= 2 {
                out.core_has_explicit_play |= parsed.has_explicit_play;
                out.core_entries.push(parsed.entry);
            } else {
                out.last_entries.push(parsed.entry);
            }
        }
    }
}

fn route_list_arg(
    doc: &McpDocument<'_>,
    cls: &ItemClassificationCache,
    spells_catalog: Option<&HashMap<i32, SpellMetadata>>,
    items: &[McpValue<'_>],
    out: &mut McpExtractedSections,
) {
    let mut generic = Vec::new();
    for item in items {
        let Some(t) = item.as_tuple() else { continue };
        if !is_item_tuple_name(t.name) {
            continue;
        }
        let Some(parsed) = parse_mcp_item_tuple(doc, t) else {
            continue;
        };
        if t.name == "CoreItems" {
            generic.push(parsed.entry);
        } else {
            route_semantic_item_entry(cls, spells_catalog, t.name, parsed, out);
        }
    }
    if generic.is_empty() {
        return;
    }
    if generic.iter().all(|e| !e.ids.is_empty() && e.ids.iter().all(|&id| is_boot_in(cls, id))) {
        out.boot_entries.extend(generic);
    } else if generic.iter().all(|e| is_starter_entry_ids(cls, &e.ids)) {
        out.starter_entries.extend(generic);
    } else if out.fourth_entries.is_empty() {
        out.fourth_entries = generic;
    } else if out.fifth_entries.is_empty() {
        out.fifth_entries = generic;
    } else if out.sixth_entries.is_empty() {
        out.sixth_entries = generic;
    } else {
        out.last_entries.extend(generic);
    }
}

/// Extracts structured sections (`position`, `damage_type`, starters, boots, core, 4th/5th/6th,
/// last items, and spell fallbacks) from a parsed `McpDocument` via schema-first routing with
/// semantic classification fallback (zero blind tuple indices).
pub fn extract_mcp_data_sections(
    doc: &McpDocument<'_>,
    cls: &ItemClassificationCache,
    spells_catalog: Option<&HashMap<i32, SpellMetadata>>,
) -> McpExtractedSections {
    let mut out = McpExtractedSections::default();
    let position_from = |tuple, field| {
        doc.field(tuple, field)
            .and_then(McpValue::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    if let Some(pos_tuple) = doc.find_tuples(&["Position"]).into_iter().next() {
        out.position = pos_tuple.args.first().and_then(McpValue::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    }
    if out.position.is_none()
        && let Some(summary) = doc.find_tuples(&["Summary"]).into_iter().next()
    {
        out.position = position_from(summary, "position");
    }

    let Some(data_tuple) = doc.find_tuples(&["Data"]).into_iter().next() else {
        for t in doc.find_tuples(ITEM_TUPLE_NAMES) {
            if let Some(parsed) = parse_mcp_item_tuple(doc, t) {
                route_semantic_item_entry(cls, spells_catalog, t.name, parsed, &mut out);
            }
        }
        return out;
    };
    if let Some(named) = doc.named_args(data_tuple) {
        for (field, val) in named {
            route_named_data_arg(doc, field, val, &mut out);
        }
        return out;
    }
    for arg in &data_tuple.args {
        match arg {
            McpValue::Str(s) if out.damage_type.is_none() && (s == "AP" || s == "AD") => {
                out.damage_type = Some(s.to_string());
            }
            McpValue::Tuple(t) if is_item_tuple_name(t.name) => {
                if let Some(parsed) = parse_mcp_item_tuple(doc, t) {
                    route_semantic_item_entry(cls, spells_catalog, t.name, parsed, &mut out);
                }
            }
            McpValue::List(items) => route_list_arg(doc, cls, spells_catalog, items, &mut out),
            _ => {}
        }
    }
    out
}

fn parse_json_item_obj(obj: &serde_json::Map<String, serde_json::Value>) -> Option<ItemStatEntry> {
    let ids: Vec<i32> = obj.get("ids")?.as_array()?.iter().filter_map(|x| x.as_i64().map(|n| n as i32)).collect();
    if ids.is_empty() {
        return None;
    }
    Some(ItemStatEntry {
        ids,
        play: obj.get("play").and_then(|p| p.as_i64()).unwrap_or(1),
        win: obj.get("win").and_then(|w| w.as_i64()).unwrap_or(0),
        pick_rate: obj.get("pick_rate").and_then(|pr| pr.as_f64()).unwrap_or(0.0),
    })
}

/// Helper to parse JSON item entries (either single object, array of ids, or array of item stat objects).
pub fn parse_json_item_entries(val: Option<&serde_json::Value>) -> Vec<ItemStatEntry> {
    let mut entries = Vec::new();
    let Some(v) = val else { return entries; };
    if let Some(arr) = v.as_array() {
        for elem in arr {
            if let Some(num) = elem.as_i64() {
                entries.push(ItemStatEntry { ids: vec![num as i32], play: 1, win: 0, pick_rate: 0.0 });
            } else if let Some(entry) = elem.as_object().and_then(parse_json_item_obj) {
                entries.push(entry);
            }
        }
    } else if let Some(entry) = v.as_object().and_then(parse_json_item_obj) {
        entries.push(entry);
    }
    entries
}

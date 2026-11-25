use super::super::cooldown::send_analytics_request;
use super::html_extract::normalize_opgg_champion_key;
use super::parsers::{RawMayhemAugmentStat, hydrate_mayhem_augments};
use crate::build_provider::models::DataDragonCache;
use crate::models::{AppLocale, AramAugmentEntry};

pub const OPGG_MCP_ARAM_AUGMENTS_TOOL: &str = "lol_list_aram_augments";

/// Resolves a Riot numeric champion ID from a champion key, display name, or numeric string.
pub fn resolve_champion_id_for_aram(champion_key: &str, ddragon: &DataDragonCache) -> Option<i32> {
    let trimmed = champion_key.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(id) = trimmed.parse::<i32>()
        && id > 0
    {
        return Some(id);
    }
    let normalized = normalize_opgg_champion_key(trimmed);
    if normalized.is_empty() {
        return None;
    }
    ddragon
        .champions
        .values()
        .find(|meta| {
            normalize_opgg_champion_key(&meta.key_name) == normalized
                || normalize_opgg_champion_key(&meta.display_name) == normalized
        })
        .map(|meta| meta.champion_id)
}

/// Calls the official OP.GG MCP `lol_list_aram_augments` tool for a champion and locale.
pub async fn fetch_opgg_mcp_aram_augments(
    client: &reqwest::Client,
    mcp_endpoint: &str,
    champion_id: i32,
    locale: AppLocale,
) -> Result<serde_json::Value, String> {
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": OPGG_MCP_ARAM_AUGMENTS_TOOL,
            "arguments": {
                "champion_id": champion_id,
                "lang": locale.as_str(),
                "desired_output_fields": [
                    "data.augments[].{id,desc,tier,performance,popular}"
                ]
            }
        }
    });

    let cooldown_key = format!(
        "mcp:{mcp_endpoint}:aram_augments:{champion_id}:{}",
        locale.as_str()
    );
    let res = send_analytics_request(
        &cooldown_key,
        client.post(mcp_endpoint).json(&payload),
        "OP.GG MCP ARAM augments",
    )
    .await?;

    let json_body: serde_json::Value = res
        .json()
        .await
        .map_err(|err| format!("Failed to parse OP.GG MCP ARAM augments JSON: {err}"))?;

    check_mcp_error_envelope(&json_body)?;
    Ok(json_body)
}

fn check_mcp_error_envelope(json_body: &serde_json::Value) -> Result<(), String> {
    if let Some(err) = json_body.get("error") {
        let msg = err
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown JSON-RPC error");
        return Err(format!("OP.GG MCP JSON-RPC error: {msg}"));
    }
    if json_body
        .pointer("/result/isError")
        .and_then(|v| v.as_bool())
        == Some(true)
    {
        let msg = json_body
            .pointer("/result/content/0/text")
            .and_then(|v| v.as_str())
            .unwrap_or("tool returned isError=true");
        return Err(format!("OP.GG MCP tool error: {msg}"));
    }
    Ok(())
}

fn parse_augment_class_fields(text: &str) -> Vec<String> {
    for line in text.lines() {
        if let Some(rest) = line.trim().strip_prefix("class Augment:") {
            let fields: Vec<String> = rest
                .split(',')
                .map(|s| s.trim().to_ascii_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
            if !fields.is_empty() {
                return fields;
            }
        }
    }
    Vec::new()
}

fn split_dsl_args(args_slice: &str) -> Vec<&str> {
    let bytes = args_slice.as_bytes();
    let mut parts = Vec::new();
    let (mut start, mut paren_depth, mut bracket_depth) = (0usize, 0usize, 0usize);
    let (mut in_string, mut escaped) = (false, false);

    for (i, &b) in bytes.iter().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b',' if paren_depth == 0 && bracket_depth == 0 => {
                parts.push(args_slice[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    if start <= args_slice.len() {
        let tail = args_slice[start..].trim();
        if !tail.is_empty() || !parts.is_empty() {
            parts.push(tail);
        }
    }
    parts
}

fn decode_dsl_string(token: &str) -> Option<String> {
    let trimmed = token.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") || trimmed == "None" {
        return None;
    }
    if trimmed.starts_with('"') {
        serde_json::from_str::<String>(trimmed).ok()
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_dsl_augment_tuple(
    args: &[&str],
    header_fields: &[String],
) -> Option<RawMayhemAugmentStat> {
    let inferred: &[&str] = match args.len() {
        4 => &["id", "tier", "performance", "popular"],
        5 => &["id", "desc", "tier", "performance", "popular"],
        6 => &["id", "name", "desc", "tier", "performance", "popular"],
        _ => &[],
    };
    let use_header = !header_fields.is_empty() && header_fields.len() == args.len();
    let (mut id, mut tier, mut performance, mut popular, mut desc) = (None, 5, 0.0, 0.0, None);

    for (idx, raw_val) in args.iter().enumerate() {
        let field = if use_header {
            header_fields[idx].as_str()
        } else {
            *inferred.get(idx)?
        };
        match field {
            "id" => id = raw_val.parse::<i32>().ok(),
            "tier" => tier = raw_val.parse::<i32>().unwrap_or(tier),
            "performance" => performance = raw_val.parse::<f64>().unwrap_or(performance),
            "popular" => popular = raw_val.parse::<f64>().unwrap_or(popular),
            "desc" => desc = decode_dsl_string(raw_val),
            _ => {}
        }
    }

    Some(RawMayhemAugmentStat {
        id: id?,
        tier,
        performance,
        popular,
        desc,
        tooltip: None,
    })
}

pub(crate) fn parse_mcp_augments_dsl(text: &str) -> Vec<RawMayhemAugmentStat> {
    const PREFIX: &str = "Augment(";
    let header_fields = parse_augment_class_fields(text);
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let (mut cursor, mut in_string, mut escaped) = (0usize, false, false);

    while cursor < bytes.len() {
        if in_string {
            if escaped {
                escaped = false;
            } else if bytes[cursor] == b'\\' {
                escaped = true;
            } else if bytes[cursor] == b'"' {
                in_string = false;
            }
            cursor += 1;
            continue;
        }
        if bytes[cursor] == b'"' {
            in_string = true;
            cursor += 1;
            continue;
        }
        let boundary_ok = cursor == 0
            || (!bytes[cursor - 1].is_ascii_alphanumeric() && bytes[cursor - 1] != b'_');
        if boundary_ok && text[cursor..].starts_with(PREFIX) {
            let args_start = cursor + PREFIX.len();
            let (mut i, mut depth, mut arg_in_str, mut arg_escaped) =
                (args_start, 1usize, false, false);
            while i < bytes.len() && depth > 0 {
                let b = bytes[i];
                if arg_in_str {
                    if arg_escaped {
                        arg_escaped = false;
                    } else if b == b'\\' {
                        arg_escaped = true;
                    } else if b == b'"' {
                        arg_in_str = false;
                    }
                } else {
                    match b {
                        b'"' => arg_in_str = true,
                        b'(' => depth += 1,
                        b')' => depth = depth.saturating_sub(1),
                        _ => {}
                    }
                }
                i += 1;
            }
            if depth == 0 && i > args_start {
                let args = split_dsl_args(&text[args_start..i - 1]);
                if let Some(stat) = parse_dsl_augment_tuple(&args, &header_fields) {
                    out.push(stat);
                }
                cursor = i;
                continue;
            }
            break;
        }
        cursor += 1;
    }
    out
}

fn extract_raw_augments_from_json_value(
    val: &serde_json::Value,
) -> Option<Vec<RawMayhemAugmentStat>> {
    let arr = val
        .pointer("/data/augments")
        .or_else(|| val.pointer("/result/data/augments"))
        .or_else(|| val.get("augments"))
        .or_else(|| val.is_array().then_some(val))?;
    serde_json::from_value::<Vec<RawMayhemAugmentStat>>(arr.clone()).ok()
}

/// Parses an OP.GG MCP `lol_list_aram_augments` response and hydrates assets/metadata via CDragon.
pub fn parse_aram_mayhem_augments_mcp_for_locale(
    json_body: &serde_json::Value,
    ddragon: &DataDragonCache,
    locale: AppLocale,
) -> Result<Vec<AramAugmentEntry>, String> {
    check_mcp_error_envelope(json_body)?;

    let raw_items = if let Some(items) = extract_raw_augments_from_json_value(json_body) {
        items
    } else {
        let text = json_body
            .pointer("/result/content/0/text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                "Missing text payload in OP.GG MCP ARAM augments response".to_string()
            })?;
        let trimmed = text.trim();
        if (trimmed.starts_with('{') || trimmed.starts_with('['))
            && let Ok(parsed_json) = serde_json::from_str::<serde_json::Value>(trimmed)
            && let Some(items) = extract_raw_augments_from_json_value(&parsed_json)
        {
            items
        } else {
            parse_mcp_augments_dsl(text)
        }
    };

    if raw_items.is_empty() {
        return Err("OP.GG MCP ARAM augments payload contained no augments".to_string());
    }
    let hydrated = hydrate_mayhem_augments(raw_items, ddragon, locale);
    if hydrated.is_empty() {
        return Err("OP.GG MCP ARAM augments had no catalogued entries in DataDragon".to_string());
    }
    Ok(hydrated)
}

#[cfg(test)]
#[path = "mcp_augments_tests.rs"]
mod tests;

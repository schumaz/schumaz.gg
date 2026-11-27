use super::super::cooldown::send_analytics_request;
use super::super::mcp_dsl::{McpDocument, McpTuple, McpValue};
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

fn parse_dsl_augment_tuple(
    doc: &McpDocument<'_>,
    tuple: &McpTuple<'_>,
) -> Option<RawMayhemAugmentStat> {
    let inferred: &[&str] = match tuple.args.len() {
        4 => &["id", "tier", "performance", "popular"],
        5 => &["id", "desc", "tier", "performance", "popular"],
        6 => &["id", "name", "desc", "tier", "performance", "popular"],
        _ => &[],
    };
    let named = doc.named_args(tuple);
    let (mut id, mut tier, mut performance, mut popular, mut desc) = (None, 5, 0.0, 0.0, None);

    for (idx, val) in tuple.args.iter().enumerate() {
        let field = if let Some(ref pairs) = named {
            pairs[idx].0
        } else {
            *inferred.get(idx)?
        };
        if field.eq_ignore_ascii_case("id") {
            id = val.as_i32();
        } else if field.eq_ignore_ascii_case("tier") {
            tier = val.as_i32().unwrap_or(tier);
        } else if field.eq_ignore_ascii_case("performance") {
            performance = val.as_f64().unwrap_or(performance);
        } else if field.eq_ignore_ascii_case("popular") {
            popular = val.as_f64().unwrap_or(popular);
        } else if field.eq_ignore_ascii_case("desc") {
            desc = match val {
                McpValue::Null => None,
                _ => val
                    .as_str()
                    .filter(|s| !s.trim().is_empty())
                    .map(str::to_string),
            };
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
    let doc = McpDocument::parse(text);
    doc.find_tuples(&["Augment"])
        .into_iter()
        .filter_map(|tuple| parse_dsl_augment_tuple(&doc, tuple))
        .collect()
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

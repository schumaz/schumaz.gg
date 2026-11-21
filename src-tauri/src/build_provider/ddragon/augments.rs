use crate::build_provider::analytics::aram::clean_augment_description;
use crate::models::{AppLocale, ArenaAugmentMetadata};
use std::collections::HashMap;

pub const CDRAGON_BASE_URL: &str = "https://raw.communitydragon.org/latest";

const CDRAGON_GAME_DATA_PREFIX: &str =
    "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/";

/// Returns the CommunityDragon `cherry-augments.json` URL localized for `locale`.
pub fn cherry_augments_url_for_locale(locale: AppLocale) -> String {
    let cd_locale = match locale {
        AppLocale::PtBr => "pt_br",
        AppLocale::EnUs => "default",
    };
    format!(
        "{}/plugins/rcp-be-lol-game-data/global/{}/v1/cherry-augments.json",
        CDRAGON_BASE_URL, cd_locale
    )
}

/// Returns the CommunityDragon `cdragon/arena/{locale}.json` URL used to supplement augment descriptions.
pub fn cdragon_arena_desc_url_for_locale(locale: AppLocale) -> String {
    let cd_locale = match locale {
        AppLocale::PtBr => "pt_br",
        AppLocale::EnUs => "en_us",
    };
    format!("{}/cdragon/arena/{}.json", CDRAGON_BASE_URL, cd_locale)
}

/// Converts a Riot `cherry-augments.json` asset path (e.g.
/// `/lol-game-data/assets/ASSETS/UX/Cherry/Augments/Icons/FirstAidKit_small.png`)
/// into a cached-protocol-compatible CommunityDragon HTTPS URL.
pub fn resolve_arena_augment_icon_url(raw_path: &str) -> String {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        return trimmed.to_string();
    }
    let lowered = trimmed.to_ascii_lowercase();
    let rel = lowered
        .strip_prefix("/lol-game-data/assets/")
        .or_else(|| lowered.strip_prefix("lol-game-data/assets/"))
        .unwrap_or(lowered.trim_start_matches('/'));
    if rel.is_empty() {
        String::new()
    } else {
        format!("{}{}", CDRAGON_GAME_DATA_PREFIX, rel)
    }
}

fn parse_cherry_rarity(val: Option<&serde_json::Value>) -> i32 {
    if let Some(n) = val.and_then(|v| v.as_i64()) {
        return match n {
            0 | 1 => 1,
            2 | 4 => 4,
            3 | 8 => 8,
            other => other as i32,
        };
    }
    match val
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "ksilver" | "silver" => 1,
        "kgold" | "gold" => 4,
        "kprismatic" | "prismatic" => 8,
        _ => 0,
    }
}

/// Parses CommunityDragon `cherry-augments.json` into a lookup map keyed by numeric augment ID.
pub fn parse_cdragon_cherry_augments(
    json: &serde_json::Value,
) -> HashMap<i32, ArenaAugmentMetadata> {
    let entries = json
        .as_array()
        .or_else(|| json.get("augments").and_then(|v| v.as_array()));
    let Some(arr) = entries else {
        return HashMap::new();
    };

    let mut map = HashMap::with_capacity(arr.len());
    for entry in arr {
        let Some(id) = entry.get("id").and_then(|v| v.as_i64()).map(|n| n as i32) else {
            continue;
        };
        let name = entry
            .get("nameTRA")
            .or_else(|| entry.get("name"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .or_else(|| {
                entry
                    .get("augmentNameId")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
            })
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let icon_raw = entry
            .get("augmentSmallIconPath")
            .or_else(|| entry.get("iconSmall"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                entry
                    .get("augmentLargeIconPath")
                    .or_else(|| entry.get("iconLarge"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("");
        let raw_desc = entry
            .get("summaryDescription")
            .or_else(|| entry.get("desc"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        map.insert(
            id,
            ArenaAugmentMetadata {
                id,
                name,
                desc: clean_augment_description(raw_desc),
                icon_url: resolve_arena_augment_icon_url(icon_raw),
                rarity: parse_cherry_rarity(entry.get("rarity")),
            },
        );
    }

    map
}

fn format_data_value(val: f64) -> String {
    let rounded = (val * 10.0).round() / 10.0;
    if (rounded.fract()).abs() < 1e-6 {
        format!("{}", rounded as i64)
    } else {
        format!("{:.1}", rounded)
    }
}

fn interpolate_arena_desc(
    raw: &str,
    data_values: Option<&serde_json::Map<String, serde_json::Value>>,
) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find('@') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('@') else {
            out.push_str(&rest[start..]);
            rest = "";
            break;
        };
        let token = &after[..end];
        rest = &after[end + 1..];

        let (key, mult) = if let Some((k, m)) = token.split_once('*') {
            (k.trim(), m.trim().parse::<f64>().unwrap_or(1.0))
        } else {
            (token.trim(), 1.0)
        };

        let resolved = data_values.and_then(|dv| {
            let val = dv.get(key)?;
            val.as_f64().or_else(|| {
                val.as_array()
                    .and_then(|a| a.first())
                    .and_then(|x| x.as_f64())
            })
        });

        if let Some(num) = resolved {
            out.push_str(&format_data_value(num * mult));
        }
    }
    out.push_str(rest);
    clean_augment_description(&out)
}

fn normalize_augment_lookup_name(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Supplements empty descriptions (and missing legacy entries) in `augments` using
/// CommunityDragon's `cdragon/arena/{locale}.json`, including normalized name fallback
/// for shared Arena <-> ARAM Mayhem (`id >= 1000`) augments.
pub fn supplement_arena_augment_descriptions(
    augments: &mut HashMap<i32, ArenaAugmentMetadata>,
    arena_json: &serde_json::Value,
) {
    let Some(arr) = arena_json.get("augments").and_then(|v| v.as_array()) else {
        return;
    };

    let mut desc_by_name: HashMap<String, String> = HashMap::with_capacity(arr.len());

    for entry in arr {
        let Some(id) = entry.get("id").and_then(|v| v.as_i64()).map(|n| n as i32) else {
            continue;
        };
        let raw_desc = entry
            .get("desc")
            .or_else(|| entry.get("tooltip"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let data_values = entry.get("dataValues").and_then(|v| v.as_object());
        let cleaned = interpolate_arena_desc(raw_desc, data_values);
        let entry_name = entry
            .get("name")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());

        if !cleaned.is_empty()
            && let Some(name) = entry_name
        {
            let norm = normalize_augment_lookup_name(name);
            if !norm.is_empty() {
                desc_by_name.entry(norm).or_insert_with(|| cleaned.clone());
            }
        }

        if let Some(existing) = augments.get_mut(&id) {
            if existing.desc.is_empty() && !cleaned.is_empty() {
                existing.desc = cleaned;
            }
            if existing.icon_url.is_empty()
                && let Some(icon) = entry.get("iconSmall").and_then(|v| v.as_str())
            {
                existing.icon_url = resolve_arena_augment_icon_url(icon);
            }
        } else if let Some(name) = entry_name {
            let icon = entry
                .get("iconSmall")
                .or_else(|| entry.get("iconLarge"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            augments.insert(
                id,
                ArenaAugmentMetadata {
                    id,
                    name: name.to_string(),
                    desc: cleaned,
                    icon_url: resolve_arena_augment_icon_url(icon),
                    rarity: parse_cherry_rarity(entry.get("rarity")),
                },
            );
        }
    }

    if !desc_by_name.is_empty() {
        for existing in augments.values_mut() {
            if existing.desc.is_empty() {
                let norm = normalize_augment_lookup_name(&existing.name);
                if let Some(desc) = desc_by_name.get(&norm) {
                    existing.desc = desc.clone();
                }
            }
        }
    }
}

/// Best-effort fetch of localized Arena augments metadata (`cherry-augments.json` + optional descriptions).
/// When `base_url` is a custom mock server (tests), queries `{base_url}/cherry-augments.json` only
/// and never touches external hosts.
pub(super) async fn fetch_arena_augments_catalog(
    client: &reqwest::Client,
    base_url: &str,
    locale: AppLocale,
) -> HashMap<i32, ArenaAugmentMetadata> {
    let is_prod = base_url == super::cache::DDRAGON_BASE_URL;
    let cherry_url = if is_prod {
        cherry_augments_url_for_locale(locale)
    } else {
        format!("{}/cherry-augments.json", base_url.trim_end_matches('/'))
    };

    let cherry_fut = async {
        if let Ok(res) = client.get(&cherry_url).send().await
            && res.status().is_success()
            && let Ok(json) = res.json::<serde_json::Value>().await
        {
            return parse_cdragon_cherry_augments(&json);
        }
        HashMap::new()
    };

    if !is_prod {
        return cherry_fut.await;
    }

    let desc_url = cdragon_arena_desc_url_for_locale(locale);
    let desc_fut = async {
        if let Ok(res) = client.get(&desc_url).send().await
            && res.status().is_success()
            && let Ok(json) = res.json::<serde_json::Value>().await
        {
            return Some(json);
        }
        None
    };

    let (mut map, desc_json) = tokio::join!(cherry_fut, desc_fut);
    if let Some(json) = desc_json {
        supplement_arena_augment_descriptions(&mut map, &json);
    }
    map
}

#[cfg(test)]
#[path = "augments_tests.rs"]
mod tests;

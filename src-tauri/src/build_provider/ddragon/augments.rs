use super::augment_interpolate::{
    infer_ability_slot_from_desc, infer_ability_slot_from_name_id,
    interpolate_arena_desc_for_locale, normalize_augment_lookup_name,
};
use crate::build_provider::analytics::aram::{clean_augment_description, is_internal_augment_key};
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

/// Converts a Riot `cherry-augments.json` asset path into a cached-protocol-compatible CDragon URL.
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

fn normalize_augment_api_name(raw: &str) -> String {
    let trimmed = raw.trim();
    let stripped = trimmed
        .get(..5)
        .filter(|p| p.eq_ignore_ascii_case("aram_"))
        .map(|_| &trimmed[5..])
        .unwrap_or(trimmed);
    normalize_augment_lookup_name(stripped)
}

fn strip_quest_name_prefix(raw: &str) -> Option<&str> {
    let (head, tail) = raw.trim().split_once(':')?;
    let h = head.trim();
    (h.eq_ignore_ascii_case("quest")
        || h.eq_ignore_ascii_case("missão")
        || h.eq_ignore_ascii_case("missao"))
    .then(|| tail.trim())
    .filter(|s| !s.is_empty())
}

/// Parses CommunityDragon `cherry-augments.json` into a lookup map keyed by numeric augment ID
/// alongside normalized `augmentNameId` keys (`ARAM_` prefix stripped).
pub(super) fn parse_cdragon_cherry_augments_with_api_names(
    json: &serde_json::Value,
) -> (HashMap<i32, ArenaAugmentMetadata>, HashMap<i32, String>) {
    let entries = json
        .as_array()
        .or_else(|| json.get("augments").and_then(|v| v.as_array()));
    let Some(arr) = entries else {
        return (HashMap::new(), HashMap::new());
    };

    let mut map = HashMap::with_capacity(arr.len());
    let mut api_names = HashMap::with_capacity(arr.len());
    for entry in arr {
        let Some(id) = entry.get("id").and_then(|v| v.as_i64()).map(|n| n as i32) else {
            continue;
        };
        let name_id = entry
            .get("augmentNameId")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let name = entry
            .get("nameTRA")
            .or_else(|| entry.get("name"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty() && !is_internal_augment_key(s))
            .or_else(|| name_id.filter(|s| !is_internal_augment_key(s)))
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let icon_raw = entry
            .get("augmentSmallIconPath").or_else(|| entry.get("iconSmall"))
            .and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty())
            .or_else(|| entry.get("augmentLargeIconPath").or_else(|| entry.get("iconLarge")).and_then(|v| v.as_str()))
            .unwrap_or("");
        let raw_desc = entry.get("summaryDescription").or_else(|| entry.get("desc")).and_then(|v| v.as_str()).unwrap_or("");

        if let Some(norm_api) = name_id.map(normalize_augment_api_name).filter(|s| !s.is_empty()) {
            api_names.insert(id, norm_api);
        }
        map.insert(id, ArenaAugmentMetadata {
            id,
            name,
            desc: clean_augment_description(raw_desc),
            icon_url: resolve_arena_augment_icon_url(icon_raw),
            rarity: parse_cherry_rarity(entry.get("rarity")),
            ability_slot: name_id.and_then(infer_ability_slot_from_name_id),
        });
    }

    (map, api_names)
}

/// Parses CommunityDragon `cherry-augments.json` into a lookup map keyed by numeric augment ID.
pub fn parse_cdragon_cherry_augments(json: &serde_json::Value) -> HashMap<i32, ArenaAugmentMetadata> {
    parse_cdragon_cherry_augments_with_api_names(json).0
}

/// Supplements empty descriptions (and missing legacy entries) in `augments` using
/// CommunityDragon's `cdragon/arena/{locale}.json`, matching by ID, normalized name
/// (including `Quest:` / `Missão:` prefix removal), and normalized `apiName`.
pub(super) fn supplement_arena_augment_descriptions_with_api_names(
    augments: &mut HashMap<i32, ArenaAugmentMetadata>,
    api_names: &HashMap<i32, String>,
    arena_json: &serde_json::Value,
    locale: AppLocale,
) {
    let Some(arr) = arena_json.get("augments").and_then(|v| v.as_array()) else {
        return;
    };

    let mut by_name: HashMap<String, (String, Option<String>)> = HashMap::with_capacity(arr.len());
    let mut by_api_name: HashMap<String, (String, Option<String>)> = HashMap::with_capacity(arr.len());

    for entry in arr {
        let Some(id) = entry.get("id").and_then(|v| v.as_i64()).map(|n| n as i32) else {
            continue;
        };
        let raw_desc = entry.get("desc").or_else(|| entry.get("tooltip")).and_then(|v| v.as_str()).unwrap_or("");
        let data_values = entry.get("dataValues").and_then(|v| v.as_object());
        let cleaned = interpolate_arena_desc_for_locale(raw_desc, data_values, locale);
        let inferred_slot = infer_ability_slot_from_desc(raw_desc);
        let entry_name = entry
            .get("name").and_then(|v| v.as_str()).map(str::trim)
            .filter(|s| !s.is_empty() && !is_internal_augment_key(s));
        let entry_api = entry
            .get("apiName").and_then(|v| v.as_str()).map(normalize_augment_api_name).filter(|s| !s.is_empty());

        if !cleaned.is_empty() || inferred_slot.is_some() {
            let fallback = (cleaned.clone(), inferred_slot.clone());
            if let Some(name) = entry_name {
                for candidate in [Some(name), strip_quest_name_prefix(name)].into_iter().flatten() {
                    let norm = normalize_augment_lookup_name(candidate);
                    if !norm.is_empty() {
                        by_name.entry(norm).or_insert_with(|| fallback.clone());
                    }
                }
            }
            if let Some(api_key) = entry_api {
                by_api_name.entry(api_key).or_insert(fallback);
            }
        }

        if let Some(existing) = augments.get_mut(&id) {
            if existing.desc.is_empty() && !cleaned.is_empty() {
                existing.desc = cleaned;
            }
            if existing.ability_slot.is_none() && inferred_slot.is_some() {
                existing.ability_slot = inferred_slot;
            }
            if existing.icon_url.is_empty()
                && let Some(icon) = entry.get("iconSmall").and_then(|v| v.as_str())
            {
                existing.icon_url = resolve_arena_augment_icon_url(icon);
            }
        } else if let Some(name) = entry_name {
            let icon = entry.get("iconSmall").or_else(|| entry.get("iconLarge")).and_then(|v| v.as_str()).unwrap_or("");
            augments.insert(id, ArenaAugmentMetadata {
                id, name: name.to_string(), desc: cleaned,
                icon_url: resolve_arena_augment_icon_url(icon),
                rarity: parse_cherry_rarity(entry.get("rarity")),
                ability_slot: inferred_slot,
            });
        }
    }

    if !by_name.is_empty() || !by_api_name.is_empty() {
        for existing in augments.values_mut() {
            let norm = normalize_augment_lookup_name(&existing.name);
            let matched = by_name
                .get(&norm)
                .or_else(|| api_names.get(&existing.id).and_then(|k| by_api_name.get(k)));
            if let Some((desc, slot)) = matched {
                if existing.desc.is_empty() && !desc.is_empty() {
                    existing.desc = desc.clone();
                }
                if existing.ability_slot.is_none() && slot.is_some() {
                    existing.ability_slot = slot.clone();
                }
            }
        }
    }
}

/// Supplements empty descriptions (and missing legacy entries) in `augments` using
/// CommunityDragon's `cdragon/arena/{locale}.json`.
pub fn supplement_arena_augment_descriptions_for_locale(
    augments: &mut HashMap<i32, ArenaAugmentMetadata>,
    arena_json: &serde_json::Value,
    locale: AppLocale,
) {
    supplement_arena_augment_descriptions_with_api_names(
        augments,
        &HashMap::new(),
        arena_json,
        locale,
    );
}

pub fn supplement_arena_augment_descriptions(
    augments: &mut HashMap<i32, ArenaAugmentMetadata>,
    arena_json: &serde_json::Value,
) {
    supplement_arena_augment_descriptions_for_locale(augments, arena_json, AppLocale::EnUs);
}

/// Best-effort fetch of localized Arena augments metadata (`cherry-augments.json` + optional descriptions).
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
            return parse_cdragon_cherry_augments_with_api_names(&json);
        }
        (HashMap::new(), HashMap::new())
    };

    if !is_prod {
        return cherry_fut.await.0;
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

    let ((mut map, api_names), desc_json) = tokio::join!(cherry_fut, desc_fut);
    if let Some(json) = desc_json {
        supplement_arena_augment_descriptions_with_api_names(&mut map, &api_names, &json, locale);
    }
    map
}

#[cfg(test)]
#[path = "augments_tests.rs"]
mod tests;

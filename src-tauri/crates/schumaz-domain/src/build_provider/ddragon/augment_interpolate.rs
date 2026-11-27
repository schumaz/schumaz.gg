use crate::build_provider::analytics::aram::clean_augment_description;
use crate::models::AppLocale;

pub(super) fn infer_ability_slot_from_name_id(raw: &str) -> Option<String> {
    let upper = raw.trim().to_ascii_uppercase();
    for (suffix, slot) in [("_Q", "Q"), ("_W", "W"), ("_E", "E"), ("_R", "R")] {
        if upper.ends_with(suffix) {
            return Some(slot.to_string());
        }
    }
    None
}

pub(super) fn infer_ability_slot_from_desc(raw: &str) -> Option<String> {
    let lower = raw.to_ascii_lowercase();
    for (token, slot) in [
        ("@spell1keybind@", "Q"),
        ("@spell2keybind@", "W"),
        ("@spell3keybind@", "E"),
        ("@spell4keybind@", "R"),
    ] {
        if lower.contains(token) {
            return Some(slot.to_string());
        }
    }
    None
}

pub(super) fn normalize_augment_lookup_name(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Given a slice starting at `'%'`, returns the inclusive byte index of the closing `'%'`
/// when it matches a Riot inline icon token such as `%i:scaleCrit%`, `%i:scaleAP%`,
/// `%i:Augment%`, or `%is:scalecrit%` (`%<ascii_letters>:<token>%`), ignoring normal percentages.
fn find_riot_icon_token_end(slice_from_pct: &str) -> Option<usize> {
    let bytes = slice_from_pct.as_bytes();
    if bytes.first() != Some(&b'%') {
        return None;
    }

    let mut i = 1usize;
    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
        i += 1;
    }
    if i == 1 || bytes.get(i) != Some(&b':') {
        return None;
    }
    i += 1;

    let payload_start = i;
    while i < bytes.len()
        && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'_' | b'-' | b'.'))
    {
        i += 1;
    }
    if i > payload_start && bytes.get(i) == Some(&b'%') {
        Some(i)
    } else {
        None
    }
}

/// Returns `true` if `desc` still contains raw Riot inline icon tokens (`%i:...%`) or
/// unresolved Mustache/template tokens (`{{ ... }}`).
pub(crate) fn has_raw_riot_augment_markup(desc: &str) -> bool {
    if let Some(open) = desc.find("{{")
        && desc[open + 2..].contains("}}")
    {
        return true;
    }

    let bytes = desc.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'%' && find_riot_icon_token_end(&desc[i..]).is_some() {
            return true;
        }
    }
    false
}

fn resolve_riot_brace_token(token: &str, locale: Option<AppLocale>) -> &'static str {
    let key = token.trim();
    if key.eq_ignore_ascii_case("Item_Keyword_OnHit") {
        return match locale {
            Some(AppLocale::PtBr) => "ao contato",
            _ => "On-Hit",
        };
    }
    if key.eq_ignore_ascii_case("Item_Grievous_Wounds") {
        return match locale {
            Some(AppLocale::PtBr) => "Feridas Dolorosas",
            _ => "Grievous Wounds",
        };
    }
    " "
}

/// Resolves known Riot template tokens (`{{ Item_Keyword_OnHit }}`, `{{ Item_Grievous_Wounds }}`),
/// strips unresolved `{{ ... }}` summary references, and removes inline icon tokens (`%i:...%`).
pub(crate) fn strip_riot_inline_markup(raw: &str, locale: Option<AppLocale>) -> String {
    if !has_raw_riot_augment_markup(raw) {
        return raw.to_string();
    }

    // 1. Resolve or strip `{{ ... }}` template tokens.
    let mut after_braces = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(open) = rest.find("{{") {
        let after_open = &rest[open + 2..];
        let Some(close) = after_open.find("}}") else {
            break;
        };
        after_braces.push_str(&rest[..open]);
        after_braces.push_str(resolve_riot_brace_token(&after_open[..close], locale));
        rest = &after_open[close + 2..];
    }
    after_braces.push_str(rest);

    // 2. Strip Riot inline icon tokens (`%i:scaleCrit%`, `%i:scaleAP%`, `%i:Augment%`, etc.).
    let mut out = String::with_capacity(after_braces.len());
    let bytes = after_braces.as_bytes();
    let mut i = 0usize;
    let mut chunk_start = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(end_rel) = find_riot_icon_token_end(&after_braces[i..])
        {
            out.push_str(&after_braces[chunk_start..i]);
            out.push(' ');
            i += end_rel + 1;
            chunk_start = i;
            continue;
        }
        i += 1;
    }
    out.push_str(&after_braces[chunk_start..]);
    out
}

fn format_data_value(val: f64) -> String {
    let rounded = (val * 10.0).round() / 10.0;
    if (rounded.fract()).abs() < 1e-6 {
        format!("{}", rounded as i64)
    } else {
        format!("{:.1}", rounded)
    }
}

fn lookup_data_value(dv: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<f64> {
    let val = dv.get(key).or_else(|| {
        dv.iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v)
    })?;
    val.as_f64().or_else(|| {
        val.as_array()
            .and_then(|a| a.first())
            .and_then(|x| x.as_f64())
    })
}

pub(super) fn interpolate_arena_desc_for_locale(
    raw: &str,
    data_values: Option<&serde_json::Map<String, serde_json::Value>>,
    locale: AppLocale,
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

        match key.to_ascii_lowercase().as_str() {
            "spell1keybind" => {
                out.push('Q');
                continue;
            }
            "spell2keybind" => {
                out.push('W');
                continue;
            }
            "spell3keybind" => {
                out.push('E');
                continue;
            }
            "spell4keybind" => {
                out.push('R');
                continue;
            }
            _ => {}
        }

        if let Some(num) = data_values.and_then(|dv| lookup_data_value(dv, key)) {
            out.push_str(&format_data_value(num * mult));
        } else if key.eq_ignore_ascii_case("rangeincrease")
            && let Some(dv) = data_values
        {
            match (
                lookup_data_value(dv, "MeleeRangeIncrease"),
                lookup_data_value(dv, "RangedRangeIncrease"),
            ) {
                (Some(melee), Some(ranged)) => {
                    out.push_str(&format!(
                        "{}/{}",
                        format_data_value(melee * mult),
                        format_data_value(ranged * mult)
                    ));
                }
                (Some(v), None) | (None, Some(v)) => out.push_str(&format_data_value(v * mult)),
                (None, None) => {}
            }
        } else if let Some(stripped) = rest.strip_prefix('%') {
            rest = stripped;
        }
    }
    out.push_str(rest);
    let cleaned_markup = strip_riot_inline_markup(&out, Some(locale));
    clean_augment_description(&cleaned_markup)
}

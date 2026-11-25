use crate::build_provider::ddragon::strip_riot_inline_markup;
use crate::models::AppLocale;

const ABILITY_SLOT_MARKER: &str = "__ABILITY_SLOT__";

pub fn normalize_opgg_champion_key(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Returns `true` when an augment title looks like an untranslated Riot internal key
/// (e.g. `GoH_PentakillAug_Generic`, `ARAM_BreadAndCheese`) with underscores and no spaces.
pub fn is_internal_augment_key(name: &str) -> bool {
    let trimmed = name.trim();
    trimmed.contains('_') && !trimmed.contains(' ')
}

/// Returns `true` if `desc` contains a standalone unresolved Riot/OP.GG `?` placeholder
/// (such as `"with ?,"`, `"Empower ?!"`, `"Gain ? Attack Range"`, `"every ?s."`),
/// while ignoring legitimate sentence-ending question marks and `"???"`.
pub fn has_unresolved_augment_placeholder(desc: &str) -> bool {
    let bytes = desc.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b != b'?' {
            continue;
        }
        let prev_is_q = i > 0 && bytes[i - 1] == b'?';
        let next_is_q = i + 1 < bytes.len() && bytes[i + 1] == b'?';
        if prev_is_q || next_is_q {
            continue;
        }
        let prev_boundary =
            i == 0 || bytes[i - 1].is_ascii_whitespace() || matches!(bytes[i - 1], b'(' | b'>');
        let next_boundary = i + 1 == bytes.len()
            || bytes[i + 1].is_ascii_whitespace()
            || matches!(
                bytes[i + 1],
                b',' | b'.' | b'!' | b':' | b';' | b')' | b'%' | b'<'
            )
            || (bytes[i + 1] == b's'
                && (i + 2 == bytes.len() || !bytes[i + 2].is_ascii_alphabetic()));
        if prev_boundary && next_boundary {
            return true;
        }
    }
    false
}

/// Replaces OP.GG ability-slot tags (`<font color='#F0C200'>?</font>`) before HTML stripping.
fn replace_ability_slot_font_tags(raw: &str, ability_slot: Option<&str>) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    let replacement = ability_slot
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(ABILITY_SLOT_MARKER);

    while let Some(font_pos) = rest.find("<font") {
        let after_font = &rest[font_pos..];
        let Some(gt_rel) = after_font.find('>') else {
            break;
        };
        let after_gt = &after_font[gt_rel + 1..];
        if let Some(close_rel) = after_gt.find("</font>") {
            let inner = after_gt[..close_rel].trim();
            if inner == "?" {
                out.push_str(&rest[..font_pos]);
                out.push_str(replacement);
                rest = &after_gt[close_rel + "</font>".len()..];
                continue;
            }
        }
        out.push_str(&rest[..font_pos + gt_rel + 1]);
        rest = after_gt;
    }

    out.push_str(rest);
    out
}

fn resolve_ability_slot_markers(text: &str, locale: AppLocale) -> String {
    if !text.contains(ABILITY_SLOT_MARKER) {
        return text.to_string();
    }

    let mut s = match locale {
        AppLocale::EnUs => text
            .replace(&format!("Your {ABILITY_SLOT_MARKER}"), "Your Ability")
            .replace(&format!("your {ABILITY_SLOT_MARKER}"), "your Ability")
            .replace(
                &format!("Empowered {ABILITY_SLOT_MARKER}"),
                "Empowered Ability",
            )
            .replace(
                &format!("empowered {ABILITY_SLOT_MARKER}"),
                "empowered Ability",
            )
            .replace(&format!(". {ABILITY_SLOT_MARKER}"), ". An Ability")
            .replace(&format!("! {ABILITY_SLOT_MARKER}"), "! An Ability"),
        AppLocale::PtBr => text
            .replace(&format!(". {ABILITY_SLOT_MARKER}"), ". Uma habilidade")
            .replace(&format!("! {ABILITY_SLOT_MARKER}"), "! Uma habilidade"),
    };

    if s.starts_with(ABILITY_SLOT_MARKER) {
        let start_phrase = match locale {
            AppLocale::EnUs => "An Ability",
            AppLocale::PtBr => "Uma habilidade",
        };
        s = format!("{start_phrase}{}", &s[ABILITY_SLOT_MARKER.len()..]);
    }

    let fallback_phrase = match locale {
        AppLocale::EnUs => "an Ability",
        AppLocale::PtBr => "uma habilidade",
    };
    s.replace(ABILITY_SLOT_MARKER, fallback_phrase)
}

/// Cleans unresolved numeric or residual `?` placeholders in plain text without inventing numbers.
pub fn sanitize_augment_placeholders(text: &str, locale: AppLocale) -> String {
    if !has_unresolved_augment_placeholder(text) {
        return text.to_string();
    }

    let mut s = match locale {
        AppLocale::EnUs => text
            .replace("every ?s", "periodically")
            .replace("for ?s", "briefly")
            .replace("Gain ? ", "Gain bonus ")
            .replace("gain ? ", "gain bonus ")
            .replace("and ? ", "and bonus ")
            .replace("Deal an extra ? ", "Deal bonus ")
            .replace("an extra ? ", "bonus ")
            .replace("dealing ? ", "dealing ")
            .replace("Score ? ", "Score ")
            .replace(" by ?%", "")
            .replace("1 plus ? ", "additional "),
        AppLocale::PtBr => text
            .replace("a cada ?s", "periodicamente")
            .replace("por ?s", "brevemente")
            .replace("Recebe ? de ", "Recebe bônus de ")
            .replace("recebe ? de ", "recebe bônus de ")
            .replace("e ? de ", "e bônus de ")
            .replace("Causa ? de dano adicional", "Causa dano adicional")
            .replace("causando ? de ", "causando ")
            .replace("consiga ? ", "consiga ")
            .replace(" em ?%", "")
            .replace(" ? de ", " "),
    };

    // Generic cleanup for any remaining mid-sentence standalone `?` placeholder.
    s = s
        .replace(" ?% ", " ")
        .replace(" ?%", "")
        .replace(" ?s.", ".")
        .replace(" ?s,", ",")
        .replace(" ?s ", " ")
        .replace(" ? ", " ")
        .replace(" ?,", ",")
        .replace(" ?.", ".")
        .replace(" ?!", "!")
        .replace(" ?:", ":")
        .replace(". ? ", ". ")
        .replace("! ? ", "! ");

    if let Some(stripped) = s.strip_prefix("? ") {
        s = stripped.to_string();
    }

    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" ,", ",")
        .replace(" .", ".")
        .replace(" !", "!")
        .replace(" :", ":")
}

pub fn clean_augment_description(raw: &str) -> String {
    let normalized_breaks = raw
        .replace("<br />", " ")
        .replace("<br/>", " ")
        .replace("<br>", " ")
        .replace("<BR />", " ")
        .replace("<BR/>", " ")
        .replace("<BR>", " ");

    let mut out = String::with_capacity(normalized_breaks.len());
    let mut in_tag = false;
    for ch in normalized_breaks.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }

    let without_riot_tokens = strip_riot_inline_markup(&out, None);
    without_riot_tokens
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" ,", ",")
        .replace(" .", ".")
        .replace(" !", "!")
        .replace(" :", ":")
        .replace(" ;", ";")
}

/// Cleans HTML augment descriptions while resolving `<font color='#F0C200'>?</font>` ability-slot
/// tokens (using `ability_slot` when known or a localized term), Riot inline markup, and `?` placeholders.
pub fn clean_augment_description_with_context(
    raw: &str,
    locale: AppLocale,
    ability_slot: Option<&str>,
) -> String {
    let pre = replace_ability_slot_font_tags(raw, ability_slot);
    let localized = strip_riot_inline_markup(&pre, Some(locale));
    let stripped = clean_augment_description(&localized);
    let with_slots = resolve_ability_slot_markers(&stripped, locale);
    sanitize_augment_placeholders(&with_slots, locale)
}

pub(crate) fn round_pct(val: f64) -> f64 {
    let pct = if val <= 1.0 { val * 100.0 } else { val };
    (pct * 10.0).round() / 10.0
}

/// Decodes all `self.__next_f.push([1,"..."])` string payloads in Next.js App Router HTML.
pub(crate) fn decode_next_f_stream(html: &str) -> String {
    const PREFIX: &str = "self.__next_f.push([1,";
    let mut combined = String::new();
    let mut cursor = 0;

    while let Some(rel_idx) = html[cursor..].find(PREFIX) {
        let start = cursor + rel_idx + PREFIX.len();
        let bytes = html.as_bytes();
        if start >= bytes.len() || bytes[start] != b'"' {
            cursor = start;
            continue;
        }

        let mut i = start + 1;
        let mut escaped = false;
        while i < bytes.len() {
            let b = bytes[i];
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                break;
            }
            i += 1;
        }

        if i < bytes.len() && bytes[i] == b'"' {
            let literal = &html[start..=i];
            if let Ok(decoded) = serde_json::from_str::<String>(literal) {
                combined.push_str(&decoded);
            }
            cursor = i + 1;
        } else {
            break;
        }
    }

    combined
}

/// Extracts a balanced JSON `[...]` slice starting at `array_start_idx`.
pub(crate) fn extract_balanced_json_array(source: &str, array_start_idx: usize) -> Option<&str> {
    let bytes = source.as_bytes();
    if bytes.get(array_start_idx) != Some(&b'[') {
        return None;
    }

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, &b) in bytes[array_start_idx..].iter().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
        } else {
            match b {
                b'"' => in_string = true,
                b'[' => depth += 1,
                b']' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some(&source[array_start_idx..=array_start_idx + offset]);
                    }
                }
                _ => {}
            }
        }
    }

    None
}

#[cfg(test)]
#[path = "sanitize_tests.rs"]
mod sanitize_tests;

use super::super::models::ItemMetadata;
use std::collections::HashMap;

const MAX_ITEM_STATS_LINES: usize = 8;
const MAX_ITEM_FALLBACK_DESC_CHARS: usize = 220;

fn find_ascii_ci(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if n.len() > h.len() {
        return None;
    }
    h.windows(n.len()).position(|w| w.eq_ignore_ascii_case(n))
}

/// Extracts the inner substring of the first `<tag>...</tag>` block (ASCII case-insensitive).
fn extract_xml_block<'a>(raw: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start_rel = find_ascii_ci(raw, &open)?;
    let after_open = &raw[start_rel + open.len()..];
    let end_rel = find_ascii_ci(after_open, &close)?;
    Some(&after_open[..end_rel])
}

/// Removes all occurrences of `<tag>...</tag>` blocks (ASCII case-insensitive).
fn strip_xml_block(raw: &str, tag: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start_rel) = find_ascii_ci(rest, &open) {
        out.push_str(&rest[..start_rel]);
        let after_open = &rest[start_rel + open.len()..];
        if let Some(end_rel) = find_ascii_ci(after_open, &close) {
            rest = &after_open[end_rel + close.len()..];
        } else {
            rest = "";
            break;
        }
    }
    out.push_str(rest);
    out
}

/// Joins item effect titles (`</passive><br>`, `</active><br>`, etc.) with `": "` when no colon is present.
fn attach_label_colons(raw: &str) -> String {
    let mut current = raw.to_string();
    for tag in ["passive", "active", "unique", "aura"] {
        let close = format!("</{tag}>");
        let mut next = String::with_capacity(current.len() + 8);
        let mut rest = current.as_str();
        while let Some(pos) = find_ascii_ci(rest, &close) {
            let prefix = &rest[..pos];
            next.push_str(prefix);
            let after_close = &rest[pos + close.len()..];
            let trimmed_after = after_close.trim_start_matches(' ');
            let br_len = if trimmed_after.len() >= 4
                && trimmed_after.as_bytes()[..4].eq_ignore_ascii_case(b"<br>")
            {
                Some(4)
            } else if trimmed_after.len() >= 5
                && trimmed_after.as_bytes()[..5].eq_ignore_ascii_case(b"<br/>")
            {
                Some(5)
            } else if trimmed_after.len() >= 6
                && trimmed_after.as_bytes()[..6].eq_ignore_ascii_case(b"<br />")
            {
                Some(6)
            } else {
                None
            };
            if let Some(blen) = br_len {
                let already_colon = next.trim_end().ends_with(':');
                next.push_str(if already_colon { " " } else { ": " });
                rest = &trimmed_after[blen..];
            } else {
                next.push_str(&rest[pos..pos + close.len()]);
                rest = after_close;
            }
        }
        next.push_str(rest);
        current = next;
    }
    current
}

fn decode_html_entities(input: &str) -> String {
    if !input.contains('&') {
        return input.to_string();
    }
    input
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

fn normalize_line_spacing(line: &str) -> String {
    line.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" ,", ",")
        .replace(" .", ".")
        .replace(" !", "!")
        .replace(" ?", "?")
        .replace(" :", ":")
        .replace(" ;", ";")
}

/// Sanitizes DataDragon HTML/XML markup (`<br>`, `<lol-uikit-tooltipped-keyword>`, `<font>`, etc.)
/// into clean multiline plain text preserving line breaks without any raw tags or HTML entities.
pub fn sanitize_ddragon_html(raw: &str) -> String {
    if raw.trim().is_empty() {
        return String::new();
    }

    let mut out = String::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            let slice = &raw[i..];
            if let Some(close_rel) = slice.find('>') {
                let tag_inner = slice[1..close_rel].trim();
                let tag_name = tag_inner
                    .trim_start_matches('/')
                    .split(|c: char| c.is_whitespace() || c == '/')
                    .next()
                    .unwrap_or("");
                if tag_name.eq_ignore_ascii_case("br")
                    || tag_name.eq_ignore_ascii_case("hr")
                    || tag_name.eq_ignore_ascii_case("li")
                {
                    out.push('\n');
                }
                i += close_rel + 1;
                continue;
            }
            // Malformed unclosed tag: stop copying raw markup.
            break;
        }
        if let Some(ch) = raw[i..].chars().next() {
            out.push(ch);
            i += ch.len_utf8();
        } else {
            break;
        }
    }

    let decoded = decode_html_entities(&out);
    decoded
        .lines()
        .map(normalize_line_spacing)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn truncate_on_word_boundary(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let prefix: String = text.chars().take(max_chars).collect();
    let cut = prefix
        .rfind(|c: char| c.is_whitespace())
        .filter(|&idx| idx > max_chars / 2)
        .unwrap_or(prefix.len());
    let trimmed = prefix[..cut]
        .trim_end_matches(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | ':' | '-'));
    format!("{trimmed}…")
}

/// Extracts localized stat lines from DataDragon `item.json` `<stats>...</stats>` markup.
pub fn extract_item_stats(description: &str) -> Vec<String> {
    let Some(stats_block) = extract_xml_block(description, "stats") else {
        return Vec::new();
    };
    sanitize_ddragon_html(stats_block)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(MAX_ITEM_STATS_LINES)
        .map(str::to_string)
        .collect()
}

/// Resolves a concise item summary preferring DataDragon `plaintext`, falling back to a sanitized
/// non-stats summary from `description` when `plaintext` is empty (e.g. modern & Arena items).
pub fn extract_item_plaintext(plaintext: &str, description: &str) -> String {
    let clean_plain = sanitize_ddragon_html(plaintext);
    if !clean_plain.is_empty() {
        return clean_plain;
    }
    if description.trim().is_empty() {
        return String::new();
    }
    let without_stats = strip_xml_block(description, "stats");
    let without_rules = strip_xml_block(&without_stats, "rules");
    let without_flavor = strip_xml_block(&without_rules, "flavorText");
    let with_colons = attach_label_colons(&without_flavor);
    let sanitized = sanitize_ddragon_html(&with_colons);
    truncate_on_word_boundary(&sanitized, MAX_ITEM_FALLBACK_DESC_CHARS)
}

/// Parses DataDragon `{locale}/item.json` into a lightweight tooltip metadata map keyed by item ID.
pub fn parse_ddragon_items(items_json: &serde_json::Value) -> HashMap<i32, ItemMetadata> {
    let Some(data) = items_json.get("data").and_then(|d| d.as_object()) else {
        return HashMap::new();
    };

    let mut map = HashMap::with_capacity(data.len());
    for (id_str, item_val) in data {
        let Ok(id) = id_str.parse::<i32>() else {
            continue;
        };
        let raw_name = item_val.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let name = sanitize_ddragon_html(raw_name);
        if name.is_empty() {
            continue;
        }
        let gold_total = item_val
            .pointer("/gold/total")
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            .max(0) as i32;
        let raw_plaintext = item_val
            .get("plaintext")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let raw_description = item_val
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let stats = extract_item_stats(raw_description);
        let plaintext = extract_item_plaintext(raw_plaintext, raw_description);

        map.insert(
            id,
            ItemMetadata {
                id,
                name,
                gold_total,
                stats,
                plaintext,
            },
        );
    }

    map
}

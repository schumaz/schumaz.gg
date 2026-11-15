pub fn normalize_opgg_champion_key(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
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

    out.split_whitespace().collect::<Vec<_>>().join(" ")
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

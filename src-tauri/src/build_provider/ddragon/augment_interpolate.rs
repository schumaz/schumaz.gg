use crate::build_provider::analytics::aram::clean_augment_description;

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

pub(super) fn interpolate_arena_desc(
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
    clean_augment_description(&out)
}

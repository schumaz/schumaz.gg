use super::html_extract::clean_augment_description_with_context;
use crate::models::AppLocale;

fn strip_html_tags_for_inspection(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    let mut in_tag = false;
    for ch in segment.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.trim().to_string()
}

fn is_placeholder_stat_tracker_line(segment: &str) -> bool {
    let plain = strip_html_tags_for_inspection(segment);
    if plain.is_empty() {
        return false;
    }
    let Some((label, value)) = plain.rsplit_once(':') else {
        return false;
    };
    if label.trim().is_empty() {
        return false;
    }
    let val = value.trim();
    val.contains('?')
        && val
            .chars()
            .all(|c| matches!(c, '?' | '%' | '.' | 's' | '/' | '-' | ' ' | '(' | ')'))
}

/// Drops trailing/inline in-game stat-tracker lines (e.g. `"Damage Dealt to Champions: ?"`,
/// `"Dano causado a Campeões: ?"`, `"Takedowns: ?"`) from raw OP.GG augment tooltips while
/// preserving rules text and `"QUEST:"` / `"REWARD:"` clauses.
pub fn strip_augment_tooltip_trackers(raw: &str) -> String {
    let normalized = raw
        .replace("<br />", "\n")
        .replace("<br/>", "\n")
        .replace("<br>", "\n")
        .replace("<BR />", "\n")
        .replace("<BR/>", "\n")
        .replace("<BR>", "\n");

    let kept: Vec<&str> = normalized
        .split('\n')
        .map(str::trim)
        .filter(|seg| !seg.is_empty() && !is_placeholder_stat_tracker_line(seg))
        .collect();

    kept.join("<br>")
}

/// Strips unresolved in-game stat-tracker lines from a raw OP.GG augment `tooltip` before
/// applying full Riot/ability-slot/placeholder cleanup.
pub fn clean_augment_tooltip_with_context(
    raw: &str,
    locale: AppLocale,
    ability_slot: Option<&str>,
) -> String {
    let without_trackers = strip_augment_tooltip_trackers(raw);
    clean_augment_description_with_context(&without_trackers, locale, ability_slot)
}

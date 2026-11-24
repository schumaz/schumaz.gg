use super::{
    RawSkillBuild, RawSkillMastery, finalize_skill_order, normalize_skill_key, normalize_skill_list,
};
use crate::models::SkillOrderRecommendation;

#[derive(Default)]
struct QuoteState {
    in_quotes: bool,
    quote_char: char,
    escaped: bool,
}

impl QuoteState {
    fn step(&mut self, ch: char) -> bool {
        if self.escaped {
            self.escaped = false;
            return true;
        }
        if self.in_quotes {
            if ch == '\\' {
                self.escaped = true;
            } else if ch == self.quote_char {
                self.in_quotes = false;
            }
            return true;
        }
        if ch == '"' || ch == '\'' {
            self.in_quotes = true;
            self.quote_char = ch;
            return true;
        }
        false
    }
}

fn extract_balanced_paren_content(after_open_paren: &str) -> Option<(&str, usize)> {
    let mut depth = 1usize;
    let mut quotes = QuoteState::default();

    for (idx, ch) in after_open_paren.char_indices() {
        if quotes.step(ch) {
            continue;
        }
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((&after_open_paren[..idx], idx + 1));
                }
            }
            _ => {}
        }
    }
    None
}

fn split_top_level_args(inside: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut start = 0usize;
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut quotes = QuoteState::default();

    for (idx, ch) in inside.char_indices() {
        if quotes.step(ch) {
            continue;
        }
        match ch {
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            ',' if bracket_depth == 0 && paren_depth == 0 => {
                args.push(inside[start..idx].trim());
                start = idx + 1;
            }
            _ => {}
        }
    }

    let tail = inside[start..].trim();
    if !tail.is_empty() || !args.is_empty() {
        args.push(tail);
    }
    args
}

fn parse_class_fields<'a>(text: &'a str, headers: &[&str]) -> Vec<&'a str> {
    for header in headers {
        if let Some(def_idx) = text.find(header) {
            let line_rest = &text[def_idx + header.len()..];
            let end_line = line_rest.find('\n').unwrap_or(line_rest.len());
            return line_rest[..end_line]
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect();
        }
    }
    Vec::new()
}

fn parse_string_array(arg: &str) -> Option<Vec<String>> {
    let trimmed = arg.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return None;
    }
    let inner = trimmed[1..trimmed.len() - 1].trim();
    if inner.is_empty() {
        return None;
    }
    let tokens: Vec<String> = split_top_level_args(inner)
        .into_iter()
        .map(|part| {
            let p = part.trim();
            if (p.starts_with('"') && p.ends_with('"') && p.len() >= 2)
                || (p.starts_with('\'') && p.ends_with('\'') && p.len() >= 2)
            {
                p[1..p.len() - 1].trim().to_string()
            } else {
                p.to_string()
            }
        })
        .collect();
    let refs: Vec<&str> = tokens.iter().map(String::as_str).collect();
    normalize_skill_list(refs)
}

fn is_ident_boundary_before(text: &str, idx: usize) -> bool {
    if idx == 0 {
        return true;
    }
    text[..idx]
        .chars()
        .next_back()
        .is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_')
}

fn find_next_ctor<'a>(text: &str, markers: &'a [&'a str]) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None;
    for marker in markers {
        let mut search_start = 0usize;
        while let Some(rel) = text[search_start..].find(marker) {
            let abs = search_start + rel;
            if is_ident_boundary_before(text, abs) {
                if best.is_none_or(|(b_idx, _)| abs < b_idx) {
                    best = Some((abs, marker.len()));
                }
                break;
            }
            search_start = abs + 1;
        }
    }
    best
}

fn extract_ctor_bodies<'a>(text: &'a str, markers: &[&str]) -> Vec<&'a str> {
    let mut bodies = Vec::new();
    let mut cursor = 0usize;
    while let Some((rel_idx, marker_len)) = find_next_ctor(&text[cursor..], markers) {
        let body_start = cursor + rel_idx + marker_len;
        if let Some((inside, consumed)) = extract_balanced_paren_content(&text[body_start..]) {
            bodies.push(inside);
            cursor = body_start + consumed;
        } else {
            cursor = body_start;
        }
    }
    bodies
}

#[derive(Default)]
struct McpSkillStats {
    play: i64,
    win: i64,
    pick_rate: f64,
    win_rate: Option<f64>,
}

impl McpSkillStats {
    fn apply_field(&mut self, field_name: &str, raw_val: &str) {
        match field_name {
            "play" => {
                if let Ok(v) = raw_val.parse::<i64>() {
                    self.play = v;
                }
            }
            "win" => {
                if let Ok(v) = raw_val.parse::<i64>() {
                    self.win = v;
                }
            }
            "pick_rate" => {
                if let Ok(v) = raw_val.parse::<f64>() {
                    self.pick_rate = v;
                }
            }
            "win_rate" => {
                if let Ok(v) = raw_val.parse::<f64>() {
                    self.win_rate = Some(v);
                }
            }
            _ => {}
        }
    }

    fn apply_positional(&mut self, args: &[&str]) {
        let scalars: Vec<&str> = args
            .iter()
            .copied()
            .filter(|a| !a.starts_with('[') && !a.starts_with('"') && !a.starts_with('\''))
            .collect();
        if let Some(Ok(p)) = scalars.first().map(|s| s.parse::<i64>()) {
            self.play = p;
        }
        if let Some(Ok(w)) = scalars.get(1).map(|s| s.parse::<i64>()) {
            self.win = w;
        }
        if let Some(Ok(pr)) = scalars.get(2).map(|s| s.parse::<f64>()) {
            self.pick_rate = pr;
        }
    }
}

fn parse_mcp_skill_build(inside: &str, class_fields: &[&str]) -> Option<RawSkillBuild> {
    let args = split_top_level_args(inside);
    if args.is_empty() {
        return None;
    }

    let mut order: Option<Vec<String>> = None;
    let mut stats = McpSkillStats::default();

    if !class_fields.is_empty() && class_fields.len() == args.len() {
        for (field_name, raw_val) in class_fields.iter().zip(args.iter()) {
            if *field_name == "order" {
                order = parse_string_array(raw_val);
            } else {
                stats.apply_field(field_name, raw_val);
            }
        }
    } else {
        order = args.iter().find_map(|a| parse_string_array(a));
        stats.apply_positional(&args);
    }

    Some(RawSkillBuild {
        order: order?,
        play: stats.play,
        win: stats.win,
        pick_rate: stats.pick_rate,
        win_rate: stats.win_rate,
    })
}

fn parse_mcp_skill_mastery(
    inside: &str,
    mastery_fields: &[&str],
    build_fields: &[&str],
) -> Option<RawSkillMastery> {
    let args = split_top_level_args(inside);
    if args.is_empty() {
        return None;
    }

    let build_markers = ["Builds(", "Build(", "SkillBuilds("];
    let mut ids: Option<Vec<String>> = None;
    let mut saw_ids_field = false;
    let mut stats = McpSkillStats::default();
    let mut builds: Vec<RawSkillBuild> = Vec::new();

    if !mastery_fields.is_empty() && mastery_fields.len() == args.len() {
        for (field_name, raw_val) in mastery_fields.iter().zip(args.iter()) {
            match *field_name {
                "ids" => {
                    saw_ids_field = true;
                    ids = parse_string_array(raw_val);
                }
                "builds" => {
                    builds = extract_ctor_bodies(raw_val, &build_markers)
                        .into_iter()
                        .filter_map(|b| parse_mcp_skill_build(b, build_fields))
                        .collect();
                }
                _ => stats.apply_field(field_name, raw_val),
            }
        }
    } else {
        for arg in &args {
            if ids.is_none()
                && let Some(parsed) = parse_string_array(arg)
                && parsed.iter().all(|k| normalize_skill_key(k).is_some())
            {
                saw_ids_field = true;
                ids = Some(parsed);
            } else if find_next_ctor(arg, &build_markers).is_some() {
                builds.extend(
                    extract_ctor_bodies(arg, &build_markers)
                        .into_iter()
                        .filter_map(|b| parse_mcp_skill_build(b, build_fields)),
                );
            }
        }
        stats.apply_positional(&args);
    }

    let resolved_ids = match ids {
        Some(list) => list,
        None if saw_ids_field => return None,
        None => Vec::new(),
    };
    if resolved_ids.is_empty() && builds.is_empty() {
        return None;
    }

    Some(RawSkillMastery {
        ids: resolved_ids,
        play: stats.play,
        win: stats.win,
        pick_rate: stats.pick_rate,
        win_rate: stats.win_rate,
        builds,
    })
}

/// Extracts `SkillMasteries(...)` and `Skills(...)` blocks from OP.GG MCP DSL text.
pub fn extract_mcp_skill_order(text: &str) -> Option<SkillOrderRecommendation> {
    let mastery_fields = parse_class_fields(text, &["class SkillMasteries:", "class SkillMastery:"]);
    let build_fields = parse_class_fields(
        text,
        &["class Builds:", "class Build:", "class SkillBuilds:"],
    );
    let skill_fields = parse_class_fields(text, &["class Skills:", "class Skill:"]);

    let masteries: Vec<RawSkillMastery> =
        extract_ctor_bodies(text, &["SkillMasteries(", "SkillMastery("])
            .into_iter()
            .filter_map(|inside| parse_mcp_skill_mastery(inside, &mastery_fields, &build_fields))
            .collect();

    let skills: Vec<RawSkillBuild> = extract_ctor_bodies(text, &["Skills(", "Skill("])
        .into_iter()
        .filter_map(|inside| parse_mcp_skill_build(inside, &skill_fields))
        .collect();

    finalize_skill_order(masteries, skills)
}

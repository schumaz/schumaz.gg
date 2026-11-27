use super::{
    RawSkillBuild, RawSkillMastery, finalize_skill_order, normalize_skill_key, normalize_skill_list,
};
use crate::build_provider::analytics::mcp_dsl::{McpDocument, McpTuple, McpValue};
use crate::models::SkillOrderRecommendation;

#[derive(Default)]
struct McpSkillStats {
    play: i64,
    win: i64,
    pick_rate: f64,
    win_rate: Option<f64>,
}

impl McpSkillStats {
    fn apply_field(&mut self, field_name: &str, val: &McpValue<'_>) {
        match field_name {
            "play" => {
                if let Some(v) = val.as_i64() {
                    self.play = v;
                }
            }
            "win" => {
                if let Some(v) = val.as_i64() {
                    self.win = v;
                }
            }
            "pick_rate" => {
                if let Some(v) = val.as_f64() {
                    self.pick_rate = v;
                }
            }
            "win_rate" => {
                if let Some(v) = val.as_f64() {
                    self.win_rate = Some(v);
                }
            }
            _ => {}
        }
    }

    fn apply_positional(&mut self, args: &[McpValue<'_>]) {
        let scalars: Vec<&McpValue<'_>> = args
            .iter()
            .filter(|a| matches!(a, McpValue::Int(_) | McpValue::Float(_)))
            .collect();
        if let Some(p) = scalars.first().and_then(|v| v.as_i64()) {
            self.play = p;
        }
        if let Some(w) = scalars.get(1).and_then(|v| v.as_i64()) {
            self.win = w;
        }
        if let Some(pr) = scalars.get(2).and_then(|v| v.as_f64()) {
            self.pick_rate = pr;
        }
    }
}

fn parse_skill_str_list(val: &McpValue<'_>) -> Option<Vec<String>> {
    let items = val.as_str_list()?;
    normalize_skill_list(items)
}

fn parse_mcp_skill_build(doc: &McpDocument<'_>, tuple: &McpTuple<'_>) -> Option<RawSkillBuild> {
    if tuple.args.is_empty() {
        return None;
    }

    let mut order: Option<Vec<String>> = None;
    let mut stats = McpSkillStats::default();

    if let Some(named) = doc.named_args(tuple) {
        for (field_name, val) in named {
            if field_name == "order" {
                order = parse_skill_str_list(val);
            } else {
                stats.apply_field(field_name, val);
            }
        }
    } else {
        order = tuple.args.iter().find_map(parse_skill_str_list);
        stats.apply_positional(&tuple.args);
    }

    Some(RawSkillBuild {
        order: order?,
        play: stats.play,
        win: stats.win,
        pick_rate: stats.pick_rate,
        win_rate: stats.win_rate,
    })
}

fn parse_mcp_skill_mastery(doc: &McpDocument<'_>, tuple: &McpTuple<'_>) -> Option<RawSkillMastery> {
    if tuple.args.is_empty() {
        return None;
    }

    let build_markers = ["Builds", "Build", "SkillBuilds"];
    let mut ids: Option<Vec<String>> = None;
    let mut saw_ids_field = false;
    let mut stats = McpSkillStats::default();
    let mut builds: Vec<RawSkillBuild> = Vec::new();

    if let Some(named) = doc.named_args(tuple) {
        for (field_name, val) in named {
            match field_name {
                "ids" => {
                    saw_ids_field = true;
                    ids = parse_skill_str_list(val);
                }
                "builds" => {
                    builds = val
                        .find_tuples(&build_markers)
                        .into_iter()
                        .filter_map(|b| parse_mcp_skill_build(doc, b))
                        .collect();
                }
                _ => stats.apply_field(field_name, val),
            }
        }
    } else {
        for arg in &tuple.args {
            if ids.is_none()
                && let Some(parsed) = parse_skill_str_list(arg)
                && parsed.iter().all(|k| normalize_skill_key(k).is_some())
            {
                saw_ids_field = true;
                ids = Some(parsed);
            } else {
                let nested_builds = arg.find_tuples(&build_markers);
                if !nested_builds.is_empty() {
                    builds.extend(
                        nested_builds
                            .into_iter()
                            .filter_map(|b| parse_mcp_skill_build(doc, b)),
                    );
                }
            }
        }
        stats.apply_positional(&tuple.args);
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

/// Extracts `SkillMasteries(...)` and `Skills(...)` from a pre-parsed `McpDocument`.
pub fn extract_mcp_skill_order_from_doc(doc: &McpDocument<'_>) -> Option<SkillOrderRecommendation> {
    let masteries: Vec<RawSkillMastery> = doc
        .find_tuples(&["SkillMasteries", "SkillMastery"])
        .into_iter()
        .filter_map(|tuple| parse_mcp_skill_mastery(doc, tuple))
        .collect();

    let skills: Vec<RawSkillBuild> = doc
        .find_tuples(&["Skills", "Skill"])
        .into_iter()
        .filter_map(|tuple| parse_mcp_skill_build(doc, tuple))
        .collect();

    finalize_skill_order(masteries, skills)
}

/// Extracts `SkillMasteries(...)` and `Skills(...)` blocks from OP.GG MCP DSL text.
pub fn extract_mcp_skill_order(text: &str) -> Option<SkillOrderRecommendation> {
    let doc = McpDocument::parse(text);
    extract_mcp_skill_order_from_doc(&doc)
}

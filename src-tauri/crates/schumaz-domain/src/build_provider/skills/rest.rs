use super::{RawSkillBuild, RawSkillMastery, finalize_skill_order, normalize_skill_list};
use crate::models::SkillOrderRecommendation;

fn parse_json_skill_array(val: Option<&serde_json::Value>) -> Option<Vec<String>> {
    let arr = val?.as_array()?;
    let mut items = Vec::with_capacity(arr.len());
    for elem in arr {
        let s = elem.as_str()?;
        items.push(s);
    }
    normalize_skill_list(items)
}

fn parse_json_skill_build(val: &serde_json::Value) -> Option<RawSkillBuild> {
    let obj = val.as_object()?;
    let order = parse_json_skill_array(obj.get("order"))?;
    let play = obj.get("play").and_then(|v| v.as_i64()).unwrap_or(0);
    let win = obj.get("win").and_then(|v| v.as_i64()).unwrap_or(0);
    let pick_rate = obj.get("pick_rate").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let win_rate = obj.get("win_rate").and_then(|v| v.as_f64());
    Some(RawSkillBuild {
        order,
        play,
        win,
        pick_rate,
        win_rate,
    })
}

/// Extracts `skill_masteries` / `skills` from an OP.GG REST champion analysis JSON payload.
pub fn extract_rest_skill_order(analysis: &serde_json::Value) -> Option<SkillOrderRecommendation> {
    let mut masteries = Vec::new();
    if let Some(arr) = analysis.get("skill_masteries").and_then(|v| v.as_array()) {
        for entry in arr {
            let Some(obj) = entry.as_object() else {
                continue;
            };
            let ids = match obj.get("ids") {
                Some(ids_val) => match parse_json_skill_array(Some(ids_val)) {
                    Some(parsed) => parsed,
                    None => continue,
                },
                None => Vec::new(),
            };
            let builds = obj
                .get("builds")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(parse_json_skill_build)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if ids.is_empty() && builds.is_empty() {
                continue;
            }
            masteries.push(RawSkillMastery {
                ids,
                play: obj.get("play").and_then(|v| v.as_i64()).unwrap_or(0),
                win: obj.get("win").and_then(|v| v.as_i64()).unwrap_or(0),
                pick_rate: obj
                    .get("pick_rate")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                win_rate: obj.get("win_rate").and_then(|v| v.as_f64()),
                builds,
            });
        }
    }

    let skills = analysis
        .get("skills")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(parse_json_skill_build)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    finalize_skill_order(masteries, skills)
}

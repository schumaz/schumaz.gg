mod mcp;
mod rest;

pub use mcp::{extract_mcp_skill_order, extract_mcp_skill_order_from_doc};
pub use rest::extract_rest_skill_order;

use super::models::{
    ChampionAbilityMetadata, ChampionBuild, DataDragonCache, SkillOrderRecommendation,
};

#[derive(Debug, Clone)]
pub(crate) struct RawSkillBuild {
    pub order: Vec<String>,
    pub play: i64,
    pub win: i64,
    pub pick_rate: f64,
    pub win_rate: Option<f64>,
}

#[derive(Debug, Clone)]
pub(crate) struct RawSkillMastery {
    pub ids: Vec<String>,
    pub play: i64,
    pub win: i64,
    pub pick_rate: f64,
    pub win_rate: Option<f64>,
    pub builds: Vec<RawSkillBuild>,
}

pub(crate) fn normalize_skill_key(raw: &str) -> Option<&'static str> {
    let trimmed = raw.trim();
    if trimmed.eq_ignore_ascii_case("Q") {
        Some("Q")
    } else if trimmed.eq_ignore_ascii_case("W") {
        Some("W")
    } else if trimmed.eq_ignore_ascii_case("E") {
        Some("E")
    } else if trimmed.eq_ignore_ascii_case("R") {
        Some("R")
    } else {
        None
    }
}

pub(crate) fn normalize_skill_list<'a, I: IntoIterator<Item = &'a str>>(
    iter: I,
) -> Option<Vec<String>> {
    let mut out = Vec::new();
    for item in iter {
        let key = normalize_skill_key(item)?;
        out.push(key.to_string());
    }
    (!out.is_empty()).then_some(out)
}

fn dedup_priority(keys: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(keys.len());
    for key in keys {
        if key != "R" && !out.contains(&key) {
            out.push(key);
        }
    }
    out
}

fn skill_slot_index(key: &str) -> Option<usize> {
    match key {
        "Q" => Some(0),
        "W" => Some(1),
        "E" => Some(2),
        "R" => Some(3),
        _ => None,
    }
}

fn normalize_pct(raw: f64) -> f64 {
    if !raw.is_finite() || raw <= 0.0 {
        return 0.0;
    }
    let pct = if raw <= 1.0 { raw * 100.0 } else { raw };
    (pct * 10.0).round() / 10.0
}

fn compute_win_rate(play: i64, win: i64, explicit_wr: Option<f64>) -> f64 {
    if let Some(wr) = explicit_wr {
        let normalized = normalize_pct(wr);
        if normalized > 0.0 {
            return normalized;
        }
    }
    if play > 0 && win > 0 {
        ((win as f64 / play as f64) * 1000.0).round() / 10.0
    } else {
        0.0
    }
}

/// Derives basic-ability maxing priority (e.g. `["Q", "E", "W"]`) from a level sequence.
pub fn derive_priority_from_order(order: &[String]) -> Vec<String> {
    let basic_keys = ["Q", "W", "E"];
    let mut stats: Vec<(&str, Option<usize>, i32, usize)> = Vec::new();

    for &key in &basic_keys {
        let mut count = 0i32;
        let mut first_idx = None;
        let mut rank5_idx = None;
        for (idx, item) in order.iter().enumerate() {
            if item.eq_ignore_ascii_case(key) {
                count += 1;
                if first_idx.is_none() {
                    first_idx = Some(idx);
                }
                if count == 5 && rank5_idx.is_none() {
                    rank5_idx = Some(idx);
                }
            }
        }
        if let Some(first) = first_idx {
            stats.push((key, rank5_idx, count, first));
        }
    }

    stats.sort_by(|a, b| match (a.1, b.1) {
        (Some(idx_a), Some(idx_b)) => idx_a
            .cmp(&idx_b)
            .then_with(|| b.2.cmp(&a.2))
            .then_with(|| a.3.cmp(&b.3)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => b.2.cmp(&a.2).then_with(|| a.3.cmp(&b.3)),
    });

    stats.into_iter().map(|(k, _, _, _)| k.to_string()).collect()
}

/// Expands standard 15–17 level OP.GG sequences up to level 18 using `priority` and `max_rank`.
/// Leaves sequences shorter than 15 levels untouched and never fabricates `R` for 0-rank ultimates.
pub fn expand_skill_order_to_18(
    order: &mut Vec<String>,
    priority: &[String],
    abilities: Option<&[ChampionAbilityMetadata]>,
) {
    if order.len() > 18 {
        order.truncate(18);
        return;
    }
    if !(15..18).contains(&order.len()) {
        return;
    }

    let mut counts = [0i32; 4];
    for skill in order.iter() {
        if let Some(idx) = skill_slot_index(skill) {
            counts[idx] += 1;
        }
    }

    let observed_max_basic = counts[0].max(counts[1]).max(counts[2]);
    let basic_default_cap = observed_max_basic.max(5);
    let max_rank_for = |key: &str| -> i32 {
        if let Some(list) = abilities
            && let Some(meta) = list.iter().find(|a| a.key.eq_ignore_ascii_case(key))
            && meta.max_rank > 0
        {
            return meta.max_rank;
        }
        if key == "R" { 3 } else { basic_default_cap }
    };

    let r_max = max_rank_for("R");
    let fallback_order = ["Q", "W", "E"];
    let mut priority_keys: Vec<&str> = priority
        .iter()
        .filter_map(|k| normalize_skill_key(k))
        .filter(|&k| k != "R")
        .collect();
    for &k in &fallback_order {
        if !priority_keys.contains(&k) {
            priority_keys.push(k);
        }
    }

    while order.len() < 18 {
        if order.len() == 15 && counts[3] > 0 && counts[3] < r_max {
            order.push("R".to_string());
            counts[3] += 1;
            continue;
        }
        let chosen = priority_keys
            .iter()
            .copied()
            .find(|&k| skill_slot_index(k).is_some_and(|idx| counts[idx] < max_rank_for(k)))
            .or_else(|| priority_keys.last().copied())
            .unwrap_or("W");
        order.push(chosen.to_string());
        if let Some(idx) = skill_slot_index(chosen) {
            counts[idx] += 1;
        }
    }
}

fn select_best_build(mut builds: Vec<RawSkillBuild>) -> Option<RawSkillBuild> {
    builds.retain(|b| !b.order.is_empty());
    builds.sort_by(|a, b| {
        b.play
            .cmp(&a.play)
            .then_with(|| b.pick_rate.total_cmp(&a.pick_rate))
            .then_with(|| b.win.cmp(&a.win))
    });
    builds.into_iter().next()
}

pub(crate) fn finalize_skill_order(
    mut masteries: Vec<RawSkillMastery>,
    skills: Vec<RawSkillBuild>,
) -> Option<SkillOrderRecommendation> {
    masteries.sort_by(|a, b| {
        b.play
            .cmp(&a.play)
            .then_with(|| b.pick_rate.total_cmp(&a.pick_rate))
            .then_with(|| b.win.cmp(&a.win))
    });

    let best_mastery = masteries.into_iter().next();
    let best_nested_build = best_mastery
        .as_ref()
        .and_then(|m| select_best_build(m.builds.clone()));
    let best_skill = select_best_build(skills);

    let mut order = best_nested_build
        .as_ref()
        .map(|b| b.order.clone())
        .or_else(|| best_skill.as_ref().map(|s| s.order.clone()))
        .unwrap_or_default();

    let mut priority = best_mastery
        .as_ref()
        .map(|m| dedup_priority(m.ids.clone()))
        .unwrap_or_default();
    if priority.is_empty() && !order.is_empty() {
        priority = derive_priority_from_order(&order);
    }

    if priority.is_empty() && order.is_empty() {
        return None;
    }

    let (play, win, raw_pick_rate, raw_win_rate) =
        match (&best_mastery, &best_skill, &best_nested_build) {
            (Some(m), _, _) if m.play > 0 || m.pick_rate > 0.0 => {
                (m.play, m.win, m.pick_rate, m.win_rate)
            }
            (_, Some(s), _) => (s.play, s.win, s.pick_rate, s.win_rate),
            (_, _, Some(nb)) => (nb.play, nb.win, nb.pick_rate, nb.win_rate),
            (Some(m), None, None) => (m.play, m.win, m.pick_rate, m.win_rate),
            _ => (0, 0, 0.0, None),
        };

    expand_skill_order_to_18(&mut order, &priority, None);

    Some(SkillOrderRecommendation {
        priority,
        order,
        play: play.max(0),
        win: win.max(0),
        pick_rate: normalize_pct(raw_pick_rate),
        win_rate: compute_win_rate(play, win, raw_win_rate),
        abilities: Vec::new(),
    })
}

/// Hydrates `build.skill_order.abilities` from `DataDragonCache` and re-applies `max_rank` expansion.
pub fn enrich_skill_order_with_ddragon(build: &mut ChampionBuild, cache: &DataDragonCache) {
    let Some(ref mut skill_order) = build.skill_order else {
        return;
    };
    if let Some(abilities) = cache.champion_abilities.get(&build.champion_id)
        && !abilities.is_empty()
    {
        skill_order.abilities = abilities.clone();
    }
    if (15..18).contains(&skill_order.order.len()) {
        expand_skill_order_to_18(
            &mut skill_order.order,
            &skill_order.priority,
            Some(&skill_order.abilities),
        );
    }
}

#[cfg(test)]
#[path = "skills_tests.rs"]
mod tests;

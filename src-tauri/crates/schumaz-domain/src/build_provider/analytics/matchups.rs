use super::super::models::{ChampionBuild, DataDragonCache};
use super::aram::normalize_opgg_champion_key;
use super::positions::normalize_opgg_lane_name;
use crate::models::{LaneMatchupEntry, LaneMatchupsSummary};
use std::collections::{HashMap, HashSet};

pub const MAX_MATCHUPS_PER_SIDE: usize = 5;
const MIN_RELIABLE_MATCHUP_GAMES: i64 = 20;

#[derive(Debug, Clone, Copy)]
struct RawMatchup {
    champion_id: i32,
    play: i64,
    win: i64,
}

impl RawMatchup {
    fn raw_wr(self) -> f64 {
        (self.win as f64) / (self.play as f64)
    }

    fn to_entry(self) -> LaneMatchupEntry {
        let win_rate = (self.raw_wr() * 1000.0).round() / 10.0;
        LaneMatchupEntry {
            champion_id: self.champion_id,
            play: self.play,
            win: self.win,
            win_rate,
            ..Default::default()
        }
    }
}

fn select_raw_counters_array<'a>(
    analysis: &'a serde_json::Value,
    position: &str,
) -> Option<(&'a Vec<serde_json::Value>, bool)> {
    if let Some(arr) = analysis
        .get("counters")
        .and_then(|v| v.as_array())
        .filter(|arr| !arr.is_empty())
    {
        return Some((arr, false));
    }

    let positions = analysis
        .pointer("/summary/positions")
        .and_then(|v| v.as_array())?;
    let target_lane = normalize_opgg_lane_name(position.trim());
    let matched_pos = if let Some(lane) = target_lane {
        positions.iter().find(|entry| {
            entry
                .get("name")
                .and_then(|v| v.as_str())
                .and_then(normalize_opgg_lane_name)
                == Some(lane)
        })
    } else if positions.len() == 1 {
        positions.first()
    } else {
        None
    }?;

    let counters = matched_pos
        .get("counters")
        .and_then(|v| v.as_array())
        .filter(|arr| !arr.is_empty())?;
    Some((counters, true))
}

fn cmp_hardest_first(a: &RawMatchup, b: &RawMatchup) -> std::cmp::Ordering {
    a.raw_wr()
        .total_cmp(&b.raw_wr())
        .then_with(|| b.play.cmp(&a.play))
        .then_with(|| a.champion_id.cmp(&b.champion_id))
}

fn cmp_favorable_first(a: &RawMatchup, b: &RawMatchup) -> std::cmp::Ordering {
    b.raw_wr()
        .total_cmp(&a.raw_wr())
        .then_with(|| b.play.cmp(&a.play))
        .then_with(|| a.champion_id.cmp(&b.champion_id))
}

/// Extracts top 5 hard counters and top 5 favorable lane matchups from an OP.GG REST analysis payload.
pub fn extract_rest_lane_matchups(
    analysis: &serde_json::Value,
    self_champion_id: i32,
    position: &str,
) -> Option<LaneMatchupsSummary> {
    extract_rest_lane_matchups_for_roster(analysis, self_champion_id, position, None)
}

/// Extracts top 5 hard counters and top 5 favorable lane matchups, restricting opponents
/// to `allowed_lane_champions` when a lane roster is provided.
pub fn extract_rest_lane_matchups_for_roster(
    analysis: &serde_json::Value,
    self_champion_id: i32,
    position: &str,
    allowed_lane_champions: Option<&HashSet<i32>>,
) -> Option<LaneMatchupsSummary> {
    if position.trim().eq_ignore_ascii_case("none") {
        return None;
    }

    let (raw_arr, is_hard_only_fallback) = select_raw_counters_array(analysis, position)?;

    let mut deduped: HashMap<i32, RawMatchup> = HashMap::new();
    for item in raw_arr {
        let Some(champion_id) = item
            .get("champion_id")
            .and_then(|v| v.as_i64())
            .map(|n| n as i32)
        else {
            continue;
        };
        let Some(play) = item.get("play").and_then(|v| v.as_i64()) else {
            continue;
        };
        let Some(raw_win) = item.get("win").and_then(|v| v.as_i64()) else {
            continue;
        };
        if champion_id <= 0 || champion_id == self_champion_id || play <= 0 || raw_win < 0 {
            continue;
        }
        let win = raw_win.min(play);
        let candidate = RawMatchup {
            champion_id,
            play,
            win,
        };
        deduped
            .entry(champion_id)
            .and_modify(|existing| {
                if candidate.play > existing.play
                    || (candidate.play == existing.play && candidate.win > existing.win)
                {
                    *existing = candidate;
                }
            })
            .or_insert(candidate);
    }

    let mut entries: Vec<RawMatchup> = deduped.into_values().collect();
    if let Some(roster) = allowed_lane_champions
        && !roster.is_empty()
    {
        entries.retain(|e| roster.contains(&e.champion_id));
    }
    let reliable_count = entries
        .iter()
        .filter(|e| e.play >= MIN_RELIABLE_MATCHUP_GAMES)
        .count();
    if reliable_count >= MAX_MATCHUPS_PER_SIDE * 2 {
        entries.retain(|e| e.play >= MIN_RELIABLE_MATCHUP_GAMES);
    }
    if entries.is_empty() {
        return None;
    }

    let (hard, favorable): (Vec<LaneMatchupEntry>, Vec<LaneMatchupEntry>) =
        if is_hard_only_fallback {
            entries.sort_by(cmp_hardest_first);
            let hard = entries
                .iter()
                .take(MAX_MATCHUPS_PER_SIDE)
                .copied()
                .map(RawMatchup::to_entry)
                .collect();
            (hard, Vec::new())
        } else if entries.len() >= MAX_MATCHUPS_PER_SIDE * 2 {
            entries.sort_by(cmp_hardest_first);
            let hard = entries[..MAX_MATCHUPS_PER_SIDE]
                .iter()
                .copied()
                .map(RawMatchup::to_entry)
                .collect();
            let remaining = &mut entries[MAX_MATCHUPS_PER_SIDE..];
            remaining.sort_by(cmp_favorable_first);
            let favorable = remaining
                .iter()
                .take(MAX_MATCHUPS_PER_SIDE)
                .copied()
                .map(RawMatchup::to_entry)
                .collect();
            (hard, favorable)
        } else {
            let (mut hard_raw, mut fav_raw): (Vec<RawMatchup>, Vec<RawMatchup>) =
                entries.into_iter().partition(|e| e.raw_wr() < 0.5);
            hard_raw.sort_by(cmp_hardest_first);
            fav_raw.sort_by(cmp_favorable_first);
            let hard = hard_raw
                .into_iter()
                .take(MAX_MATCHUPS_PER_SIDE)
                .map(RawMatchup::to_entry)
                .collect();
            let favorable = fav_raw
                .into_iter()
                .take(MAX_MATCHUPS_PER_SIDE)
                .map(RawMatchup::to_entry)
                .collect();
            (hard, favorable)
        };

    if hard.is_empty() && favorable.is_empty() {
        None
    } else {
        Some(LaneMatchupsSummary { hard, favorable })
    }
}

/// Hydrates opponent champion names, normalized keys, and DataDragon avatars for lane matchups.
pub fn enrich_lane_matchups_with_ddragon(build: &mut ChampionBuild, cache: &DataDragonCache) {
    let Some(ref mut matchups) = build.lane_matchups else {
        return;
    };
    for entry in matchups
        .hard
        .iter_mut()
        .chain(matchups.favorable.iter_mut())
    {
        if let Some(meta) = cache.champions.get(&entry.champion_id) {
            entry.name = meta.display_name.clone();
            entry.key = normalize_opgg_champion_key(&meta.key_name);
            if !meta.icon_url.is_empty() {
                entry.icon_url = meta.icon_url.clone();
            }
        } else if entry.name.is_empty() {
            entry.name = format!("Champion {}", entry.champion_id);
        }
    }
}

#[cfg(test)]
#[path = "matchups_tests.rs"]
mod tests;

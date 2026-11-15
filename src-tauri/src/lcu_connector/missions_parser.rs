use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn current_epoch_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Normalizes timestamps that LCU may provide in either seconds (10 digits, e.g. TFT) or milliseconds (13 digits, e.g. LoL).
pub fn normalize_epoch_millis(raw: i64) -> i64 {
    if raw <= 0 {
        return 0;
    }
    if raw < 10_000_000_000 {
        raw.saturating_mul(1000)
    } else {
        raw
    }
}

fn is_in_time_window(start_raw: i64, end_raw: i64, now_ms: i64) -> bool {
    let start_ms = normalize_epoch_millis(start_raw);
    let end_ms = normalize_epoch_millis(end_raw);

    if start_ms > 0 && now_ms < start_ms {
        return false;
    }
    if end_ms > 0 && now_ms > end_ms {
        return false;
    }
    true
}

fn is_active_mission_status(status: &str) -> bool {
    matches!(
        status,
        "PENDING" | "UPCOMING" | "SELECT_REWARDS" | "COMPLETED" | "REWARDS_PENDING"
    )
}

fn has_only_dummy_objectives(mission: &serde_json::Value) -> bool {
    let Some(objectives) = mission.get("objectives").and_then(|v| v.as_array()) else {
        return false;
    };
    !objectives.is_empty()
        && objectives
            .iter()
            .all(|obj| obj.get("status").and_then(|v| v.as_str()) == Some("DUMMY"))
}

fn is_mission_unread(mission: &serde_json::Value) -> bool {
    let is_new = mission.get("isNew").and_then(|v| v.as_bool()) == Some(true);
    let not_viewed = mission.get("viewed").and_then(|v| v.as_bool()) == Some(false);
    is_new || not_viewed
}

fn parse_json_timestamp(val: Option<&serde_json::Value>) -> i64 {
    let Some(v) = val else {
        return 0;
    };
    if let Some(n) = v.as_i64() {
        return n;
    }
    if let Some(u) = v.as_u64() {
        return i64::try_from(u).unwrap_or(i64::MAX);
    }
    0
}

fn is_mission_in_time_window(mission: &serde_json::Value, now_ms: i64) -> bool {
    let start_time = parse_json_timestamp(mission.get("startTime"));
    let end_time = parse_json_timestamp(mission.get("endTime"));
    is_in_time_window(start_time, end_time, now_ms)
}

fn is_mission_eligible_from_objectives(mission: &serde_json::Value, now_ms: i64) -> bool {
    if has_only_dummy_objectives(mission) {
        return false;
    }

    let status = mission.get("status").and_then(|v| v.as_str()).unwrap_or("");
    let status_ok = status.is_empty() || is_active_mission_status(status);
    status_ok && is_mission_in_time_window(mission, now_ms) && is_mission_unread(mission)
}

fn has_display_locations(mission: &serde_json::Value) -> bool {
    mission
        .get("display")
        .and_then(|d| d.get("locations"))
        .and_then(|l| l.as_array())
        .is_some_and(|arr| !arr.is_empty())
}

fn is_mission_eligible_from_flat_list(mission: &serde_json::Value, now_ms: i64) -> bool {
    if has_only_dummy_objectives(mission) {
        return false;
    }

    let status = mission.get("status").and_then(|v| v.as_str()).unwrap_or("");
    if !is_active_mission_status(status) || !is_mission_in_time_window(mission, now_ms) {
        return false;
    }

    let is_new = mission.get("isNew").and_then(|v| v.as_bool()) == Some(true);
    if is_new {
        return true;
    }

    let not_viewed = mission.get("viewed").and_then(|v| v.as_bool()) == Some(false);
    let display_type = mission
        .get("displayType")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .unwrap_or("");
    not_viewed && !display_type.is_empty() && has_display_locations(mission)
}

/// Extracts `(id, is_new)` pairs for all active, unread missions from `/lol-objectives/v1/objectives/{gameType}`,
/// purely by structural fields (independent of mission/group/category names).
pub fn extract_unread_missions_from_objectives(
    objectives_json: &serde_json::Value,
    now_ms: i64,
) -> Vec<(String, bool)> {
    let mut items = Vec::new();
    let mut seen = HashSet::new();

    let Some(groups) = objectives_json.as_array() else {
        return items;
    };

    for game_group in groups {
        let Some(categories) = game_group
            .get("objectivesCategories")
            .and_then(|v| v.as_array())
        else {
            continue;
        };

        for category in categories {
            let category_start = category
                .get("startDate")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let category_end = category
                .get("endDate")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            if !is_in_time_window(category_start, category_end, now_ms) {
                continue;
            }

            let Some(objective_groups) = category.get("objectives").and_then(|v| v.as_array())
            else {
                continue;
            };

            for obj_group in objective_groups {
                if obj_group.get("isEnabled").and_then(|v| v.as_bool()) == Some(false) {
                    continue;
                }

                let group_start = obj_group
                    .get("startDate")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let group_end = obj_group
                    .get("endDate")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                if !is_in_time_window(group_start, group_end, now_ms) {
                    continue;
                }

                let Some(missions) = obj_group.get("missions").and_then(|v| v.as_array()) else {
                    continue;
                };

                for mission in missions {
                    if !is_mission_eligible_from_objectives(mission, now_ms) {
                        continue;
                    }
                    if let Some(id) = mission
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        && seen.insert(id.to_string())
                    {
                        let is_new = mission.get("isNew").and_then(|v| v.as_bool()) == Some(true);
                        items.push((id.to_string(), is_new));
                    }
                }
            }
        }
    }

    items
}

/// Extracts unique IDs of all active, unread missions from `/lol-objectives/v1/objectives/{gameType}`.
pub fn extract_unread_mission_ids_from_objectives(
    objectives_json: &serde_json::Value,
    now_ms: i64,
) -> Vec<String> {
    extract_unread_missions_from_objectives(objectives_json, now_ms)
        .into_iter()
        .map(|(id, _)| id)
        .collect()
}

/// Extracts `(id, is_new)` pairs for active, unread visible/notifying missions from flat `GET /lol-missions/v1/missions`,
/// filtering strictly by structural attributes (status, time window, `isNew`, `displayType`, `display.locations`).
pub fn extract_unread_missions_from_flat_list(
    missions_json: &serde_json::Value,
    now_ms: i64,
) -> Vec<(String, bool)> {
    let mut items = Vec::new();
    let mut seen = HashSet::new();

    let Some(missions) = missions_json.as_array() else {
        return items;
    };

    for mission in missions {
        if !is_mission_eligible_from_flat_list(mission, now_ms) {
            continue;
        }
        if let Some(id) = mission
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            && seen.insert(id.to_string())
        {
            let is_new = mission.get("isNew").and_then(|v| v.as_bool()) == Some(true);
            items.push((id.to_string(), is_new));
        }
    }

    items
}

/// Extracts unique IDs of active, unread missions from flat `GET /lol-missions/v1/missions`.
pub fn extract_unread_mission_ids_from_missions(
    missions_json: &serde_json::Value,
    now_ms: i64,
) -> Vec<String> {
    extract_unread_missions_from_flat_list(missions_json, now_ms)
        .into_iter()
        .map(|(id, _)| id)
        .collect()
}

/// Extracts active, unread series IDs from `GET /lol-missions/v1/series`.
pub fn extract_unread_series_ids(series_json: &serde_json::Value, now_ms: i64) -> Vec<String> {
    let mut ids = Vec::new();
    let mut seen = HashSet::new();

    let Some(series_list) = series_json.as_array() else {
        return ids;
    };

    for series in series_list {
        let status = series.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if !matches!(status, "ACTIVE" | "PENDING") {
            continue;
        }

        let viewed = series.get("viewed").and_then(|v| v.as_bool()) == Some(true);
        if viewed {
            continue;
        }

        let display_type = series
            .get("displayType")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .unwrap_or("");
        if display_type.is_empty()
            || display_type.eq_ignore_ascii_case("INTERNAL")
            || display_type.eq_ignore_ascii_case("DUMMY")
        {
            continue;
        }

        let start_date = series
            .get("startDate")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let end_date = series.get("endDate").and_then(|v| v.as_i64()).unwrap_or(0);
        if !is_in_time_window(start_date, end_date, now_ms) {
            continue;
        }

        if let Some(id) = series
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            && seen.insert(id.to_string())
        {
            ids.push(id.to_string());
        }
    }

    ids
}

#[cfg(test)]
#[path = "missions_parser_tests.rs"]
mod tests;

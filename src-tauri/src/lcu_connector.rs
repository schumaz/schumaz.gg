//! Module 1: Connection and Authentication with the LCU (LeagueClientUx.exe)
//! Handles locating the client, authenticating via Basic Auth, and performing REST calls.

pub use crate::models::{LcuStatus, SummonerInfo};
use irelia::rest::LcuClient;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri_specta::Event;

pub const CURRENT_SUMMONER_ENDPOINT: &str = "/lol-summoner/v1/current-summoner";
pub const CURRENT_SUMMONER_PROFILE_ENDPOINT: &str =
    "/lol-summoner/v1/current-summoner/summoner-profile";
pub const CHAMPION_MASTERIES_ENDPOINT: &str =
    "/lol-champion-mastery/v1/local-player/champion-masteries";
pub const RANKED_STATS_ENDPOINT: &str = "/lol-ranked/v1/current-ranked-stats";
pub const MATCH_HISTORY_ENDPOINT: &str =
    "/lol-match-history/v1/products/lol/current-summoner/matches?begIndex=0&endIndex=29";
pub const MATCH_HISTORY_FALLBACK_ENDPOINT: &str =
    "/lol-match-history/v1/products/lol/current-summoner/matches";
pub const GAMEFLOW_SESSION_ENDPOINT: &str = "/lol-gameflow/v1/session";
pub const GAMEFLOW_PHASE_ENDPOINT: &str = "/lol-gameflow/v1/gameflow-phase";
pub const CHAMP_SELECT_SESSION_ENDPOINT: &str = "/lol-champ-select/v1/session";
pub const OBJECTIVES_READY_ENDPOINT: &str = "/lol-objectives/v1/ready";
pub const LOL_OBJECTIVES_ENDPOINT: &str = "/lol-objectives/v1/objectives/lol";
pub const TFT_OBJECTIVES_ENDPOINT: &str = "/lol-objectives/v1/objectives/tft";
pub const JADE_OBJECTIVES_ENDPOINT: &str = "/lol-objectives/v1/objectives/jade";
pub const OBJECTIVES_ENDPOINTS: [&str; 3] = [
    LOL_OBJECTIVES_ENDPOINT,
    TFT_OBJECTIVES_ENDPOINT,
    JADE_OBJECTIVES_ENDPOINT,
];
pub const LOL_MISSIONS_ENDPOINT: &str = "/lol-missions/v1/missions";
pub const LOL_MISSIONS_SERIES_ENDPOINT: &str = "/lol-missions/v1/series";
pub const LOL_MISSIONS_PLAYER_ENDPOINT: &str = "/lol-missions/v1/player";

const MAX_AUTO_MARK_ATTEMPTS_PER_ID: u8 = 2;
const AUTO_MARK_PUT_COOLDOWN: Duration = Duration::from_millis(800);

static AUTO_MARK_SWEEP_ACTIVE: AtomicBool = AtomicBool::new(false);
static AUTO_MARK_WS_DEBOUNCE_SEQ: AtomicU64 = AtomicU64::new(0);
static AUTO_MARK_EXEC_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
static AUTO_MARK_TRACKER: OnceLock<Mutex<AutoMarkTracker>> = OnceLock::new();

#[derive(Default)]
struct AutoMarkTracker {
    /// Maps mission or series ID -> (attempt_count, last_is_new_flag) for the current LCU session.
    attempts: HashMap<String, (u8, bool)>,
    last_put_at: Option<Instant>,
}

fn auto_mark_exec_lock() -> &'static tokio::sync::Mutex<()> {
    AUTO_MARK_EXEC_LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

fn auto_mark_tracker() -> &'static Mutex<AutoMarkTracker> {
    AUTO_MARK_TRACKER.get_or_init(|| Mutex::new(AutoMarkTracker::default()))
}

/// Resets per-session mission auto-mark retry tracking (called on LCU disconnect or manual re-enable).
pub fn reset_auto_mark_session_state() {
    if let Ok(mut guard) = auto_mark_tracker().lock() {
        guard.attempts.clear();
        guard.last_put_at = None;
    }
}

fn current_epoch_millis() -> i64 {
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

fn filter_candidates_with_tracker(
    tracker: &mut AutoMarkTracker,
    mission_candidates: &[(String, bool)],
    series_candidates: &[String],
    now: Instant,
) -> (Vec<(String, bool)>, Vec<String>) {
    if let Some(last_put) = tracker.last_put_at
        && now.saturating_duration_since(last_put) < AUTO_MARK_PUT_COOLDOWN
    {
        return (Vec::new(), Vec::new());
    }

    let active_keys: HashSet<&str> = mission_candidates
        .iter()
        .map(|(id, _)| id.as_str())
        .chain(series_candidates.iter().map(String::as_str))
        .collect();
    // Prune IDs that are now cleared in LCU so future reappearances are handled cleanly.
    tracker
        .attempts
        .retain(|key, _| active_keys.contains(key.as_str()));

    let mut missions_to_send = Vec::new();
    for (id, is_new) in mission_candidates {
        match tracker.attempts.get_mut(id) {
            Some((count, prev_is_new)) => {
                if *is_new && !*prev_is_new {
                    *count = 0;
                    *prev_is_new = true;
                    missions_to_send.push((id.clone(), *is_new));
                } else if *count < MAX_AUTO_MARK_ATTEMPTS_PER_ID {
                    missions_to_send.push((id.clone(), *is_new));
                }
            }
            None => {
                missions_to_send.push((id.clone(), *is_new));
            }
        }
    }

    let mut series_to_send = Vec::new();
    for id in series_candidates {
        match tracker.attempts.get(id) {
            Some((count, _)) if *count >= MAX_AUTO_MARK_ATTEMPTS_PER_ID => {}
            _ => series_to_send.push(id.clone()),
        }
    }

    (missions_to_send, series_to_send)
}

fn record_sent_with_tracker(
    tracker: &mut AutoMarkTracker,
    missions_sent: &[(String, bool)],
    series_sent: &[String],
    now: Instant,
) {
    for (id, is_new) in missions_sent {
        let entry = tracker.attempts.entry(id.clone()).or_insert((0, *is_new));
        entry.0 = entry.0.saturating_add(1);
        entry.1 = *is_new;
    }
    for id in series_sent {
        let entry = tracker.attempts.entry(id.clone()).or_insert((0, false));
        entry.0 = entry.0.saturating_add(1);
    }
    tracker.last_put_at = Some(now);
}

/// Calculates the reconnection backoff duration in seconds.
/// Starts at 2s and caps at 10s to conserve CPU while remaining responsive.
pub fn calculate_backoff_secs(attempt: u32) -> u64 {
    let base = 2u64;
    let max_secs = 10u64;
    let delay = base.saturating_mul(1u64 << attempt.min(3));
    delay.min(max_secs)
}

/// Determines whether an event should be emitted to the frontend based on status transitions.
pub fn should_emit_status_change(prev: &LcuStatus, current: &LcuStatus) -> bool {
    prev != current
}

/// Pure parser function to extract summoner information from a raw JSON string.
/// Falls back to `displayName` if `gameName` is empty.
pub fn parse_summoner_info(raw_json: &str) -> Result<SummonerInfo, String> {
    let mut info: SummonerInfo = serde_json::from_str(raw_json)
        .map_err(|err| format!("Failed to parse summoner JSON: {}", err))?;

    // Apply fallback if game_name is empty (legacy accounts or edge-case response)
    if info.game_name.is_empty() && !info.display_name.is_empty() {
        info.game_name = info.display_name.clone();
    }

    Ok(info)
}

/// Pure parser function to extract summoner information from a `serde_json::Value`.
/// Useful for deserializing irelia's JSON responses directly.
pub fn parse_summoner_value(value: &serde_json::Value) -> Result<SummonerInfo, String> {
    let mut info: SummonerInfo = serde_json::from_value(value.clone())
        .map_err(|err| format!("Failed to convert serde value to SummonerInfo: {}", err))?;

    if info.game_name.is_empty() && !info.display_name.is_empty() {
        info.game_name = info.display_name.clone();
    }

    Ok(info)
}

pub type DefaultRequestClient = hyper_util::client::legacy::Client<
    hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
    http_body_util::Full<bytes::Bytes>,
>;

pub type ConnectedLcuClient = LcuClient<DefaultRequestClient>;

/// High-level struct managing the REST connection to the League Client.
pub struct LcuConnection {
    client: ConnectedLcuClient,
}

impl LcuConnection {
    /// Attempts to locate and connect to the active League Client process (LeagueClientUx.exe).
    pub fn connect() -> Result<Self, String> {
        // Ensure rustls has a default crypto provider installed to avoid runtime ambiguity
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

        LcuClient::connect()
            .map(|client| Self { client })
            .map_err(|err| format!("Failed to connect to LCU process: {:?}", err))
    }

    /// Fetches the current summoner info via GET /lol-summoner/v1/current-summoner.
    pub async fn get_current_summoner(&self) -> Result<SummonerInfo, String> {
        let (info, _) = self.get_current_summoner_with_puuid().await?;
        Ok(info)
    }

    /// Fetches current summoner info along with PUUID when provided by LCU.
    pub async fn get_current_summoner_with_puuid(
        &self,
    ) -> Result<(SummonerInfo, Option<String>), String> {
        let response: Option<serde_json::Value> = self
            .client
            .get(CURRENT_SUMMONER_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting current summoner: {:?}", err))?;

        match response {
            Some(val) => {
                let info = parse_summoner_value(&val)?;
                let puuid = val
                    .get("puuid")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
                Ok((info, puuid))
            }
            None => Err("LCU returned an empty response for current summoner".to_string()),
        }
    }

    /// Fetches the current summoner profile (including backgroundSkinId) via GET /lol-summoner/v1/current-summoner/summoner-profile.
    pub async fn get_summoner_profile(&self) -> Result<serde_json::Value, String> {
        let response: Option<serde_json::Value> = self
            .client
            .get(CURRENT_SUMMONER_PROFILE_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting summoner profile: {:?}", err))?;

        response.ok_or_else(|| "LCU returned an empty response for summoner profile".to_string())
    }

    /// Fetches local player champion masteries across supported LCU inventory/mastery endpoints.
    pub async fn get_champion_masteries(
        &self,
        summoner_id: u64,
        puuid: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let mut endpoints = vec![
            format!(
                "/lol-collections/v1/inventories/{}/champion-mastery",
                summoner_id
            ),
            format!(
                "/lol-collections/v1/inventories/{}/champion-mastery/top?limit=3",
                summoner_id
            ),
            "/lol-champion-mastery/v1/local-player/champion-mastery".to_string(),
            CHAMPION_MASTERIES_ENDPOINT.to_string(),
        ];

        if let Some(p) = puuid
            && !p.is_empty()
        {
            endpoints.push(format!(
                "/lol-collections/v1/inventories/{}/champion-mastery",
                p
            ));
            endpoints.push(format!("/lol-champion-mastery/v1/{}/champion-masteries", p));
            endpoints.push(format!("/lol-champion-mastery/v1/{}/champion-mastery", p));
        }

        let mut fallback_valid: Option<serde_json::Value> = None;

        for ep in endpoints {
            if let Ok(Some(val)) = self.client.get::<Option<serde_json::Value>>(&ep).await {
                let is_lcu_err = val.get("errorCode").is_some()
                    || val
                        .get("httpStatus")
                        .and_then(|s| s.as_i64())
                        .is_some_and(|s| s >= 400);
                if is_lcu_err {
                    continue;
                }

                let has_entries = val.as_array().is_some_and(|a| !a.is_empty())
                    || val
                        .get("masteries")
                        .and_then(|v| v.as_array())
                        .is_some_and(|a| !a.is_empty())
                    || val
                        .get("championMasteries")
                        .and_then(|v| v.as_array())
                        .is_some_and(|a| !a.is_empty())
                    || val
                        .get("data")
                        .and_then(|v| v.as_array())
                        .is_some_and(|a| !a.is_empty());

                if has_entries {
                    return Ok(val);
                }

                if fallback_valid.is_none() {
                    fallback_valid = Some(val);
                }
            }
        }

        fallback_valid.ok_or_else(|| "No champion mastery endpoint returned valid data".to_string())
    }

    /// Fetches ranked queue stats via GET /lol-ranked/v1/current-ranked-stats.
    pub async fn get_ranked_stats(&self) -> Result<serde_json::Value, String> {
        let response: Option<serde_json::Value> = self
            .client
            .get(RANKED_STATS_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting ranked stats: {:?}", err))?;

        response.ok_or_else(|| "LCU returned an empty response for ranked stats".to_string())
    }

    /// Fetches recent match history via GET /lol-match-history/v1/products/lol/current-summoner/matches.
    pub async fn get_match_history(&self) -> Result<serde_json::Value, String> {
        if let Ok(Some(val)) = self
            .client
            .get::<Option<serde_json::Value>>(MATCH_HISTORY_ENDPOINT)
            .await
        {
            return Ok(val);
        }

        let response: Option<serde_json::Value> = self
            .client
            .get(MATCH_HISTORY_FALLBACK_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting match history: {:?}", err))?;

        response.ok_or_else(|| "LCU returned an empty response for match history".to_string())
    }

    /// Fetches the current queue ID via GET /lol-gameflow/v1/session.
    pub async fn get_current_queue_id(&self) -> Result<i32, String> {
        let response: Option<serde_json::Value> = self
            .client
            .get(GAMEFLOW_SESSION_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting gameflow session: {:?}", err))?;

        match response {
            Some(val) => crate::ws_listener::extract_queue_id(&val),
            None => Err("LCU returned an empty response for gameflow session".to_string()),
        }
    }

    /// Fetches the current gameflow phase string via GET /lol-gameflow/v1/gameflow-phase.
    pub async fn get_gameflow_phase(&self) -> Result<String, String> {
        let response: Option<serde_json::Value> = self
            .client
            .get(GAMEFLOW_PHASE_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting gameflow phase: {:?}", err))?;

        match response.and_then(|v| v.as_str().map(|s| s.trim().to_string())) {
            Some(phase) if !phase.is_empty() => Ok(phase),
            _ => Err("LCU returned an empty or invalid response for gameflow phase".to_string()),
        }
    }

    /// Fetches the active champion select session via GET /lol-champ-select/v1/session.
    pub async fn get_champ_select_session(&self) -> Result<Option<serde_json::Value>, String> {
        self.client
            .get(CHAMP_SELECT_SESSION_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting champ select session: {:?}", err))
    }

    /// Checks if the objectives plugin has finished loading via GET /lol-objectives/v1/ready.
    pub async fn get_objectives_ready(&self) -> Result<bool, String> {
        let response: Option<serde_json::Value> = self
            .client
            .get(OBJECTIVES_READY_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting objectives readiness: {:?}", err))?;

        Ok(response.and_then(|v| v.as_bool()).unwrap_or(false))
    }

    /// Fetches objectives groups for a given game tab (`lol`, `tft`, `jade`) via GET /lol-objectives/v1/objectives/{gameType}.
    pub async fn get_objectives_for_endpoint(
        &self,
        endpoint: &str,
    ) -> Result<serde_json::Value, String> {
        let response: Option<serde_json::Value> = self
            .client
            .get(endpoint)
            .await
            .map_err(|err| format!("Error requesting objectives ({}): {:?}", endpoint, err))?;

        response.ok_or_else(|| format!("LCU returned an empty response for {}", endpoint))
    }

    /// Fetches LoL objectives groups via GET /lol-objectives/v1/objectives/lol.
    pub async fn get_lol_objectives(&self) -> Result<serde_json::Value, String> {
        self.get_objectives_for_endpoint(LOL_OBJECTIVES_ENDPOINT)
            .await
    }

    /// Fetches player missions via GET /lol-missions/v1/missions.
    pub async fn get_missions(&self) -> Result<serde_json::Value, String> {
        let response: Option<serde_json::Value> = self
            .client
            .get(LOL_MISSIONS_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting missions: {:?}", err))?;

        response.ok_or_else(|| "LCU returned an empty response for missions".to_string())
    }

    /// Fetches player mission series via GET /lol-missions/v1/series.
    pub async fn get_mission_series(&self) -> Result<serde_json::Value, String> {
        let response: Option<serde_json::Value> = self
            .client
            .get(LOL_MISSIONS_SERIES_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting mission series: {:?}", err))?;

        response.ok_or_else(|| "LCU returned an empty response for mission series".to_string())
    }

    /// Marks the given mission and series IDs as viewed/read via PUT /lol-missions/v1/player.
    pub async fn mark_player_items_viewed(
        &self,
        mission_ids: &[String],
        series_ids: &[String],
    ) -> Result<(), String> {
        if mission_ids.is_empty() && series_ids.is_empty() {
            return Ok(());
        }

        let payload = serde_json::json!({
            "missionIds": mission_ids,
            "seriesIds": series_ids
        });

        match self
            .client
            .put::<&serde_json::Value, Option<serde_json::Value>>(
                LOL_MISSIONS_PLAYER_ENDPOINT,
                &payload,
            )
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => {
                let err_str = format!("{:?}", err);
                if err_str.contains("unexpected end of file")
                    || err_str.contains("Unexpected end of file")
                    || err_str.contains("Eof")
                    || err_str.contains("EOF")
                {
                    Ok(())
                } else {
                    Err(format!("Error marking missions viewed: {}", err_str))
                }
            }
        }
    }

    /// Marks the given mission IDs as viewed/read via PUT /lol-missions/v1/player.
    pub async fn mark_missions_viewed(&self, mission_ids: &[String]) -> Result<(), String> {
        self.mark_player_items_viewed(mission_ids, &[]).await
    }

    /// Collects all unread, active missions across LoL/TFT/Jade objectives, flat `/lol-missions/v1/missions`,
    /// and `/lol-missions/v1/series`, and marks them as read/viewed to clear the client notification counter completely.
    pub async fn mark_unread_missions_as_read(&self) -> Result<usize, String> {
        let now_ms = current_epoch_millis();

        let (lol_res, tft_res, jade_res, missions_res, series_res) = tokio::join!(
            self.get_objectives_for_endpoint(LOL_OBJECTIVES_ENDPOINT),
            self.get_objectives_for_endpoint(TFT_OBJECTIVES_ENDPOINT),
            self.get_objectives_for_endpoint(JADE_OBJECTIVES_ENDPOINT),
            self.get_missions(),
            self.get_mission_series(),
        );

        let mut combined_missions: Vec<(String, bool)> = Vec::new();
        let mut seen_mission_ids: HashMap<String, usize> = HashMap::new();

        let mut push_mission = |id: String, is_new: bool| {
            if let Some(&idx) = seen_mission_ids.get(&id) {
                if is_new {
                    combined_missions[idx].1 = true;
                }
            } else {
                let idx = combined_missions.len();
                seen_mission_ids.insert(id.clone(), idx);
                combined_missions.push((id, is_new));
            }
        };

        for obj_res in [lol_res, tft_res, jade_res] {
            if let Ok(obj_json) = obj_res {
                for (id, is_new) in extract_unread_missions_from_objectives(&obj_json, now_ms) {
                    push_mission(id, is_new);
                }
            }
        }

        if let Ok(missions_json) = missions_res {
            for (id, is_new) in extract_unread_missions_from_flat_list(&missions_json, now_ms) {
                push_mission(id, is_new);
            }
        }

        let series_candidates = series_res
            .map(|json| extract_unread_series_ids(&json, now_ms))
            .unwrap_or_default();

        if combined_missions.is_empty() && series_candidates.is_empty() {
            return Ok(0);
        }

        let now_instant = Instant::now();
        let (missions_to_send, series_to_send) = {
            let Ok(mut guard) = auto_mark_tracker().lock() else {
                return Ok(0);
            };
            filter_candidates_with_tracker(
                &mut guard,
                &combined_missions,
                &series_candidates,
                now_instant,
            )
        };

        if missions_to_send.is_empty() && series_to_send.is_empty() {
            return Ok(0);
        }

        let mission_ids: Vec<String> = missions_to_send.iter().map(|(id, _)| id.clone()).collect();
        self.mark_player_items_viewed(&mission_ids, &series_to_send)
            .await?;

        if let Ok(mut guard) = auto_mark_tracker().lock() {
            record_sent_with_tracker(
                &mut guard,
                &missions_to_send,
                &series_to_send,
                Instant::now(),
            );
        }

        Ok(mission_ids.len() + series_to_send.len())
    }

    /// Fetches the current rune page via GET /lol-perks/v1/currentpage.
    pub async fn get_current_rune_page(&self) -> Result<serde_json::Value, String> {
        let response: Option<serde_json::Value> = self
            .client
            .get(crate::lcu_injector::PERKS_CURRENT_PAGE_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting current rune page: {:?}", err))?;

        response.ok_or_else(|| "LCU returned an empty response for current rune page".to_string())
    }

    /// Fetches all rune pages via GET /lol-perks/v1/pages.
    pub async fn get_all_rune_pages(&self) -> Result<serde_json::Value, String> {
        let response: Option<serde_json::Value> = self
            .client
            .get(crate::lcu_injector::PERKS_PAGES_ENDPOINT)
            .await
            .map_err(|err| format!("Error requesting rune pages: {:?}", err))?;

        response.ok_or_else(|| "LCU returned an empty response for rune pages".to_string())
    }

    /// Updates an existing rune page in-place via PUT /lol-perks/v1/pages/{id}.
    pub async fn put_rune_page(
        &self,
        page_id: i64,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let endpoint = format!("{}/{}", crate::lcu_injector::PERKS_PAGES_ENDPOINT, page_id);
        let response: Option<serde_json::Value> = self
            .client
            .put(&endpoint, payload)
            .await
            .map_err(|err| format!("Error updating rune page {}: {:?}", page_id, err))?;

        response.ok_or_else(|| "LCU returned an empty response when updating rune page".to_string())
    }

    /// Deletes a rune page by ID via DELETE /lol-perks/v1/pages/{id}.
    pub async fn delete_rune_page(&self, page_id: i64) -> Result<(), String> {
        let endpoint = format!("{}/{}", crate::lcu_injector::PERKS_PAGES_ENDPOINT, page_id);
        match self
            .client
            .delete::<Option<serde_json::Value>>(&endpoint)
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => {
                let err_str = format!("{:?}", err);
                if err_str.contains("unexpected end of file")
                    || err_str.contains("Unexpected end of file")
                    || err_str.contains("Eof")
                    || err_str.contains("EOF")
                {
                    Ok(())
                } else {
                    Err(format!("Error deleting rune page {}: {}", page_id, err_str))
                }
            }
        }
    }

    /// Creates a new rune page via POST /lol-perks/v1/pages.
    pub async fn create_rune_page(
        &self,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let response: Option<serde_json::Value> = self
            .client
            .post(crate::lcu_injector::PERKS_PAGES_ENDPOINT, payload)
            .await
            .map_err(|err| format!("Error creating rune page: {:?}", err))?;

        response.ok_or_else(|| "LCU returned an empty response when creating rune page".to_string())
    }

    /// Updates summoner spells via PATCH /lol-champ-select/v1/session/my-selection.
    /// Retries up to 5 times with 200ms backoff if champ select session is transitioning.
    pub async fn patch_summoner_spells(&self, payload: &serde_json::Value) -> Result<(), String> {
        let mut last_err = String::new();
        for attempt in 1..=5 {
            match self
                .client
                .patch::<&serde_json::Value, Option<serde_json::Value>>(
                    crate::lcu_injector::CHAMP_SELECT_MY_SELECTION_ENDPOINT,
                    payload,
                )
                .await
            {
                Ok(_) => return Ok(()),
                Err(err) => {
                    let err_str = format!("{:?}", err);
                    if err_str.contains("unexpected end of file")
                        || err_str.contains("Unexpected end of file")
                        || err_str.contains("Eof")
                        || err_str.contains("EOF")
                    {
                        return Ok(());
                    }
                    last_err = format!("Error patching summoner spells: {}", err_str);
                    if attempt < 5 && (err_str.contains("404") || err_str.contains("NotFound")) {
                        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                        continue;
                    }
                    break;
                }
            }
        }
        Err(last_err)
    }

    /// Updates custom item sets via PUT /lol-item-sets/v1/item-sets/{summoner_id}/sets.
    pub async fn put_item_sets(
        &self,
        summoner_id: u64,
        payload: &serde_json::Value,
    ) -> Result<(), String> {
        let endpoint = format!("/lol-item-sets/v1/item-sets/{}/sets", summoner_id);
        match self
            .client
            .put::<&serde_json::Value, Option<serde_json::Value>>(&endpoint, payload)
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => {
                let err_str = format!("{:?}", err);
                if err_str.contains("unexpected end of file")
                    || err_str.contains("Unexpected end of file")
                    || err_str.contains("Eof")
                    || err_str.contains("EOF")
                {
                    Ok(())
                } else {
                    Err(format!("Error putting item sets: {}", err_str))
                }
            }
        }
    }
}

async fn run_auto_mark_once() {
    if !crate::lcu_injector::get_user_preferences().auto_mark_daily_missions_read {
        return;
    }

    let _exec_guard = auto_mark_exec_lock().lock().await;
    if !crate::lcu_injector::get_user_preferences().auto_mark_daily_missions_read {
        return;
    }

    let Ok(conn) = LcuConnection::connect() else {
        return;
    };

    let _ = conn.mark_unread_missions_as_read().await;
}

/// Debounced trigger called by the LCU WebSocket listener whenever missions or objectives update.
pub fn schedule_debounced_auto_mark_missions() {
    if !crate::lcu_injector::get_user_preferences().auto_mark_daily_missions_read {
        return;
    }

    let seq = AUTO_MARK_WS_DEBOUNCE_SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(1200)).await;
        if AUTO_MARK_WS_DEBOUNCE_SEQ.load(Ordering::SeqCst) != seq {
            return;
        }
        run_auto_mark_once().await;
    });
}

/// Spawns a non-blocking post-connection sweep (~65s window) to catch delayed mission hydration
/// after opening the League Client, and executes immediately if triggered by Settings toggle.
pub fn spawn_auto_mark_daily_missions_if_enabled() {
    if !crate::lcu_injector::get_user_preferences().auto_mark_daily_missions_read {
        return;
    }

    if AUTO_MARK_SWEEP_ACTIVE
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        // A startup sweep is already active; run an immediate pass in case the user just toggled it ON.
        tokio::spawn(async move {
            run_auto_mark_once().await;
        });
        return;
    }

    tokio::spawn(async move {
        struct ResetGuard;
        impl Drop for ResetGuard {
            fn drop(&mut self) {
                AUTO_MARK_SWEEP_ACTIVE.store(false, Ordering::SeqCst);
            }
        }
        let _guard = ResetGuard;

        // Post-login/startup mission hydration in the League Client can take ~30+ seconds.
        // Sweep immediately and periodically across the first ~65s without spamming PUTs.
        let step_delays_secs: [u64; 13] = [0, 3, 3, 4, 5, 5, 5, 5, 5, 5, 5, 10, 10];
        for delay_secs in step_delays_secs {
            if delay_secs > 0 {
                tokio::time::sleep(Duration::from_secs(delay_secs)).await;
            }
            if !crate::lcu_injector::get_user_preferences().auto_mark_daily_missions_read {
                return;
            }
            run_auto_mark_once().await;
        }
    });
}

/// Background task that continuously monitors LCU availability and notifies the UI.
pub async fn start_lcu_monitor(app_handle: tauri::AppHandle) {
    let mut current_status = LcuStatus::Disconnected;
    let mut reconnect_attempt: u32 = 0;

    loop {
        match LcuConnection::connect() {
            Ok(conn) => {
                match conn.get_current_summoner().await {
                    Ok(summoner) => {
                        let new_status = LcuStatus::Connected(summoner.clone());
                        if should_emit_status_change(&current_status, &new_status) {
                            let _ = crate::models::LcuConnected(summoner.clone()).emit(&app_handle);
                            current_status = new_status;
                            reset_auto_mark_session_state();
                            spawn_auto_mark_daily_missions_if_enabled();
                        }
                        reconnect_attempt = 0;
                        // Client is connected; check liveness every 3 seconds
                        tokio::time::sleep(Duration::from_secs(3)).await;
                    }
                    Err(_) => {
                        // Process found but summoner endpoint not ready yet (e.g., logging in)
                        let new_status = LcuStatus::Disconnected;
                        if should_emit_status_change(&current_status, &new_status) {
                            let _ = crate::models::LcuDisconnected.emit(&app_handle);
                            current_status = new_status;
                            reset_auto_mark_session_state();
                        }
                        let delay = calculate_backoff_secs(reconnect_attempt);
                        reconnect_attempt = reconnect_attempt.saturating_add(1);
                        tokio::time::sleep(Duration::from_secs(delay)).await;
                    }
                }
            }
            Err(_) => {
                let new_status = LcuStatus::Disconnected;
                if should_emit_status_change(&current_status, &new_status) {
                    let _ = crate::models::LcuDisconnected.emit(&app_handle);
                    current_status = new_status;
                    reset_auto_mark_session_state();
                }
                let delay = calculate_backoff_secs(reconnect_attempt);
                reconnect_attempt = reconnect_attempt.saturating_add(1);
                tokio::time::sleep(Duration::from_secs(delay)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_backoff_secs_caps_at_maximum() {
        assert_eq!(calculate_backoff_secs(0), 2);
        assert_eq!(calculate_backoff_secs(1), 4);
        assert_eq!(calculate_backoff_secs(2), 8);
        assert_eq!(calculate_backoff_secs(3), 10);
        assert_eq!(calculate_backoff_secs(10), 10);
    }

    #[test]
    fn test_should_emit_status_change_transitions() {
        let summoner_a = SummonerInfo {
            summoner_id: 1,
            game_name: "PlayerA".to_string(),
            tag_line: "BR1".to_string(),
            display_name: "PlayerA".to_string(),
            summoner_level: 100,
            profile_icon_id: 10,
        };

        let summoner_b = SummonerInfo {
            summoner_id: 2,
            game_name: "PlayerB".to_string(),
            tag_line: "BR1".to_string(),
            display_name: "PlayerB".to_string(),
            summoner_level: 200,
            profile_icon_id: 20,
        };

        // Transition from Disconnected to Connected
        assert!(should_emit_status_change(
            &LcuStatus::Disconnected,
            &LcuStatus::Connected(summoner_a.clone())
        ));

        // Same status should not re-emit
        assert!(!should_emit_status_change(
            &LcuStatus::Connected(summoner_a.clone()),
            &LcuStatus::Connected(summoner_a.clone())
        ));

        // Account change should emit
        assert!(should_emit_status_change(
            &LcuStatus::Connected(summoner_a.clone()),
            &LcuStatus::Connected(summoner_b.clone())
        ));

        // Transition from Connected to Disconnected
        assert!(should_emit_status_change(
            &LcuStatus::Connected(summoner_a),
            &LcuStatus::Disconnected
        ));

        // Same Disconnected status should not re-emit
        assert!(!should_emit_status_change(
            &LcuStatus::Disconnected,
            &LcuStatus::Disconnected
        ));
    }

    #[test]
    fn test_parse_valid_current_summoner_json() {
        // Real fixture from /lol-summoner/v1/current-summoner
        let fixture = r#"{
            "accountId": 123456789,
            "displayName": "Schuma",
            "gameName": "Schuma",
            "internalName": "schuma",
            "nameChangeFlag": false,
            "percentCompleteForNextLevel": 42,
            "profileIconId": 548,
            "puuid": "00000000-0000-0000-0000-000000000000",
            "rerollPoints": {
                "currentPoints": 250,
                "maxRolls": 2,
                "numberOfRolls": 1,
                "pointsCostToRoll": 250,
                "pointsToReroll": 250
            },
            "summonerId": 987654321,
            "summonerLevel": 350,
            "tagLine": "BR1",
            "unnamed": false,
            "xpSinceLastLevel": 1200,
            "xpUntilNextLevel": 2800
        }"#;

        let result = parse_summoner_info(fixture);
        assert!(result.is_ok());

        let summoner = result.unwrap();
        assert_eq!(summoner.summoner_id, 987654321);
        assert_eq!(summoner.game_name, "Schuma");
        assert_eq!(summoner.tag_line, "BR1");
        assert_eq!(summoner.summoner_level, 350);
        assert_eq!(summoner.profile_icon_id, 548);
    }

    #[test]
    fn test_parse_summoner_fallback_to_display_name() {
        let fixture = r#"{
            "accountId": 123456789,
            "displayName": "LegacyPlayer",
            "gameName": "",
            "profileIconId": 29,
            "summonerId": 11223344,
            "summonerLevel": 45,
            "tagLine": "EUW"
        }"#;

        let result = parse_summoner_info(fixture);
        assert!(result.is_ok());

        let summoner = result.unwrap();
        assert_eq!(summoner.game_name, "LegacyPlayer");
        assert_eq!(summoner.summoner_id, 11223344);
        assert_eq!(summoner.summoner_level, 45);
    }

    #[test]
    fn test_parse_invalid_or_incomplete_json_returns_err() {
        // Simulating error response when client is not authenticated / 404
        let fixture = r#"{
            "errorCode": "RESOURCE_NOT_FOUND",
            "httpStatus": 404,
            "message": "Summoner not found"
        }"#;

        let result = parse_summoner_info(fixture);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_summoner_value_from_json_value() {
        let val = serde_json::json!({
            "summonerId": 55667788,
            "gameName": "Taker",
            "tagLine": "KR1",
            "displayName": "Taker",
            "summonerLevel": 600,
            "profileIconId": 123
        });

        let result = parse_summoner_value(&val);
        assert!(result.is_ok());

        let summoner = result.unwrap();
        assert_eq!(summoner.summoner_id, 55667788);
        assert_eq!(summoner.game_name, "Taker");
        assert_eq!(summoner.tag_line, "KR1");
        assert_eq!(summoner.summoner_level, 600);
        assert_eq!(summoner.profile_icon_id, 123);
    }

    #[test]
    fn test_normalize_epoch_millis_handles_seconds_and_millis() {
        assert_eq!(normalize_epoch_millis(-1), 0);
        assert_eq!(normalize_epoch_millis(0), 0);
        // 10-digit epoch seconds (used by TFT missions/series)
        assert_eq!(normalize_epoch_millis(1_794_398_400), 1_794_398_400_000);
        // 13-digit epoch milliseconds (used by LoL missions)
        assert_eq!(normalize_epoch_millis(1_794_679_200_000), 1_794_679_200_000);
    }

    #[test]
    fn test_extract_unread_mission_ids_from_objectives_is_name_agnostic_and_clears_all_tabs() {
        let now_ms = 1_794_700_000_000i64;
        let now_secs = now_ms / 1000;
        let objectives = serde_json::json!([
            {
                "gameType": "tft",
                "objectivesCategories": [
                    {
                        "id": "cat-renamed-next-season",
                        "startDate": now_secs - 100,
                        "endDate": now_secs + 1000,
                        "objectives": [
                            {
                                "id": "Completely_New_Group_Name_2027",
                                "isEnabled": true,
                                "localizedTag": "Whatever",
                                "localizedTitle": ["Future Mode"],
                                "tag": [],
                                "startDate": now_secs - 100,
                                "endDate": now_secs + 1000,
                                "missions": [
                                    {
                                        "id": "tft-weekly-new-1",
                                        "internalName": "Future_Set_Mission_A",
                                        "isNew": true,
                                        "viewed": false,
                                        "status": "PENDING",
                                        "startTime": now_secs - 50,
                                        "endTime": 9223372036854776000u64
                                    },
                                    {
                                        "id": "lol-weekly-not-viewed",
                                        "internalName": "Arbitrary_Objective_B",
                                        "isNew": false,
                                        "viewed": false,
                                        "status": "COMPLETED",
                                        "startTime": now_ms - 5000,
                                        "endTime": now_ms + 50000
                                    },
                                    {
                                        "id": "already-read",
                                        "isNew": false,
                                        "viewed": true,
                                        "status": "PENDING",
                                        "startTime": now_ms - 5000,
                                        "endTime": now_ms + 50000
                                    },
                                    {
                                        "id": "dummy-mission-ignored",
                                        "isNew": true,
                                        "viewed": false,
                                        "status": "DUMMY",
                                        "startTime": now_ms - 5000,
                                        "endTime": now_ms + 50000
                                    },
                                    {
                                        "id": "expired-mission-ignored",
                                        "isNew": true,
                                        "viewed": false,
                                        "status": "PENDING",
                                        "startTime": now_secs - 500,
                                        "endTime": now_secs - 1
                                    },
                                    {
                                        "id": "tft-weekly-new-1",
                                        "isNew": true,
                                        "viewed": false,
                                        "status": "PENDING",
                                        "startTime": now_secs - 50,
                                        "endTime": now_secs + 500
                                    }
                                ]
                            },
                            {
                                "id": "disabled-group",
                                "isEnabled": false,
                                "missions": [
                                    {
                                        "id": "disabled-group-mission",
                                        "isNew": true,
                                        "viewed": false,
                                        "status": "PENDING"
                                    }
                                ]
                            }
                        ]
                    }
                ]
            }
        ]);

        let ids = extract_unread_mission_ids_from_objectives(&objectives, now_ms);
        assert_eq!(ids, vec!["tft-weekly-new-1", "lol-weekly-not-viewed"]);
    }

    #[test]
    fn test_extract_unread_mission_ids_from_flat_list_uses_structure_not_names() {
        let now_ms = 1_794_700_000_000i64;
        let now_secs = now_ms / 1000;
        let missions = serde_json::json!([
            {
                "id": "visible-tracker-unread",
                "internalName": "Totally_Renamed_Event_Mission",
                "displayType": "ALWAYS",
                "display": { "locations": ["TFT_WEEKLY", "LCU_TRACKER"] },
                "isNew": false,
                "viewed": false,
                "status": "PENDING",
                "startTime": now_secs - 60,
                "endTime": now_secs + 600
            },
            {
                "id": "hidden-but-flagged-is-new",
                "internalName": "Some_Future_Hidden_Mission",
                "displayType": "",
                "display": { "locations": [] },
                "isNew": true,
                "viewed": false,
                "status": "COMPLETED",
                "startTime": now_ms - 1000,
                "endTime": now_ms + 10000
            },
            {
                "id": "internal-boon-without-locations-or-is-new",
                "internalName": "Internal_Game_Mechanic",
                "displayType": "",
                "display": { "locations": [] },
                "isNew": false,
                "viewed": false,
                "status": "PENDING",
                "startTime": 0,
                "endTime": 0
            },
            {
                "id": "dummy-npe-step",
                "displayType": "ALWAYS",
                "display": { "locations": ["NPE_REWARDS_DAILY_PLAY"] },
                "objectives": [{ "status": "DUMMY" }],
                "isNew": false,
                "viewed": false,
                "status": "PENDING",
                "startTime": now_ms - 1000,
                "endTime": now_ms + 10000
            }
        ]);

        let ids = extract_unread_mission_ids_from_missions(&missions, now_ms);
        assert_eq!(
            ids,
            vec!["visible-tracker-unread", "hidden-but-flagged-is-new"]
        );
    }

    #[test]
    fn test_extract_unread_series_ids_filters_and_normalizes_timestamps() {
        let now_ms = 1_794_700_000_000i64;
        let now_secs = now_ms / 1000;
        let series = serde_json::json!([
            {
                "id": "series-tracker-unread",
                "status": "ACTIVE",
                "displayType": "TRACKER",
                "viewed": false,
                "startDate": now_secs - 100,
                "endDate": 0
            },
            {
                "id": "series-already-viewed",
                "status": "ACTIVE",
                "displayType": "TRACKER",
                "viewed": true,
                "startDate": now_secs - 100,
                "endDate": 0
            },
            {
                "id": "series-internal-ignored",
                "status": "ACTIVE",
                "displayType": "INTERNAL",
                "viewed": false,
                "startDate": 0,
                "endDate": 0
            },
            {
                "id": "series-expired-ignored",
                "status": "ACTIVE",
                "displayType": "TRACKER",
                "viewed": false,
                "startDate": now_secs - 200,
                "endDate": now_secs - 10
            }
        ]);

        let ids = extract_unread_series_ids(&series, now_ms);
        assert_eq!(ids, vec!["series-tracker-unread"]);
    }

    #[test]
    fn test_auto_mark_tracker_prevents_infinite_loops_and_resets_when_cleared() {
        let mut tracker = AutoMarkTracker::default();
        let t0 = Instant::now();

        let candidates = vec![("m1".to_string(), true), ("m2".to_string(), false)];
        let series = vec!["s1".to_string()];

        // Attempt 1
        let (m_send, s_send) =
            filter_candidates_with_tracker(&mut tracker, &candidates, &series, t0);
        assert_eq!(m_send.len(), 2);
        assert_eq!(s_send.len(), 1);
        record_sent_with_tracker(&mut tracker, &m_send, &s_send, t0);

        // Within cooldown -> blocked
        let (m_cool, s_cool) = filter_candidates_with_tracker(
            &mut tracker,
            &candidates,
            &series,
            t0 + Duration::from_millis(200),
        );
        assert!(m_cool.is_empty() && s_cool.is_empty());

        // Attempt 2 after cooldown
        let t1 = t0 + Duration::from_secs(2);
        let (m_send2, s_send2) =
            filter_candidates_with_tracker(&mut tracker, &candidates, &series, t1);
        assert_eq!(m_send2.len(), 2);
        assert_eq!(s_send2.len(), 1);
        record_sent_with_tracker(&mut tracker, &m_send2, &s_send2, t1);

        // Attempt 3 -> capped to avoid PUT loop if LCU refuses to flip an optional item
        let t2 = t1 + Duration::from_secs(2);
        let (m_send3, s_send3) =
            filter_candidates_with_tracker(&mut tracker, &candidates, &series, t2);
        assert!(m_send3.is_empty() && s_send3.is_empty());

        // Once m2 clears (disappears from unread list), it is pruned; if it returns with isNew=true later, it runs again
        let t3 = t2 + Duration::from_secs(2);
        let (m_send4, _) =
            filter_candidates_with_tracker(&mut tracker, &[("m1".to_string(), true)], &series, t3);
        assert!(m_send4.is_empty());
        assert!(!tracker.attempts.contains_key("m2"));

        let t4 = t3 + Duration::from_secs(2);
        let (m_send5, _) = filter_candidates_with_tracker(
            &mut tracker,
            &[("m1".to_string(), true), ("m2".to_string(), true)],
            &series,
            t4,
        );
        assert_eq!(m_send5, vec![("m2".to_string(), true)]);
    }

    #[test]
    fn test_connect_lcu_handles_closed_client_gracefully() {
        // Unless LeagueClientUx.exe is actively running on this machine during test execution,
        // LcuConnection::connect should return an Err result without panicking.
        let connection_res = LcuConnection::connect();
        match connection_res {
            Ok(_) => println!("LCU process was detected during test execution."),
            Err(err) => {
                assert!(!err.is_empty());
                println!(
                    "LCU was not detected (expected when client is closed): {}",
                    err
                );
            }
        }
    }
}

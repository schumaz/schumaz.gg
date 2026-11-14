pub use crate::models::{
    ChampSelectedPayload, QUEUE_ARAM, QUEUE_ARAM_MAYHEM, QUEUE_FLEX, QUEUE_NORMAL_DRAFT,
    QUEUE_PRACTICE_TOOL, QUEUE_QUICKPLAY, QUEUE_SOLO_DUO, QueueMode,
};
use std::time::Duration;

/// Lead time before the end of the FINALIZATION phase when safety reconciliation triggers.
pub const FINALIZATION_SAFETY_WINDOW_MS: i64 = 3_500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsSessionTransition {
    Active(ChampSelectedPayload),
    Ended,
    NoChampionSelected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChampSelectTimer {
    pub phase: String,
    pub adjusted_time_left_ms: i64,
    pub is_infinite: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinalizationAction {
    TriggerNow,
    ScheduleAfter(Duration),
    NotApplicable,
}

/// Helper to detect if a session JSON represents a Custom Game or Practice Tool.
pub fn is_custom_or_practice_session(session_json: &serde_json::Value) -> bool {
    session_json
        .get("isCustomGame")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Extracts timer metadata from `/lol-champ-select/v1/session` JSON.
pub fn extract_champ_select_timer(session_json: &serde_json::Value) -> Option<ChampSelectTimer> {
    let timer = session_json.get("timer")?;
    let phase = timer
        .get("phase")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let adjusted_time_left_ms = timer
        .get("adjustedTimeLeftInPhase")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let is_infinite = timer
        .get("isInfinite")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Some(ChampSelectTimer {
        phase,
        adjusted_time_left_ms,
        is_infinite,
    })
}

/// Evaluates whether the session timer requires immediate final reconciliation,
/// scheduling a precision wakeup before FINALIZATION ends, or no action.
pub fn evaluate_finalization_window(
    timer: &ChampSelectTimer,
    window_ms: i64,
) -> FinalizationAction {
    if timer.is_infinite {
        return FinalizationAction::NotApplicable;
    }

    match timer.phase.trim().to_ascii_uppercase().as_str() {
        "FINALIZATION" => {
            if timer.adjusted_time_left_ms <= window_ms {
                FinalizationAction::TriggerNow
            } else {
                let wait_ms = (timer.adjusted_time_left_ms - window_ms).max(0) as u64;
                FinalizationAction::ScheduleAfter(Duration::from_millis(wait_ms))
            }
        }
        "GAME_STARTING" => FinalizationAction::TriggerNow,
        _ => FinalizationAction::NotApplicable,
    }
}

/// Evaluates raw WebSocket message details and determines session state changes.
pub fn evaluate_ws_event(
    event_type: &str,
    data: &serde_json::Value,
    current_queue_id: i32,
) -> WsSessionTransition {
    if event_type == "Delete" {
        return WsSessionTransition::Ended;
    }

    let effective_queue_id = if current_queue_id <= 0 && is_custom_or_practice_session(data) {
        QUEUE_PRACTICE_TOOL
    } else {
        current_queue_id
    };

    let assigned_pos = extract_assigned_position(data);

    match extract_selected_champion(data) {
        Ok(Some((champ_id, locked_in))) => WsSessionTransition::Active(ChampSelectedPayload {
            champion_id: champ_id,
            queue_id: effective_queue_id,
            queue_mode: QueueMode::from(effective_queue_id),
            locked_in,
            assigned_position: assigned_pos,
        }),
        Ok(None) => WsSessionTransition::NoChampionSelected,
        Err(_) => WsSessionTransition::NoChampionSelected,
    }
}

/// Deduplicates timer ticks and only triggers an emit when champion or lock-in state transitions.
pub fn should_emit_champ_select(
    last_payload: &Option<ChampSelectedPayload>,
    current_payload: &Option<ChampSelectedPayload>,
) -> bool {
    last_payload != current_payload
}

/// Returns `true` when the player transitions into a locked-in champion state
/// (or swaps/trades into a different locked-in champion) across any queue/mode.
/// Hovering (`locked_in == false`) and metadata-only updates for an already locked
/// champion return `false` so the taskbar alert fires only on actual lock-ins/swaps.
pub fn should_request_lock_in_attention(
    prev_payload: Option<&ChampSelectedPayload>,
    next_payload: &ChampSelectedPayload,
) -> bool {
    if !next_payload.locked_in {
        return false;
    }

    match prev_payload {
        None => true,
        Some(prev) => !prev.locked_in || prev.champion_id != next_payload.champion_id,
    }
}

/// Pure parser function to extract the selected `championId` and `locked_in` status.
///
/// Precedence:
/// 1. `myTeam` where `cellId == localPlayerCellId` and `championId > 0`: authoritative for locked picks,
///    Draft champion trades/swaps during FINALIZATION, and ARAM/ARAM Mayhem bench swaps.
/// 2. Draft and Practice Tool pick actions via `actions` where `type == "pick"` and `actorCellId == localPlayerCellId`.
/// 3. Draft `championPickIntent` fallback from `myTeam`.
///
/// Explicitly ignores `"ban"` actions so banning a champion never triggers a build recommendation.
pub fn extract_selected_champion(
    session_json: &serde_json::Value,
) -> Result<Option<(i32, bool)>, String> {
    let local_player_cell_id = session_json
        .get("localPlayerCellId")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "Missing 'localPlayerCellId' in session JSON".to_string())?
        as i32;

    // 1. Check authoritative locked/swapped champion in `myTeam` first so trades in FINALIZATION
    //    and ARAM bench swaps always reflect the actual final champion even if `actions` keeps the old pick.
    let mut local_intent_id = 0;
    if let Some(my_team) = session_json.get("myTeam").and_then(|v| v.as_array()) {
        for member in my_team {
            let cell_id = member.get("cellId").and_then(|v| v.as_i64()).unwrap_or(-1) as i32;

            if cell_id == local_player_cell_id {
                let champ_id = member
                    .get("championId")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;

                if champ_id > 0 {
                    return Ok(Some((champ_id, true)));
                }

                local_intent_id = member
                    .get("championPickIntent")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;
            }
        }
    }

    // 2. Check draft actions (Solo/Duo, Flex, Normal Draft, Practice Tool) for active/hovered picks
    if let Some(actions) = session_json.get("actions").and_then(|v| v.as_array()) {
        for action_group in actions {
            if let Some(sub_actions) = action_group.as_array() {
                for action in sub_actions {
                    let action_type = action.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    let actor_cell_id = action
                        .get("actorCellId")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(-1) as i32;

                    // Only consider "pick" actions for the local player (strictly ignore "ban")
                    if action_type == "pick" && actor_cell_id == local_player_cell_id {
                        let champ_id = action
                            .get("championId")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0) as i32;
                        let completed = action
                            .get("completed")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        if champ_id > 0 {
                            return Ok(Some((champ_id, completed)));
                        }
                    }
                }
            }
        }
    }

    // 3. Fallback to draft pick intent before player's pick turn
    if local_intent_id > 0 {
        return Ok(Some((local_intent_id, false)));
    }

    Ok(None)
}

/// Normalizes draft `assignedPosition` strings ("top", "jungle", "middle", "bottom", "utility")
/// to standard OP.GG positions ("top", "jungle", "mid", "adc", "support").
pub fn normalize_position(pos: &str) -> Option<String> {
    let lower = pos.trim().to_lowercase();
    match lower.as_str() {
        "top" => Some("top".to_string()),
        "jungle" | "jng" => Some("jungle".to_string()),
        "middle" | "mid" => Some("mid".to_string()),
        "bottom" | "bot" | "adc" => Some("adc".to_string()),
        "utility" | "support" | "supp" => Some("support".to_string()),
        _ => None,
    }
}

/// Extracts the local player's `assignedPosition` from the draft `myTeam` payload.
pub fn extract_assigned_position(session_json: &serde_json::Value) -> Option<String> {
    let local_player_cell_id = session_json
        .get("localPlayerCellId")
        .and_then(|v| v.as_i64())? as i32;

    let my_team = session_json.get("myTeam").and_then(|v| v.as_array())?;
    for member in my_team {
        let cell_id = member.get("cellId").and_then(|v| v.as_i64()).unwrap_or(-1) as i32;
        if cell_id == local_player_cell_id
            && let Some(pos) = member.get("assignedPosition").and_then(|v| v.as_str())
        {
            return normalize_position(pos);
        }
    }
    None
}

/// Extracts the queue ID from a `GET /lol-gameflow/v1/session` JSON payload.
/// Supports Standard queues, ARAM Mayhem, Practice Tool, and Custom games.
pub fn extract_queue_id(gameflow_json: &serde_json::Value) -> Result<i32, String> {
    // 1. Check for Practice Tool, ARAM Mayhem, or custom game indicators in gameMode
    let game_mode = gameflow_json
        .pointer("/gameData/queue/gameMode")
        .or_else(|| gameflow_json.pointer("/gameData/gameMode"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if game_mode.eq_ignore_ascii_case("PRACTICETOOL")
        || game_mode.eq_ignore_ascii_case("PRACTICE_TOOL")
    {
        return Ok(QUEUE_PRACTICE_TOOL);
    }

    if game_mode.eq_ignore_ascii_case("ARAM_MAYHEM") || game_mode.to_uppercase().contains("MAYHEM")
    {
        return Ok(QUEUE_ARAM_MAYHEM);
    }

    let is_custom = gameflow_json
        .pointer("/gameData/isCustomGame")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let id_opt = gameflow_json
        .pointer("/gameData/queue/id")
        .and_then(|v| v.as_i64())
        .map(|id| id as i32);

    if let Some(id) = id_opt {
        if id == -1 || (is_custom && id <= 0) {
            return Ok(QUEUE_PRACTICE_TOOL);
        }
        return Ok(id);
    }

    if is_custom {
        return Ok(QUEUE_PRACTICE_TOOL);
    }

    Err("Could not find '/gameData/queue/id' in gameflow session JSON".to_string())
}

#[cfg(test)]
#[path = "extract_tests.rs"]
pub mod tests;

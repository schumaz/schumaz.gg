use super::extract::{
    ChampSelectedPayload, LobbyPreselectSlot, QueueMode, is_lobby_preselect_queue,
    is_swiftplay_game_mode, normalize_position,
};
use crate::build_provider::DataDragonCache;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LobbyPreselectTransition {
    Active(ChampSelectedPayload),
    Ended,
    NotPreselectQueue,
    NoChampionSelected,
}

/// Extracts `queueId` from `/lol-lobby/v2/lobby`, falling back to `QUEUE_SWIFTPLAY` (480)
/// when `gameConfig.gameMode` identifies Swiftplay.
pub fn extract_lobby_queue_id(lobby_json: &serde_json::Value) -> Option<i32> {
    if let Some(id) = lobby_json
        .pointer("/gameConfig/queueId")
        .or_else(|| lobby_json.get("queueId"))
        .and_then(|v| v.as_i64())
        .map(|id| id as i32)
        && id > 0
    {
        return Some(id);
    }

    let game_mode = lobby_json
        .pointer("/gameConfig/gameMode")
        .or_else(|| lobby_json.get("gameMode"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if is_swiftplay_game_mode(game_mode) {
        return Some(crate::models::QUEUE_SWIFTPLAY);
    }

    None
}

/// Extracts filled champion slots from `/lol-lobby/v2/lobby` (`localMember.playerSlots`).
pub fn extract_lobby_player_slots(
    lobby_json: &serde_json::Value,
    ddragon: Option<&DataDragonCache>,
) -> Vec<LobbyPreselectSlot> {
    let Some(slots) = lobby_json
        .pointer("/localMember/playerSlots")
        .and_then(|v| v.as_array())
    else {
        return Vec::new();
    };

    slots
        .iter()
        .enumerate()
        .filter_map(|(idx, slot)| {
            let champ_id = slot.get("championId").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            if champ_id <= 0 {
                return None;
            }
            let assigned_position = slot
                .get("positionPreference")
                .and_then(|v| v.as_str())
                .and_then(normalize_position);
            let meta = ddragon.and_then(|cache| cache.champions.get(&champ_id));
            Some(LobbyPreselectSlot {
                slot_index: idx as u32,
                champion_id: champ_id,
                champion_name: meta.map(|m| m.display_name.clone()).unwrap_or_default(),
                champion_icon_url: meta.map(|m| m.icon_url.clone()).unwrap_or_default(),
                assigned_position,
            })
        })
        .collect()
}

/// Evaluates raw `/lol-lobby/v2/lobby` WebSocket/poll payloads for Swiftplay/Quickplay pre-selection.
pub fn evaluate_lobby_ws_event(
    event_type: &str,
    data: &serde_json::Value,
    preferred_slot_index: Option<u32>,
    ddragon: Option<&DataDragonCache>,
) -> LobbyPreselectTransition {
    if event_type == "Delete" {
        return LobbyPreselectTransition::Ended;
    }

    let Some(queue_id) = extract_lobby_queue_id(data) else {
        return LobbyPreselectTransition::NotPreselectQueue;
    };
    if !is_lobby_preselect_queue(queue_id) {
        return LobbyPreselectTransition::NotPreselectQueue;
    }

    let lobby_slots = extract_lobby_player_slots(data, ddragon);
    if lobby_slots.is_empty() {
        return LobbyPreselectTransition::NoChampionSelected;
    }

    let active = preferred_slot_index
        .and_then(|idx| lobby_slots.iter().find(|s| s.slot_index == idx))
        .unwrap_or(&lobby_slots[0])
        .clone();

    LobbyPreselectTransition::Active(ChampSelectedPayload {
        champion_id: active.champion_id,
        queue_id,
        queue_mode: QueueMode::from(queue_id),
        locked_in: true,
        assigned_position: active.assigned_position,
        is_lobby_preselect: true,
        active_slot_index: Some(active.slot_index),
        lobby_slots,
    })
}

/// Canonical comparison tuple ignoring perks/spells/metadata so our own `PUT /player-slots`
/// never triggers redundant rebuild/re-injection loops.
pub fn lobby_slots_signature(payload: &ChampSelectedPayload) -> Vec<(u32, i32, Option<String>)> {
    payload
        .lobby_slots
        .iter()
        .map(|s| (s.slot_index, s.champion_id, s.assigned_position.clone()))
        .collect()
}

pub fn lobby_payload_changed(
    prev: Option<&ChampSelectedPayload>,
    next: &ChampSelectedPayload,
) -> bool {
    let Some(p) = prev else {
        return true;
    };
    p.is_lobby_preselect != next.is_lobby_preselect
        || p.queue_id != next.queue_id
        || p.active_slot_index != next.active_slot_index
        || lobby_slots_signature(p) != lobby_slots_signature(next)
}

fn entry_matches_local_player(
    entry: &serde_json::Value,
    summoner_id: u64,
    puuid: Option<&str>,
) -> bool {
    if summoner_id > 0
        && entry.get("summonerId").and_then(|v| v.as_u64()) == Some(summoner_id)
    {
        return true;
    }
    puuid.is_some_and(|p| {
        !p.is_empty() && entry.get("puuid").and_then(|v| v.as_str()) == Some(p)
    })
}

/// Extracts the champion ID actually assigned to the local player in `/lol-gameflow/v1/session`
/// once a Swiftplay/Quickplay match starts (`teamOne`, `teamTwo`, `playerChampionSelections`).
pub fn extract_assigned_match_champion_id(
    gameflow_session: &serde_json::Value,
    summoner_id: u64,
    puuid: Option<&str>,
) -> Option<i32> {
    for ptr in [
        "/gameData/teamOne",
        "/gameData/teamTwo",
        "/gameData/playerChampionSelections",
    ] {
        let Some(entries) = gameflow_session.pointer(ptr).and_then(|v| v.as_array()) else {
            continue;
        };
        for entry in entries {
            if entry_matches_local_player(entry, summoner_id, puuid)
                && let Some(champ_id) = entry
                    .get("championId")
                    .and_then(|v| v.as_i64())
                    .map(|id| id as i32)
                    .filter(|&id| id > 0)
            {
                return Some(champ_id);
            }
        }
    }
    None
}

#[cfg(test)]
#[path = "lobby_tests.rs"]
mod tests;

use tauri_specta::Event as SpectaEvent;

use super::lobby::extract_assigned_match_champion_id;
use super::reconcile_state::{
    build_active_champ_select_snapshot, is_lobby_phase_editable, shared_last_gameflow_phase,
    shared_session_state,
};
use crate::build_provider::ChampionBuild;
use crate::models::{self, ActiveChampSelectSnapshot};
use crate::{lcu_connector::LcuConnection, lcu_injector, logging};

/// Switches the active Swiftplay/Quickplay pre-selection slot (e.g. Primary vs Secondary)
/// and emits the corresponding `ChampSelected` + `RecommendedBuild` events.
pub async fn select_lobby_preselect_slot_service(
    slot_index: u32,
    app_handle: &tauri::AppHandle,
) -> Result<Option<ActiveChampSelectSnapshot>, String> {
    let state_ref = shared_session_state();
    let phase_ref = shared_last_gameflow_phase();

    let (updated_payload, cached_build) = {
        let mut guard = state_ref.lock().await;
        let Some(ref mut payload) = guard.last_payload else {
            return Ok(None);
        };
        if !payload.is_lobby_preselect {
            return Ok(None);
        }
        let Some(target) = payload
            .lobby_slots
            .iter()
            .find(|s| s.slot_index == slot_index)
            .cloned()
        else {
            return Err(format!("Lobby slot {slot_index} is empty or unavailable"));
        };
        payload.active_slot_index = Some(slot_index);
        payload.champion_id = target.champion_id;
        payload.assigned_position = target.assigned_position;
        let snapshot_payload = payload.clone();
        let cached = guard.lobby_builds.get(&slot_index).cloned();
        if let Some(ref b) = cached {
            guard.last_build = Some(b.clone());
        }
        (snapshot_payload, cached)
    };

    let build = if let Some(b) = cached_build {
        b
    } else {
        let client = crate::build_provider::shared_http_client();
        let fetched = crate::build_provider::get_recommended_build_with_filters(
            client,
            updated_payload.champion_id,
            updated_payload.queue_id,
            updated_payload.assigned_position.as_deref(),
            None,
            None,
            None,
        )
        .await?;
        let mut guard = state_ref.lock().await;
        if guard
            .last_payload
            .as_ref()
            .is_some_and(|p| p.is_lobby_preselect && p.active_slot_index == Some(slot_index))
        {
            guard.lobby_builds.insert(slot_index, fetched.clone());
            guard.last_build = Some(fetched.clone());
        }
        fetched
    };

    logging::log_info(
        "ws.lobby",
        format!(
            "event=lobby_slot_switched slot_index={slot_index} champ_id={}",
            updated_payload.champion_id
        ),
    );
    let _ = models::ChampSelected(updated_payload).emit(app_handle);
    let _ = models::RecommendedBuild(build).emit(app_handle);

    let phase = phase_ref.lock().await.clone();
    let guard = state_ref.lock().await;
    Ok(Some(build_active_champ_select_snapshot(&phase, &guard)))
}

/// Auto-switches the displayed build when a Swiftplay match starts (`GameStart`/`InProgress`)
/// and Riot assigned the player's Secondary champion instead of the currently focused slot.
pub(crate) async fn sync_swiftplay_assigned_champion_on_match_start(handle: tauri::AppHandle) {
    let (active_slot, slots) = {
        let state_ref = shared_session_state();
        let guard = state_ref.lock().await;
        let Some(ref p) = guard.last_payload else {
            return;
        };
        if !p.is_lobby_preselect || p.lobby_slots.len() <= 1 {
            return;
        }
        (p.active_slot_index, p.lobby_slots.clone())
    };

    let Ok(conn) = LcuConnection::connect() else {
        return;
    };
    let Ok((summoner, puuid)) = conn.get_current_summoner_with_puuid().await else {
        return;
    };
    let Ok(Some(session_json)) = conn.get_gameflow_session().await else {
        return;
    };
    let Some(assigned_champ_id) =
        extract_assigned_match_champion_id(&session_json, summoner.summoner_id, puuid.as_deref())
    else {
        return;
    };
    if let Some(matched) = slots.iter().find(|s| s.champion_id == assigned_champ_id)
        && active_slot != Some(matched.slot_index)
    {
        let _ = select_lobby_preselect_slot_service(matched.slot_index, &handle).await;
    }
}

/// Re-injects an active Swiftplay/Quickplay slot after UI filter changes (`tier`/`position`/`region`/`patch`)
/// while keeping the companion slot's shop item set and `playerSlots` intact.
pub(crate) async fn reinject_active_lobby_filter_update(
    champion_id: i32,
    build: &ChampionBuild,
    app_handle: &tauri::AppHandle,
) -> bool {
    let state_ref = shared_session_state();
    let (slot_idx, slot_builds) = {
        let mut guard = state_ref.lock().await;
        let Some(ref payload) = guard.last_payload else {
            return false;
        };
        if !payload.is_lobby_preselect {
            return false;
        }
        let target_slot = payload
            .active_slot_index
            .or_else(|| {
                payload
                    .lobby_slots
                    .iter()
                    .find(|s| s.champion_id == champion_id)
                    .map(|s| s.slot_index)
            })
            .unwrap_or(0);
        guard.lobby_builds.insert(target_slot, build.clone());
        guard.last_build = Some(build.clone());
        let mut builds: Vec<(u32, ChampionBuild)> = guard
            .lobby_builds
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        builds.sort_by_key(|(k, _)| *k);
        (target_slot, builds)
    };

    if let Ok(conn) = LcuConnection::connect()
        && let Ok(summoner) = conn.get_current_summoner().await
    {
        let phase = shared_last_gameflow_phase().lock().await.clone();
        let report = lcu_injector::inject_lobby_preselect_builds(
            &conn,
            summoner.summoner_id,
            slot_idx,
            &slot_builds,
            is_lobby_phase_editable(&phase),
        )
        .await;
        let _ = models::BuildInjected(report).emit(app_handle);
    }
    true
}

use std::sync::Arc;
use std::sync::atomic::Ordering;
use tauri_specta::Event as SpectaEvent;

use super::extract::ChampSelectedPayload;
use super::gameflow::{is_in_game_phase, process_gameflow_phase_update};
use super::lobby::{LobbyPreselectTransition, evaluate_lobby_ws_event, lobby_payload_changed};
use super::reconcile_state::is_lobby_phase_editable;
use super::subscription::{WsListenerCtx, clear_active_session};
use crate::build_provider::{ChampionBuild, DataDragonCache};
use crate::models;
use crate::{lcu_connector::LcuConnection, lcu_injector, logging};

fn peek_ram_ddragon() -> Option<Arc<DataDragonCache>> {
    let locale = lcu_injector::get_active_locale();
    crate::build_provider::ddragon::get_ddragon_cache_store()
        .read()
        .ok()
        .and_then(|g| g.get(&locale).cloned())
}

fn enrich_payload_slots_with_ddragon(
    payload: &mut ChampSelectedPayload,
    ddragon: &DataDragonCache,
) -> bool {
    let mut updated = false;
    for slot in &mut payload.lobby_slots {
        if let Some(meta) = ddragon.champions.get(&slot.champion_id) {
            if slot.champion_name.is_empty() {
                slot.champion_name = meta.display_name.clone();
                updated = true;
            }
            if slot.champion_icon_url.is_empty() {
                slot.champion_icon_url = meta.icon_url.clone();
                updated = true;
            }
        }
    }
    updated
}

async fn fetch_lobby_slot_builds(payload: &ChampSelectedPayload) -> Vec<(u32, ChampionBuild)> {
    let http_client = crate::build_provider::shared_http_client();
    let mut join_set = tokio::task::JoinSet::new();
    for slot in &payload.lobby_slots {
        let client = http_client.clone();
        let (slot_idx, champ_id, pos, q_id) = (
            slot.slot_index,
            slot.champion_id,
            slot.assigned_position.clone(),
            payload.queue_id,
        );
        join_set.spawn(async move {
            let res = crate::build_provider::get_recommended_build_with_filters(
                &client,
                champ_id,
                q_id,
                pos.as_deref(),
                None,
                None,
                None,
            )
            .await;
            (slot_idx, res)
        });
    }

    let mut slot_builds = Vec::new();
    while let Some(joined) = join_set.join_next().await {
        if let Ok((slot_idx, Ok(build))) = joined {
            slot_builds.push((slot_idx, build));
        }
    }
    slot_builds.sort_by_key(|(idx, _)| *idx);
    slot_builds
}

async fn run_lobby_preselect_build_and_inject(
    ctx: WsListenerCtx,
    mut payload: ChampSelectedPayload,
    my_seq: u64,
) {
    let http_client = crate::build_provider::shared_http_client();
    if let Ok(ddragon) = crate::build_provider::get_or_init_ddragon_cache(http_client).await
        && enrich_payload_slots_with_ddragon(&mut payload, &ddragon)
        && ctx.coordinator.is_current(my_seq)
    {
        {
            let mut guard = ctx.session_state.lock().await;
            if ctx.coordinator.is_current(my_seq) {
                guard.last_payload = Some(payload.clone());
            }
        }
        let _ = models::ChampSelected(payload.clone()).emit(&ctx.app_handle);
    }

    let slot_builds = fetch_lobby_slot_builds(&payload).await;
    if !ctx.coordinator.is_current(my_seq) || slot_builds.is_empty() {
        return;
    }

    let active_slot_idx = payload
        .active_slot_index
        .filter(|idx| slot_builds.iter().any(|(i, _)| i == idx))
        .unwrap_or(slot_builds[0].0);

    let Some((_, active_build)) = slot_builds
        .iter()
        .find(|(idx, _)| *idx == active_slot_idx)
        .or_else(|| slot_builds.first())
    else {
        return;
    };
    let active_build = active_build.clone();

    {
        let _inject_guard = ctx.coordinator.lock_injection().await;
        if !ctx.coordinator.is_current(my_seq) {
            return;
        }
        {
            let mut guard = ctx.session_state.lock().await;
            if ctx.coordinator.is_current(my_seq) {
                for (slot_idx, build) in &slot_builds {
                    guard.lobby_builds.insert(*slot_idx, build.clone());
                }
                guard.last_build = Some(active_build.clone());
            }
        }

        let _ = models::RecommendedBuild(active_build).emit(&ctx.app_handle);

        let Ok(conn) = LcuConnection::connect() else {
            return;
        };
        let Ok(summoner) = conn.get_current_summoner().await else {
            return;
        };
        if !ctx.coordinator.is_current(my_seq) {
            return;
        }

        let phase = ctx.last_gameflow_phase.lock().await.clone();
        let report = lcu_injector::inject_lobby_preselect_builds(
            &conn,
            summoner.summoner_id,
            active_slot_idx,
            &slot_builds,
            is_lobby_phase_editable(&phase),
        )
        .await;

        if ctx.coordinator.is_current(my_seq) {
            let _ = models::BuildInjected(report).emit(&ctx.app_handle);
        }
    }
}

/// Processes `/lol-lobby/v2/lobby` updates and polls without interfering with standard Champ Select.
pub(crate) async fn handle_lobby_ws_payload(
    ctx: &WsListenerCtx,
    event_type: &str,
    data: &serde_json::Value,
) {
    let (preferred_slot, existing_is_champ_select, existing_is_lobby) = {
        let guard = ctx.session_state.lock().await;
        match guard.last_payload.as_ref() {
            Some(p) if p.is_lobby_preselect => (p.active_slot_index, false, true),
            Some(_) => (None, true, false),
            None => (None, false, false),
        }
    };
    let current_phase = ctx.last_gameflow_phase.lock().await.clone();
    if existing_is_champ_select || current_phase == "ChampSelect" {
        return;
    }

    let ddragon = peek_ram_ddragon();
    match evaluate_lobby_ws_event(event_type, data, preferred_slot, ddragon.as_deref()) {
        LobbyPreselectTransition::Active(payload) => {
            let should_spawn = {
                let mut guard = ctx.session_state.lock().await;
                if !lobby_payload_changed(guard.last_payload.as_ref(), &payload) {
                    false
                } else {
                    let prev_slots = guard
                        .last_payload
                        .as_ref()
                        .map(|p| p.lobby_slots.clone())
                        .unwrap_or_default();
                    let active_changed = guard.last_payload.as_ref().is_none_or(|p| {
                        p.champion_id != payload.champion_id
                            || p.assigned_position != payload.assigned_position
                    });
                    guard.lobby_builds.retain(|slot_idx, _| {
                        let next_slot = payload.lobby_slots.iter().find(|s| s.slot_index == *slot_idx);
                        let prev_slot = prev_slots.iter().find(|s| s.slot_index == *slot_idx);
                        matches!((prev_slot, next_slot), (Some(a), Some(b)) if a.champion_id == b.champion_id && a.assigned_position == b.assigned_position)
                    });
                    if active_changed {
                        guard.last_build = payload
                            .active_slot_index
                            .and_then(|idx| guard.lobby_builds.get(&idx).cloned());
                    }
                    guard.last_payload = Some(payload.clone());
                    ctx.active_queue_id.store(payload.queue_id, Ordering::Relaxed);
                    logging::log_info(
                        "ws.lobby",
                        format!(
                            "event=lobby_preselect_updated champ_id={} queue_id={} slots={}",
                            payload.champion_id,
                            payload.queue_id,
                            payload.lobby_slots.len()
                        ),
                    );
                    let _ = models::ChampSelected(payload.clone()).emit(&ctx.app_handle);
                    true
                }
            };

            if should_spawn {
                let ctx_clone = ctx.clone();
                ctx.coordinator
                    .spawn_latest(move |my_seq, _| {
                        run_lobby_preselect_build_and_inject(ctx_clone, payload, my_seq)
                    })
                    .await;
            }
        }
        LobbyPreselectTransition::Ended
        | LobbyPreselectTransition::NotPreselectQueue
        | LobbyPreselectTransition::NoChampionSelected => {
            if !existing_is_lobby {
                return;
            }
            if let Ok(conn) = LcuConnection::connect()
                && let Ok(live_phase) = conn.get_gameflow_phase().await
            {
                process_gameflow_phase_update(
                    &ctx.app_handle,
                    &ctx.last_gameflow_phase,
                    live_phase,
                )
                .await;
            }
            let latest_phase = ctx.last_gameflow_phase.lock().await.clone();
            if is_in_game_phase(&latest_phase) {
                return;
            }
            logging::log_info("ws.lobby", "event=lobby_preselect_ended");
            clear_active_session(ctx, true).await;
        }
    }
}

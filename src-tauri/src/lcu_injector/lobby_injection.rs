use super::injection::{
    format_injection_report_message, inject_runes,
};
use super::injection_state::{log_injection_outcome, resolve_injection_spell_pair};
use super::item_sets::sync_item_sets_with_store;
use super::lobby_payloads::{LobbySlotPatch, patch_lobby_player_slots_value};
use super::preferences::{UserPreferences, get_user_preferences};
use super::selections::get_champion_selections;
use crate::build_provider::ChampionBuild;
use crate::lcu_connector::LcuConnection;
use crate::models::InjectionReport;
use std::sync::atomic::Ordering;

pub trait LobbyInjectionSink: Send + Sync {
    fn patch_player_slots(
        &self,
        patches: &[LobbySlotPatch],
    ) -> impl std::future::Future<Output = Result<bool, String>> + Send;

    fn inject_runes_for_build(
        &self,
        build: &ChampionBuild,
    ) -> impl std::future::Future<Output = Result<(), String>> + Send;

    fn inject_multi_item_sets(
        &self,
        summoner_id: u64,
        builds: &[&ChampionBuild],
    ) -> impl std::future::Future<Output = Result<(), String>> + Send;
}

impl LobbyInjectionSink for LcuConnection {
    async fn patch_player_slots(&self, patches: &[LobbySlotPatch]) -> Result<bool, String> {
        patch_lobby_slots_on_lcu(self, patches).await
    }

    async fn inject_runes_for_build(&self, build: &ChampionBuild) -> Result<(), String> {
        inject_runes(self, build).await
    }

    async fn inject_multi_item_sets(
        &self,
        summoner_id: u64,
        builds: &[&ChampionBuild],
    ) -> Result<(), String> {
        inject_multi_item_sets(self, summoner_id, builds).await
    }
}

/// Injects combined item sets for multiple champion builds in a single LCU PUT call,
/// preserving player-created sets and skipping unchanged PUTs.
pub async fn inject_multi_item_sets(
    conn: &LcuConnection,
    summoner_id: u64,
    builds: &[&ChampionBuild],
) -> Result<(), String> {
    sync_item_sets_with_store(conn, summoner_id, builds).await?;
    Ok(())
}

async fn fetch_current_player_slots(conn: &LcuConnection) -> Result<serde_json::Value, String> {
    if let Ok(Some(slots)) = conn.get_lobby_player_slots().await
        && slots.is_array()
    {
        return Ok(slots);
    }

    let lobby = conn
        .get_lobby_session()
        .await?
        .ok_or_else(|| "Lobby session is not active".to_string())?;
    lobby
        .pointer("/localMember/playerSlots")
        .filter(|v| v.is_array())
        .cloned()
        .ok_or_else(|| "Missing 'localMember.playerSlots' in lobby payload".to_string())
}

/// Fetches current lobby `playerSlots`, applies rune/spell patches, and PUTs only if changed.
pub async fn patch_lobby_slots_on_lcu(
    conn: &LcuConnection,
    patches: &[LobbySlotPatch],
) -> Result<bool, String> {
    if patches.is_empty() {
        return Ok(false);
    }
    let current = fetch_current_player_slots(conn).await?;
    let (patched, changed) = patch_lobby_player_slots_value(&current, patches)?;
    if changed {
        conn.put_lobby_player_slots(&patched).await?;
    }
    Ok(changed)
}

fn resolve_build_runes_for_slot(build: &ChampionBuild) -> Option<(i32, i32, Vec<i32>)> {
    if !build.supports_runes {
        return None;
    }
    if let Some(sel) = get_champion_selections(build.champion_id)
        && let Some(idx) = sel.rune_page_index
        && let Some(page) = build.rune_pages.get(idx)
    {
        return Some((
            page.primary_style_id,
            page.sub_style_id,
            page.selected_perk_ids.clone(),
        ));
    }
    Some((
        build.primary_style_id,
        build.sub_style_id,
        build.selected_perk_ids.clone(),
    ))
}

/// Injects Swiftplay/Quickplay pre-selection builds against an abstract sink:
/// patches `playerSlots` (when editable), updates the reusable `/lol-perks/v1/pages` page
/// for `active_slot_index`, and pushes combined item sets for all filled slots.
pub async fn inject_lobby_preselect_with_sink(
    sink: &impl LobbyInjectionSink,
    summoner_id: u64,
    active_slot_index: u32,
    slot_builds: &[(u32, ChampionBuild)],
    can_patch_player_slots: bool,
    prefs: &UserPreferences,
) -> InjectionReport {
    let active_build = slot_builds
        .iter()
        .find(|(idx, _)| *idx == active_slot_index)
        .or_else(|| slot_builds.first())
        .map(|(_, b)| b);

    let Some(active) = active_build else {
        return InjectionReport {
            champion_name: String::new(),
            runes_injected: false,
            spells_injected: false,
            items_injected: false,
            message: format_injection_report_message(
                prefs.language,
                true,
                true,
                None,
                None,
                None,
            ),
        };
    };

    let patches: Vec<LobbySlotPatch> = slot_builds
        .iter()
        .map(|(slot_idx, build)| {
            let runes = if prefs.auto_inject_runes {
                resolve_build_runes_for_slot(build)
            } else {
                None
            };
            let spells = if prefs.auto_inject_spells && build.supports_spells {
                Some(resolve_injection_spell_pair(build, prefs.flash_key))
            } else {
                None
            };
            LobbySlotPatch {
                slot_index: Some(*slot_idx),
                champion_id: build.champion_id,
                runes,
                spells,
            }
        })
        .collect();

    let should_patch_slots = can_patch_player_slots
        && !patches.is_empty()
        && (prefs.auto_inject_runes || prefs.auto_inject_spells);
    let should_inject_rune_page = prefs.auto_inject_runes && active.supports_runes;
    let should_inject_items = prefs.auto_inject_items;

    let slots_fut = async {
        if should_patch_slots {
            Some(sink.patch_player_slots(&patches).await)
        } else {
            None
        }
    };
    let rune_page_fut = async {
        if should_inject_rune_page {
            Some(sink.inject_runes_for_build(active).await)
        } else {
            None
        }
    };
    let build_refs: Vec<&ChampionBuild> = slot_builds.iter().map(|(_, b)| b).collect();
    let items_fut = async {
        if should_inject_items {
            Some(sink.inject_multi_item_sets(summoner_id, &build_refs).await)
        } else {
            None
        }
    };

    let (slots_res, rune_page_res, items_res) = tokio::join!(slots_fut, rune_page_fut, items_fut);

    let runes_res: Option<Result<(), String>> = if should_inject_rune_page {
        match (
            rune_page_res,
            if can_patch_player_slots {
                slots_res.as_ref()
            } else {
                None
            },
        ) {
            (Some(Err(e)), _) => Some(Err(e)),
            (_, Some(Err(e))) => Some(Err(e.clone())),
            (Some(Ok(())), _) => Some(Ok(())),
            (None, Some(Ok(_))) => Some(Ok(())),
            (None, None) => None,
        }
    } else {
        None
    };

    let spells_res: Option<Result<(), String>> =
        if can_patch_player_slots && prefs.auto_inject_spells && active.supports_spells {
            slots_res.map(|res| res.map(|_| ()))
        } else {
            None
        };

    let active_queue_id = crate::ws_listener::shared_active_queue_id().load(Ordering::Relaxed);
    log_injection_outcome(
        active.champion_id,
        active_queue_id,
        runes_res.as_ref(),
        spells_res.as_ref(),
        items_res.as_ref(),
    );

    let message = format_injection_report_message(
        prefs.language,
        active.supports_runes,
        active.supports_spells,
        runes_res.as_ref(),
        spells_res.as_ref(),
        items_res.as_ref(),
    );

    InjectionReport {
        champion_name: active.champion_name.clone(),
        runes_injected: runes_res.as_ref().is_some_and(Result::is_ok),
        spells_injected: spells_res.as_ref().is_some_and(Result::is_ok),
        items_injected: items_res.as_ref().is_some_and(Result::is_ok),
        message,
    }
}

/// Injects Swiftplay/Quickplay pre-selection builds into the live LCU.
pub async fn inject_lobby_preselect_builds(
    conn: &LcuConnection,
    summoner_id: u64,
    active_slot_index: u32,
    slot_builds: &[(u32, ChampionBuild)],
    can_patch_player_slots: bool,
) -> InjectionReport {
    let prefs = get_user_preferences();
    inject_lobby_preselect_with_sink(
        conn,
        summoner_id,
        active_slot_index,
        slot_builds,
        can_patch_player_slots,
        &prefs,
    )
    .await
}

/// Updates only runes for a single Swiftplay/Quickplay lobby slot.
pub async fn patch_single_lobby_slot_runes(
    conn: &LcuConnection,
    slot_index: Option<u32>,
    champion_id: i32,
    primary_style_id: i32,
    sub_style_id: i32,
    selected_perk_ids: &[i32],
) -> Result<bool, String> {
    let patch = LobbySlotPatch {
        slot_index,
        champion_id,
        runes: Some((primary_style_id, sub_style_id, selected_perk_ids.to_vec())),
        spells: None,
    };
    patch_lobby_slots_on_lcu(conn, &[patch]).await
}

/// Updates only summoner spells for a single Swiftplay/Quickplay lobby slot.
pub async fn patch_single_lobby_slot_spells(
    conn: &LcuConnection,
    slot_index: Option<u32>,
    champion_id: i32,
    spell1_id: i32,
    spell2_id: i32,
) -> Result<bool, String> {
    let patch = LobbySlotPatch {
        slot_index,
        champion_id,
        runes: None,
        spells: Some((spell1_id, spell2_id)),
    };
    patch_lobby_slots_on_lcu(conn, &[patch]).await
}

#[cfg(test)]
#[path = "lobby_injection_tests.rs"]
mod tests;

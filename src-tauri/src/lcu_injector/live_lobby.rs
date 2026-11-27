use crate::build_provider::ChampionBuild;
use crate::lcu_connector::LcuConnection;
use crate::models::InjectionReport;

use super::live_injection::inject_runes;
use super::{
    LobbyInjectionSink, LobbySlotPatch, get_user_preferences, inject_lobby_preselect_with_sink,
    patch_lobby_player_slots_value, sync_item_sets_with_store,
};

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

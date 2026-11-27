#![allow(clippy::too_many_arguments, unused_variables)]

use crate::ipc_export::CmdResult;
use crate::{
    ActiveChampSelectSnapshot, AppLocale, AramAugmentEntry, AramChampionRank, AramMode,
    ArenaTierListResponse, CachedProfileResult, ChampionBuild, MatchHistoryPage,
    MatchScoreboardDetail, PlayerProfile, RankedTierListResponse, SummonerInfo, UserPreferences,
};

/// Tauri command to fetch the recommended build (with visual metadata) for a champion.
#[specta::specta]
pub async fn get_recommended_build_for_champion(
    champion_id: i32,
    queue_id: Option<i32>,
) -> CmdResult<ChampionBuild> {
    unreachable!()
}

/// Tauri command to update build filters (position, tier, region, patch) and re-inject if connected.
#[specta::specta]
pub async fn update_build_filters(
    champion_id: i32,
    queue_id: Option<i32>,
    position: Option<String>,
    tier: Option<String>,
    region: Option<String>,
    patch: Option<String>,
) -> CmdResult<ChampionBuild> {
    unreachable!()
}

/// Tauri command to fetch the current summoner status if the client is connected.
#[specta::specta]
pub async fn get_summoner_status() -> CmdResult<Option<SummonerInfo>> {
    unreachable!()
}

/// Tauri command to fetch the current LCU gameflow phase if the client is connected.
#[specta::specta]
pub async fn get_gameflow_phase() -> Option<String> {
    unreachable!()
}

/// Tauri command returning a consolidated bootstrap snapshot of the active LCU gameflow phase,
/// Champ Select payload, and compatible cached build (probing LCU on demand if not yet polled).
#[specta::specta]
pub async fn get_active_champ_select_snapshot() -> ActiveChampSelectSnapshot {
    unreachable!()
}

/// Tauri command to return the cached profile for the last known summoner in 0ms (even before LCU connects).
#[specta::specta]
pub async fn get_cached_player_profile(summoner_id: Option<u64>) -> Option<CachedProfileResult> {
    unreachable!()
}

/// Tauri command to fetch aggregated player profile (wallpaper, ranked stats, top 3 masteries, 10 matches).
/// When `force_refresh` is `Some(true)`, bypasses the `.lz4` disk cache.
#[specta::specta]
pub async fn get_player_profile(force_refresh: Option<bool>) -> CmdResult<Option<PlayerProfile>> {
    unreachable!()
}

/// Tauri command to fetch an incremental match history page (`beg_index..=beg_index+count-1`) from LCU.
#[specta::specta]
pub async fn get_player_match_history_page(
    beg_index: u32,
    count: Option<u32>,
) -> CmdResult<MatchHistoryPage> {
    unreachable!()
}

/// Tauri command to fetch the full on-demand scoreboard for a single match (`5v5` or `Arena`).
#[specta::specta]
pub async fn get_match_scoreboard_detail(game_id: u64) -> CmdResult<MatchScoreboardDetail> {
    unreachable!()
}

/// Syncs the champion ID currently displayed in the frontend UI to backend session state
/// so final Champ Select reconciliation can detect UI/backend divergence.
#[specta::specta]
pub async fn sync_displayed_champion(champion_id: Option<i32>) {
    unreachable!()
}

/// Switches the active Swiftplay/Quickplay pre-selection slot (Primary vs Secondary)
/// and emits the corresponding build & selection events.
#[specta::specta]
pub async fn select_lobby_preselect_slot(
    slot_index: u32,
) -> CmdResult<Option<ActiveChampSelectSnapshot>> {
    unreachable!()
}

/// Tauri command to sync user preferences from the frontend to the backend engine and `UserSettings.ini`.
#[specta::specta]
pub async fn sync_user_preferences(preferences: UserPreferences) {
    unreachable!()
}

/// Tauri command to update the active backend locale (`pt_BR` | `en_US`), persist to `UserSettings.ini`, and pre-warm DataDragon cache.
#[specta::specta]
pub async fn set_locale(locale: AppLocale) {
    unreachable!()
}

/// Tauri command to immediately inject summoner spells into an active Champ Select session.
#[specta::specta]
pub async fn inject_summoner_spells(
    spell1_id: i32,
    spell2_id: i32,
    champion_id: Option<i32>,
) -> CmdResult<()> {
    unreachable!()
}

/// Tauri command to immediately inject a specific chosen rune page into the LoL client.
#[specta::specta]
pub async fn inject_rune_page(
    champion_id: i32,
    champion_name: String,
    page_index: usize,
    primary_style_id: i32,
    sub_style_id: i32,
    selected_perk_ids: Vec<i32>,
) -> CmdResult<()> {
    unreachable!()
}

/// Tauri command to immediately update and re-inject item sets into the LoL client shop.
#[specta::specta]
pub async fn inject_item_build(
    champion_id: i32,
    queue_id: Option<i32>,
    build_index: Option<usize>,
) -> CmdResult<()> {
    unreachable!()
}

/// Tauri command to fetch ARAM or ARAM Mayhem champion tier list.
#[specta::specta]
pub async fn get_aram_tier_list(mode: AramMode) -> CmdResult<Vec<AramChampionRank>> {
    unreachable!()
}

/// Tauri command to fetch ARAM Mayhem augments for a specific champion key.
#[specta::specta]
pub async fn get_aram_mayhem_augments(champion_key: String) -> CmdResult<Vec<AramAugmentEntry>> {
    unreachable!()
}

/// Tauri command to fetch a read-only ARAM or ARAM Mayhem champion build preview
/// without emitting Champ Select events or injecting into LCU.
#[specta::specta]
pub async fn get_aram_champion_build(
    champion_id: i32,
    mode: AramMode,
    tier: Option<String>,
    region: Option<String>,
    patch: Option<String>,
) -> CmdResult<ChampionBuild> {
    unreachable!()
}

/// Tauri command to fetch Ranked Summoner's Rift tier list filtered by region, elo tier, and patch.
#[specta::specta]
pub async fn get_ranked_tier_list(
    region: Option<String>,
    tier: Option<String>,
    patch: Option<String>,
) -> CmdResult<RankedTierListResponse> {
    unreachable!()
}

/// Tauri command to fetch a read-only champion build preview for the Ranked browser tab
/// without emitting Champ Select events or injecting into LCU.
#[specta::specta]
pub async fn get_ranked_champion_build(
    champion_id: i32,
    position: Option<String>,
    tier: Option<String>,
    region: Option<String>,
    patch: Option<String>,
) -> CmdResult<ChampionBuild> {
    unreachable!()
}

/// Tauri command to fetch the Arena (`CHERRY`) champion tier list, optionally filtered by patch.
#[specta::specta]
pub async fn get_arena_tier_list(patch: Option<String>) -> CmdResult<ArenaTierListResponse> {
    unreachable!()
}

/// Tauri command to fetch a read-only Arena (`CHERRY`) champion build preview
/// (items, prismatics, augments, synergies) without injecting into LCU.
#[specta::specta]
pub async fn get_arena_champion_build(
    champion_id: i32,
    patch: Option<String>,
) -> CmdResult<ChampionBuild> {
    unreachable!()
}

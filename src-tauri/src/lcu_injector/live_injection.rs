use std::sync::atomic::Ordering;

use crate::build_provider::ChampionBuild;
use crate::lcu_connector::LcuConnection;
use crate::models::InjectionReport;

use super::{
    AppliedInjectionState, FlashKeyPreference, ItemSetsStore, SafetyInjectionSink,
    build_rune_page_payload, build_rune_page_payload_from_parts, build_spells_payload,
    find_first_deletable_page_id, find_schumaz_page_id, get_user_preferences,
    inject_champion_build_with_sink_for_queue, resolve_injection_spell_pair_from_raw_for_queue,
    safety_reinject_build_with_sink, set_selected_rune_page, sync_item_sets_with_store,
};

impl ItemSetsStore for LcuConnection {
    async fn get_item_sets(&self, summoner_id: u64) -> Result<serde_json::Value, String> {
        LcuConnection::get_item_sets(self, summoner_id).await
    }

    async fn put_item_sets(
        &self,
        summoner_id: u64,
        payload: &serde_json::Value,
    ) -> Result<(), String> {
        LcuConnection::put_item_sets(self, summoner_id, payload).await
    }
}

impl SafetyInjectionSink for LcuConnection {
    async fn inject_runes_for_build(&self, build: &ChampionBuild) -> Result<(), String> {
        inject_runes(self, build).await
    }

    async fn inject_spells_for_build(
        &self,
        build: &ChampionBuild,
        queue_id: Option<i32>,
        flash_key: FlashKeyPreference,
    ) -> Result<(), String> {
        inject_spells_for_queue(self, build, queue_id, flash_key).await
    }

    async fn inject_items_for_build(
        &self,
        summoner_id: u64,
        build: &ChampionBuild,
    ) -> Result<(), String> {
        inject_item_sets(self, summoner_id, build).await
    }
}

/// Injects rune page into the LCU. Reuses and overwrites the single schumaz.gg rune page across matches
/// instead of creating a new page every match.
pub async fn inject_rune_payload(
    conn: &LcuConnection,
    payload: &serde_json::Value,
) -> Result<(), String> {
    if let Ok(pages) = conn.get_all_rune_pages().await {
        if let Some(existing_page_id) = find_schumaz_page_id(&pages) {
            if conn.put_rune_page(existing_page_id, payload).await.is_ok() {
                return Ok(());
            }
            let _ = conn.delete_rune_page(existing_page_id).await;
            conn.create_rune_page(payload).await?;
            return Ok(());
        }

        match conn.create_rune_page(payload).await {
            Ok(_) => return Ok(()),
            Err(_) => {
                if let Some(deletable_id) = find_first_deletable_page_id(&pages) {
                    if conn.put_rune_page(deletable_id, payload).await.is_ok() {
                        return Ok(());
                    }
                    let _ = conn.delete_rune_page(deletable_id).await;
                    conn.create_rune_page(payload).await?;
                    return Ok(());
                }
            }
        }
    } else {
        conn.create_rune_page(payload).await?;
    }

    Ok(())
}

/// Injects rune page into the LCU. Reuses and overwrites the single schumaz.gg rune page across matches
/// instead of creating a new page every match.
pub async fn inject_runes(conn: &LcuConnection, build: &ChampionBuild) -> Result<(), String> {
    let payload = build_rune_page_payload(build)?;
    inject_rune_payload(conn, &payload).await
}

/// Injects a specific chosen rune page directly into the LCU and saves the user choice in ChampionSelections.
pub async fn inject_specific_rune_page(
    conn: &LcuConnection,
    champion_id: i32,
    champion_name: &str,
    page_index: usize,
    primary_style_id: i32,
    sub_style_id: i32,
    selected_perk_ids: &[i32],
) -> Result<(), String> {
    set_selected_rune_page(champion_id, page_index);
    let payload = build_rune_page_payload_from_parts(
        champion_name,
        primary_style_id,
        sub_style_id,
        selected_perk_ids,
    )?;
    inject_rune_payload(conn, &payload).await
}

/// Fast-path spell injection directly from raw spell IDs for an explicit queue ID.
pub async fn inject_spells_pair_for_queue(
    conn: &LcuConnection,
    champion_id: i32,
    queue_id: Option<i32>,
    spell1_id: i32,
    spell2_id: i32,
    is_jungle: bool,
    flash_key: FlashKeyPreference,
) -> Result<(), String> {
    let (final_s1, final_s2) = resolve_injection_spell_pair_from_raw_for_queue(
        champion_id,
        queue_id,
        spell1_id,
        spell2_id,
        is_jungle,
        flash_key,
    );
    let payload = build_spells_payload(final_s1, final_s2)?;
    conn.patch_summoner_spells(&payload).await?;
    Ok(())
}

/// Fast-path spell injection directly from raw spell IDs without requiring full build enrichment.
pub async fn inject_spells_pair(
    conn: &LcuConnection,
    champion_id: i32,
    spell1_id: i32,
    spell2_id: i32,
    is_jungle: bool,
    flash_key: FlashKeyPreference,
) -> Result<(), String> {
    inject_spells_pair_for_queue(
        conn,
        champion_id,
        None,
        spell1_id,
        spell2_id,
        is_jungle,
        flash_key,
    )
    .await
}

/// Injects summoner spells into the active Champion Select session for an explicit queue ID.
pub async fn inject_spells_for_queue(
    conn: &LcuConnection,
    build: &ChampionBuild,
    queue_id: Option<i32>,
    flash_key: FlashKeyPreference,
) -> Result<(), String> {
    let is_jungle = build.active_position.eq_ignore_ascii_case("jungle");
    let effective_q = queue_id.or_else(|| {
        let q = crate::ws_listener::shared_active_queue_id().load(Ordering::Relaxed);
        (q != 0).then_some(q)
    });
    inject_spells_pair_for_queue(
        conn,
        build.champion_id,
        effective_q,
        build.spell1_id,
        build.spell2_id,
        is_jungle,
        flash_key,
    )
    .await
}

/// Injects summoner spells into the active Champion Select session adapting to the Flash key preference
/// and mandatory Jungle Smite rules.
pub async fn inject_spells(
    conn: &LcuConnection,
    build: &ChampionBuild,
    flash_key: FlashKeyPreference,
) -> Result<(), String> {
    inject_spells_for_queue(conn, build, None, flash_key).await
}

/// Injects custom item sets for the summoner into the game client shop,
/// preserving player-created sets and skipping unchanged PUTs.
pub async fn inject_item_sets(
    conn: &LcuConnection,
    summoner_id: u64,
    build: &ChampionBuild,
) -> Result<(), String> {
    sync_item_sets_with_store(conn, summoner_id, &[build]).await?;
    Ok(())
}

/// High-level orchestrator injecting runes, summoner spells, and item sets for `queue_id`.
pub async fn inject_champion_build_with_options_for_queue(
    conn: &LcuConnection,
    summoner_id: u64,
    queue_id: Option<i32>,
    build: &ChampionBuild,
    spells_already_injected: bool,
) -> InjectionReport {
    let prefs = get_user_preferences();
    inject_champion_build_with_sink_for_queue(
        conn,
        summoner_id,
        queue_id,
        build,
        spells_already_injected,
        &prefs,
    )
    .await
}

/// High-level orchestrator injecting runes, summoner spells, and item sets concurrently while
/// respecting user preferences and optionally skipping spell patching when already fast-injected.
pub async fn inject_champion_build_with_options(
    conn: &LcuConnection,
    summoner_id: u64,
    build: &ChampionBuild,
    spells_already_injected: bool,
) -> InjectionReport {
    inject_champion_build_with_options_for_queue(
        conn,
        summoner_id,
        None,
        build,
        spells_already_injected,
    )
    .await
}

/// Performs a full safety re-injection into the live LCU before match load without skipping spells,
/// respecting mode rune/spell support and preserving manual ChampionSelections.
pub async fn safety_reinject_build(
    conn: &LcuConnection,
    summoner_id: u64,
    queue_id: i32,
    build: &ChampionBuild,
) -> (InjectionReport, AppliedInjectionState) {
    let prefs = get_user_preferences();
    safety_reinject_build_with_sink(conn, summoner_id, queue_id, build, &prefs).await
}

/// High-level orchestrator injecting runes, summoner spells, and item sets respecting user preferences.
/// Produces a comprehensive report of all injection outcomes.
pub async fn inject_champion_build(
    conn: &LcuConnection,
    summoner_id: u64,
    build: &ChampionBuild,
) -> InjectionReport {
    inject_champion_build_with_options(conn, summoner_id, build, false).await
}

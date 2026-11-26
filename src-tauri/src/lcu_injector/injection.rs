use super::injection_state::{
    AppliedInjectionState, SafetyInjectionSink, log_injection_outcome, plan_safety_injection,
    resolve_injection_spell_pair_from_raw,
};
use super::item_sets::sync_item_sets_with_store;
use super::payloads::{
    build_rune_page_payload, build_rune_page_payload_from_parts, build_spells_payload,
    find_first_deletable_page_id, find_schumaz_page_id,
};
use super::preferences::{FlashKeyPreference, UserPreferences, get_user_preferences};
use super::selections::set_selected_rune_page;
use crate::build_provider::ChampionBuild;
use crate::models::{AppLocale, InjectionReport};
use std::sync::atomic::Ordering;

pub const PERKS_CURRENT_PAGE_ENDPOINT: &str = "/lol-perks/v1/currentpage";
pub const PERKS_PAGES_ENDPOINT: &str = "/lol-perks/v1/pages";
pub const CHAMP_SELECT_MY_SELECTION_ENDPOINT: &str = "/lol-champ-select/v1/session/my-selection";

pub(crate) fn format_injection_report_message(
    locale: AppLocale,
    supports_runes: bool,
    supports_spells: bool,
    runes_res: Option<&Result<(), String>>,
    spells_res: Option<&Result<(), String>>,
    items_res: Option<&Result<(), String>>,
) -> String {
    let mut parts = Vec::new();

    if !supports_runes && !supports_spells {
        parts.push(match locale {
            AppLocale::PtBr => "Modo Arena (runas e feitiços desativados pela Riot)".to_string(),
            AppLocale::EnUs => "Arena mode (runes and spells disabled by Riot)".to_string(),
        });
    } else {
        if !supports_runes {
            parts.push(match locale {
                AppLocale::PtBr => "Modo ARAM Mayhem (runas desativadas pela Riot)".to_string(),
                AppLocale::EnUs => "ARAM Mayhem mode (runes disabled by Riot)".to_string(),
            });
        } else if let Some(r) = runes_res {
            parts.push(match (locale, r) {
                (AppLocale::PtBr, Ok(_)) => "Runas injetadas".to_string(),
                (AppLocale::PtBr, Err(e)) => format!("Falha nas Runas: {e}"),
                (AppLocale::EnUs, Ok(_)) => "Runes injected".to_string(),
                (AppLocale::EnUs, Err(e)) => format!("Runes failed: {e}"),
            });
        }

        if !supports_spells {
            parts.push(match locale {
                AppLocale::PtBr => "Feitiços desativados neste modo".to_string(),
                AppLocale::EnUs => "Spells disabled in this mode".to_string(),
            });
        } else if let Some(s) = spells_res {
            parts.push(match (locale, s) {
                (AppLocale::PtBr, Ok(_)) => "Feitiços injetados".to_string(),
                (AppLocale::PtBr, Err(e)) => format!("Falha nos Feitiços: {e}"),
                (AppLocale::EnUs, Ok(_)) => "Spells injected".to_string(),
                (AppLocale::EnUs, Err(e)) => format!("Spells failed: {e}"),
            });
        }
    }

    if let Some(i) = items_res {
        parts.push(match (locale, i) {
            (AppLocale::PtBr, Ok(_)) => "Itens injetados".to_string(),
            (AppLocale::PtBr, Err(e)) => format!("Falha nos Itens: {e}"),
            (AppLocale::EnUs, Ok(_)) => "Items injected".to_string(),
            (AppLocale::EnUs, Err(e)) => format!("Items failed: {e}"),
        });
    }

    if parts.is_empty() {
        match locale {
            AppLocale::PtBr => "Nenhuma injeção configurada nas preferências".to_string(),
            AppLocale::EnUs => "No injections enabled in preferences".to_string(),
        }
    } else {
        parts.join(" | ")
    }
}

/// Injects rune page into the LCU. Reuses and overwrites the single schumaz.gg rune page across matches
/// instead of creating a new page every match.
pub async fn inject_rune_payload(
    conn: &crate::lcu_connector::LcuConnection,
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
pub async fn inject_runes(
    conn: &crate::lcu_connector::LcuConnection,
    build: &ChampionBuild,
) -> Result<(), String> {
    let payload = build_rune_page_payload(build)?;
    inject_rune_payload(conn, &payload).await
}

/// Injects a specific chosen rune page directly into the LCU and saves the user choice in ChampionSelections.
pub async fn inject_specific_rune_page(
    conn: &crate::lcu_connector::LcuConnection,
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

/// Fast-path spell injection directly from raw spell IDs without requiring full build enrichment.
pub async fn inject_spells_pair(
    conn: &crate::lcu_connector::LcuConnection,
    champion_id: i32,
    spell1_id: i32,
    spell2_id: i32,
    is_jungle: bool,
    flash_key: FlashKeyPreference,
) -> Result<(), String> {
    let (final_s1, final_s2) = resolve_injection_spell_pair_from_raw(
        champion_id,
        spell1_id,
        spell2_id,
        is_jungle,
        flash_key,
    );
    let payload = build_spells_payload(final_s1, final_s2)?;
    conn.patch_summoner_spells(&payload).await?;
    Ok(())
}

/// Injects summoner spells into the active Champion Select session adapting to the Flash key preference
/// and mandatory Jungle Smite rules.
pub async fn inject_spells(
    conn: &crate::lcu_connector::LcuConnection,
    build: &ChampionBuild,
    flash_key: FlashKeyPreference,
) -> Result<(), String> {
    let is_jungle = build.active_position.eq_ignore_ascii_case("jungle");
    inject_spells_pair(
        conn,
        build.champion_id,
        build.spell1_id,
        build.spell2_id,
        is_jungle,
        flash_key,
    )
    .await
}

/// Injects custom item sets for the summoner into the game client shop,
/// preserving player-created sets and skipping unchanged PUTs.
pub async fn inject_item_sets(
    conn: &crate::lcu_connector::LcuConnection,
    summoner_id: u64,
    build: &ChampionBuild,
) -> Result<(), String> {
    sync_item_sets_with_store(conn, summoner_id, &[build]).await?;
    Ok(())
}

/// Executes concurrent injection of runes, summoner spells, and item sets against any
/// `SafetyInjectionSink`, respecting mode rune/spell support and explicit preferences.
pub async fn inject_champion_build_with_sink(
    sink: &impl SafetyInjectionSink,
    summoner_id: u64,
    build: &ChampionBuild,
    spells_already_injected: bool,
    prefs: &UserPreferences,
) -> InjectionReport {
    let targets = plan_safety_injection(build, prefs);

    let runes_fut = async {
        if targets.runes {
            Some(sink.inject_runes_for_build(build).await)
        } else {
            None
        }
    };

    let spells_fut = async {
        if targets.spells {
            if spells_already_injected {
                Some(Ok(()))
            } else {
                Some(sink.inject_spells_for_build(build, prefs.flash_key).await)
            }
        } else {
            None
        }
    };

    let items_fut = async {
        if targets.items {
            Some(sink.inject_items_for_build(summoner_id, build).await)
        } else {
            None
        }
    };

    let (runes_res, spells_res, items_res) = tokio::join!(runes_fut, spells_fut, items_fut);
    let active_queue_id = crate::ws_listener::shared_active_queue_id().load(Ordering::Relaxed);
    log_injection_outcome(
        build.champion_id,
        active_queue_id,
        runes_res.as_ref(),
        spells_res.as_ref(),
        items_res.as_ref(),
    );

    let runes_ok = runes_res.as_ref().is_some_and(|r| r.is_ok());
    let spells_ok = spells_res.as_ref().is_some_and(|r| r.is_ok());
    let items_ok = items_res.as_ref().is_some_and(|r| r.is_ok());

    let message = format_injection_report_message(
        prefs.language,
        build.supports_runes,
        build.supports_spells,
        runes_res.as_ref(),
        spells_res.as_ref(),
        items_res.as_ref(),
    );

    InjectionReport {
        champion_name: build.champion_name.clone(),
        runes_injected: runes_ok,
        spells_injected: spells_ok,
        items_injected: items_ok,
        message,
    }
}

/// High-level orchestrator injecting runes, summoner spells, and item sets concurrently while
/// respecting user preferences and optionally skipping spell patching when already fast-injected.
pub async fn inject_champion_build_with_options(
    conn: &crate::lcu_connector::LcuConnection,
    summoner_id: u64,
    build: &ChampionBuild,
    spells_already_injected: bool,
) -> InjectionReport {
    let prefs = get_user_preferences();
    inject_champion_build_with_sink(conn, summoner_id, build, spells_already_injected, &prefs).await
}

/// Performs a full safety re-injection before match load using an injectable sink,
/// forcing spell re-patching (`spells_already_injected = false`) while preserving manual selections.
pub async fn safety_reinject_build_with_sink(
    sink: &impl SafetyInjectionSink,
    summoner_id: u64,
    queue_id: i32,
    build: &ChampionBuild,
    prefs: &UserPreferences,
) -> (InjectionReport, AppliedInjectionState) {
    let report = inject_champion_build_with_sink(sink, summoner_id, build, false, prefs).await;
    let applied = AppliedInjectionState::from_report(queue_id, build, &report);
    (report, applied)
}

/// Performs a full safety re-injection into the live LCU before match load without skipping spells,
/// respecting mode rune/spell support (e.g. ARAM Mayhem skips runes, Arena skips runes & spells)
/// and preserving manual ChampionSelections.
pub async fn safety_reinject_build(
    conn: &crate::lcu_connector::LcuConnection,
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
    conn: &crate::lcu_connector::LcuConnection,
    summoner_id: u64,
    build: &ChampionBuild,
) -> InjectionReport {
    inject_champion_build_with_options(conn, summoner_id, build, false).await
}

#[cfg(test)]
#[path = "injection_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "injection_arena_tests.rs"]
mod arena_tests;

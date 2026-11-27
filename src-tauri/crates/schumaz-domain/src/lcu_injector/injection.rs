use std::sync::atomic::Ordering;

use super::injection_state::{
    AppliedInjectionState, SafetyInjectionSink, format_injection_report_message,
    log_injection_outcome, plan_safety_injection,
};
use super::preferences::UserPreferences;
use crate::build_provider::ChampionBuild;
use crate::models::InjectionReport;

pub const PERKS_CURRENT_PAGE_ENDPOINT: &str = "/lol-perks/v1/currentpage";
pub const PERKS_PAGES_ENDPOINT: &str = "/lol-perks/v1/pages";
pub const CHAMP_SELECT_MY_SELECTION_ENDPOINT: &str = "/lol-champ-select/v1/session/my-selection";

/// Executes concurrent injection of runes, summoner spells, and item sets against any
/// `SafetyInjectionSink` for an explicit optional `queue_id`.
pub async fn inject_champion_build_with_sink_for_queue(
    sink: &impl SafetyInjectionSink,
    summoner_id: u64,
    queue_id: Option<i32>,
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
                Some(
                    sink.inject_spells_for_build(build, queue_id, prefs.flash_key)
                        .await,
                )
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
    let logged_queue_id = queue_id
        .unwrap_or_else(|| crate::draft_session::shared_active_queue_id().load(Ordering::Relaxed));
    log_injection_outcome(
        build.champion_id,
        logged_queue_id,
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

/// Executes concurrent injection of runes, summoner spells, and item sets against any
/// `SafetyInjectionSink`, respecting mode rune/spell support and explicit preferences.
pub async fn inject_champion_build_with_sink(
    sink: &impl SafetyInjectionSink,
    summoner_id: u64,
    build: &ChampionBuild,
    spells_already_injected: bool,
    prefs: &UserPreferences,
) -> InjectionReport {
    inject_champion_build_with_sink_for_queue(
        sink,
        summoner_id,
        None,
        build,
        spells_already_injected,
        prefs,
    )
    .await
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
    let report =
        inject_champion_build_with_sink_for_queue(sink, summoner_id, Some(queue_id), build, false, prefs)
            .await;
    let applied = AppliedInjectionState::from_report(queue_id, build, &report);
    (report, applied)
}

#[cfg(test)]
#[path = "injection_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "injection_arena_tests.rs"]
mod arena_tests;

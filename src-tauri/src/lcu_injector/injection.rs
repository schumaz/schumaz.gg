use super::*;
use crate::build_provider::ChampionBuild;
pub use crate::models::InjectionReport;

pub const PERKS_CURRENT_PAGE_ENDPOINT: &str = "/lol-perks/v1/currentpage";
pub const PERKS_PAGES_ENDPOINT: &str = "/lol-perks/v1/pages";
pub const CHAMP_SELECT_MY_SELECTION_ENDPOINT: &str = "/lol-champ-select/v1/session/my-selection";

/// Resolved injection targets for a build according to game mode support and user preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InjectionTargets {
    pub runes: bool,
    pub spells: bool,
    pub items: bool,
}

/// Determines which build components should be injected for a build and user preferences.
/// Modes without rune support (e.g. ARAM Mayhem) never target runes.
pub fn plan_safety_injection(build: &ChampionBuild, prefs: &UserPreferences) -> InjectionTargets {
    InjectionTargets {
        runes: prefs.auto_inject_runes && build.supports_runes,
        spells: prefs.auto_inject_spells,
        items: prefs.auto_inject_items,
    }
}

/// Tracks the latest injection outcome actually applied to the LCU client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedInjectionState {
    pub champion_id: i32,
    pub queue_id: i32,
    pub supports_runes: bool,
    pub runes_injected: bool,
    pub spells_injected: bool,
    pub items_injected: bool,
}

impl AppliedInjectionState {
    pub fn from_report(queue_id: i32, build: &ChampionBuild, report: &InjectionReport) -> Self {
        Self {
            champion_id: build.champion_id,
            queue_id,
            supports_runes: build.supports_runes,
            runes_injected: report.runes_injected,
            spells_injected: report.spells_injected,
            items_injected: report.items_injected,
        }
    }

    /// Returns true when the applied state matches the target champion, mode rune support,
    /// and every injection category enabled in user preferences succeeded.
    pub fn matches_expected(
        &self,
        champion_id: i32,
        queue_id: i32,
        build: &ChampionBuild,
        prefs: &UserPreferences,
    ) -> bool {
        if self.champion_id != champion_id
            || self.queue_id != queue_id
            || self.supports_runes != build.supports_runes
        {
            return false;
        }
        let targets = plan_safety_injection(build, prefs);
        (!targets.runes || self.runes_injected)
            && (!targets.spells || self.spells_injected)
            && (!targets.items || self.items_injected)
    }
}

/// Abstraction over LCU injection operations so orchestration and mode support can be unit-tested.
pub trait SafetyInjectionSink: Send + Sync {
    fn inject_runes_for_build(
        &self,
        build: &ChampionBuild,
    ) -> impl std::future::Future<Output = Result<(), String>> + Send;

    fn inject_spells_for_build(
        &self,
        build: &ChampionBuild,
        flash_key: FlashKeyPreference,
    ) -> impl std::future::Future<Output = Result<(), String>> + Send;

    fn inject_items_for_build(
        &self,
        summoner_id: u64,
        build: &ChampionBuild,
    ) -> impl std::future::Future<Output = Result<(), String>> + Send;
}

impl SafetyInjectionSink for crate::lcu_connector::LcuConnection {
    async fn inject_runes_for_build(&self, build: &ChampionBuild) -> Result<(), String> {
        inject_runes(self, build).await
    }

    async fn inject_spells_for_build(
        &self,
        build: &ChampionBuild,
        flash_key: FlashKeyPreference,
    ) -> Result<(), String> {
        inject_spells(self, build, flash_key).await
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
    conn: &crate::lcu_connector::LcuConnection,
    payload: &serde_json::Value,
) -> Result<(), String> {
    // 1. Check all rune pages to see if a schumaz.gg page already exists
    if let Ok(pages) = conn.get_all_rune_pages().await {
        if let Some(existing_page_id) = find_schumaz_page_id(&pages) {
            // Overwrite the existing page in-place via PUT
            if conn.put_rune_page(existing_page_id, payload).await.is_ok() {
                return Ok(());
            }
            // If PUT fails, delete the existing schumaz page and recreate
            let _ = conn.delete_rune_page(existing_page_id).await;
            conn.create_rune_page(payload).await?;
            return Ok(());
        }

        // 2. No schumaz.gg page exists yet. Try creating a new one.
        match conn.create_rune_page(payload).await {
            Ok(_) => return Ok(()),
            Err(_) => {
                // If creation failed (e.g. max page limit reached), overwrite or delete the first custom page
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
        // Fallback: direct create if pages endpoint query failed
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

/// Resolves the final `(spell1Id, spell2Id)` pair to inject for a champion, applying
/// user secondary spell selections, mandatory Jungle Smite rules, and Flash key preference (D vs F)
/// across all game modes (Summoner's Rift, ARAM, ARAM Mayhem, Practice Tool).
pub fn resolve_injection_spell_pair_from_raw(
    champion_id: i32,
    spell1_id: i32,
    spell2_id: i32,
    is_jungle: bool,
    flash_key: FlashKeyPreference,
) -> (i32, i32) {
    let (mut s1, mut s2) = (spell1_id, spell2_id);

    if let Some(sel) = get_champion_selections(champion_id)
        && let Some(sec_spell) = sel.secondary_spell_id
    {
        if is_jungle {
            if s1 == SPELL_SMITE_ID {
                s2 = sec_spell;
            } else if s2 == SPELL_SMITE_ID {
                s1 = sec_spell;
            } else {
                s2 = sec_spell;
            }
        } else if s1 == SPELL_FLASH_ID {
            s2 = sec_spell;
        } else if s2 == SPELL_FLASH_ID {
            s1 = sec_spell;
        } else {
            s2 = sec_spell;
        }
    }

    apply_jungle_smite_and_flash_preference(s1, s2, flash_key, is_jungle)
}

/// Resolves the final `(spell1Id, spell2Id)` pair to inject from a `ChampionBuild`.
pub fn resolve_injection_spell_pair(
    build: &ChampionBuild,
    flash_key: FlashKeyPreference,
) -> (i32, i32) {
    let is_jungle = build.active_position.eq_ignore_ascii_case("jungle");
    resolve_injection_spell_pair_from_raw(
        build.champion_id,
        build.spell1_id,
        build.spell2_id,
        is_jungle,
        flash_key,
    )
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

/// Injects custom item sets for the summoner into the game client shop.
pub async fn inject_item_sets(
    conn: &crate::lcu_connector::LcuConnection,
    summoner_id: u64,
    build: &ChampionBuild,
) -> Result<(), String> {
    let payload = build_item_sets_payload(summoner_id, build);
    conn.put_item_sets(summoner_id, &payload).await?;
    Ok(())
}

/// Executes concurrent injection of runes, summoner spells, and item sets against any
/// `SafetyInjectionSink`, respecting mode rune support and explicit preferences.
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

    let runes_ok = runes_res.as_ref().is_some_and(|r| r.is_ok());
    let spells_ok = spells_res.as_ref().is_some_and(|r| r.is_ok());
    let items_ok = items_res.as_ref().is_some_and(|r| r.is_ok());

    let mut parts = Vec::new();
    if !build.supports_runes {
        parts.push("Modo ARAM Mayhem (runas desativadas pela Riot)".to_string());
    } else if let Some(r) = runes_res {
        match r {
            Ok(_) => parts.push("Runas injetadas".to_string()),
            Err(e) => parts.push(format!("Falha nas Runas: {}", e)),
        }
    }

    if let Some(s) = spells_res {
        match s {
            Ok(_) => parts.push("Feitiços injetados".to_string()),
            Err(e) => parts.push(format!("Falha nos Feitiços: {}", e)),
        }
    }
    if let Some(i) = items_res {
        match i {
            Ok(_) => parts.push("Itens injetados".to_string()),
            Err(e) => parts.push(format!("Falha nos Itens: {}", e)),
        }
    }

    let message = if parts.is_empty() {
        "Nenhuma injeção configurada nas preferências".to_string()
    } else {
        parts.join(" | ")
    };

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
/// respecting mode rune support (e.g. ARAM Mayhem skips runes) and preserving manual ChampionSelections.
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

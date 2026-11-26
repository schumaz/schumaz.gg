use super::preferences::{FlashKeyPreference, UserPreferences};
use super::selections::get_selected_spell_pair_for_queue;
use super::spell_rules::apply_jungle_smite_and_flash_preference;
use crate::build_provider::ChampionBuild;
use crate::logging;
use crate::models::AppLocale;
pub use crate::models::InjectionReport;

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

fn injection_outcome_tag(res: Option<&Result<(), String>>) -> &'static str {
    match res {
        Some(Ok(())) => "ok",
        Some(Err(_)) => "err",
        None => "skip",
    }
}

pub(crate) fn log_injection_outcome(
    champ_id: i32,
    queue_id: i32,
    runes_res: Option<&Result<(), String>>,
    spells_res: Option<&Result<(), String>>,
    items_res: Option<&Result<(), String>>,
) {
    let runes = injection_outcome_tag(runes_res);
    let spells = injection_outcome_tag(spells_res);
    let items = injection_outcome_tag(items_res);
    let first_err = [runes_res, spells_res, items_res]
        .into_iter()
        .flatten()
        .find_map(|res| res.as_ref().err());

    if let Some(err) = first_err {
        logging::log_warn(
            "lcu.injector",
            format!(
                "event=inject_result champ_id={champ_id} queue_id={queue_id} runes={runes} spells={spells} items={items} err={err}"
            ),
        );
    } else {
        logging::log_info(
            "lcu.injector",
            format!(
                "event=inject_result champ_id={champ_id} queue_id={queue_id} runes={runes} spells={spells} items={items}"
            ),
        );
    }
}

/// Resolved injection targets for a build according to game mode support and user preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InjectionTargets {
    pub runes: bool,
    pub spells: bool,
    pub items: bool,
}

/// Determines which build components should be injected for a build and user preferences.
/// Modes without rune/spell support (e.g. ARAM Mayhem, Arena) never target disabled categories.
pub fn plan_safety_injection(build: &ChampionBuild, prefs: &UserPreferences) -> InjectionTargets {
    InjectionTargets {
        runes: prefs.auto_inject_runes && build.supports_runes,
        spells: prefs.auto_inject_spells && build.supports_spells,
        items: prefs.auto_inject_items,
    }
}

/// Tracks the latest injection outcome actually applied to the LCU client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedInjectionState {
    pub champion_id: i32,
    pub queue_id: i32,
    pub supports_runes: bool,
    pub supports_spells: bool,
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
            supports_spells: build.supports_spells,
            runes_injected: report.runes_injected,
            spells_injected: report.spells_injected,
            items_injected: report.items_injected,
        }
    }

    /// Returns true when the applied state matches the target champion, mode rune/spell support,
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
            || self.supports_spells != build.supports_spells
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
        queue_id: Option<i32>,
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
        super::injection::inject_runes(self, build).await
    }

    async fn inject_spells_for_build(
        &self,
        build: &ChampionBuild,
        queue_id: Option<i32>,
        flash_key: FlashKeyPreference,
    ) -> Result<(), String> {
        super::injection::inject_spells_for_queue(self, build, queue_id, flash_key).await
    }

    async fn inject_items_for_build(
        &self,
        summoner_id: u64,
        build: &ChampionBuild,
    ) -> Result<(), String> {
        super::injection::inject_item_sets(self, summoner_id, build).await
    }
}

fn infer_build_queue_id(build: &ChampionBuild) -> Option<i32> {
    if !build.supports_spells {
        return Some(crate::models::QUEUE_ARENA);
    }
    if !build.supports_runes {
        return Some(crate::models::QUEUE_ARAM_MAYHEM);
    }
    let has_aram_spell = matches!(build.spell1_id, 13 | 32)
        || matches!(build.spell2_id, 13 | 32)
        || build
            .spell_options
            .iter()
            .any(|o| matches!(o.spell1_id, 13 | 32) || matches!(o.spell2_id, 13 | 32));
    if has_aram_spell {
        return Some(crate::models::QUEUE_ARAM);
    }
    if !build.active_position.is_empty()
        || matches!(build.spell1_id, 11 | 12)
        || matches!(build.spell2_id, 11 | 12)
    {
        return Some(crate::models::QUEUE_SOLO_DUO);
    }
    None
}

/// Resolves the final `(spell1Id, spell2Id)` pair to inject for a champion against an explicit or
/// active queue, validating any manual selection against mode rules before applying Jungle Smite and D/F rules.
pub fn resolve_injection_spell_pair_from_raw_for_queue(
    champion_id: i32,
    queue_id: Option<i32>,
    spell1_id: i32,
    spell2_id: i32,
    is_jungle: bool,
    flash_key: FlashKeyPreference,
) -> (i32, i32) {
    let effective_q = queue_id.filter(|&q| q != 0).or_else(|| {
        let q = crate::ws_listener::shared_active_queue_id()
            .load(std::sync::atomic::Ordering::Relaxed);
        (q != 0).then_some(q)
    });
    let (s1, s2) = get_selected_spell_pair_for_queue(champion_id, effective_q)
        .unwrap_or((spell1_id, spell2_id));

    apply_jungle_smite_and_flash_preference(s1, s2, flash_key, is_jungle)
}

/// Resolves the final `(spell1Id, spell2Id)` pair to inject for a champion using the active queue if present.
pub fn resolve_injection_spell_pair_from_raw(
    champion_id: i32,
    spell1_id: i32,
    spell2_id: i32,
    is_jungle: bool,
    flash_key: FlashKeyPreference,
) -> (i32, i32) {
    resolve_injection_spell_pair_from_raw_for_queue(
        champion_id,
        None,
        spell1_id,
        spell2_id,
        is_jungle,
        flash_key,
    )
}

/// Resolves the final `(spell1Id, spell2Id)` pair to inject from a `ChampionBuild` for `queue_id`.
pub fn resolve_injection_spell_pair_for_queue(
    build: &ChampionBuild,
    queue_id: Option<i32>,
    flash_key: FlashKeyPreference,
) -> (i32, i32) {
    let effective_q = queue_id
        .filter(|&q| q != 0)
        .or_else(|| {
            let q = crate::ws_listener::shared_active_queue_id()
                .load(std::sync::atomic::Ordering::Relaxed);
            (q != 0).then_some(q)
        })
        .or_else(|| infer_build_queue_id(build));
    let is_jungle = build.active_position.eq_ignore_ascii_case("jungle");
    resolve_injection_spell_pair_from_raw_for_queue(
        build.champion_id,
        effective_q,
        build.spell1_id,
        build.spell2_id,
        is_jungle,
        flash_key,
    )
}

/// Resolves the final `(spell1Id, spell2Id)` pair to inject from a `ChampionBuild`.
pub fn resolve_injection_spell_pair(
    build: &ChampionBuild,
    flash_key: FlashKeyPreference,
) -> (i32, i32) {
    resolve_injection_spell_pair_for_queue(build, None, flash_key)
}

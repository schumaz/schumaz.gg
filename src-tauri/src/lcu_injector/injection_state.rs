use super::preferences::{FlashKeyPreference, UserPreferences};
use super::selections::get_champion_selections;
use super::spell_rules::{SPELL_FLASH_ID, SPELL_SMITE_ID, apply_jungle_smite_and_flash_preference};
use crate::build_provider::ChampionBuild;
use crate::logging;
pub use crate::models::InjectionReport;

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
        flash_key: FlashKeyPreference,
    ) -> Result<(), String> {
        super::injection::inject_spells(self, build, flash_key).await
    }

    async fn inject_items_for_build(
        &self,
        summoner_id: u64,
        build: &ChampionBuild,
    ) -> Result<(), String> {
        super::injection::inject_item_sets(self, summoner_id, build).await
    }
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

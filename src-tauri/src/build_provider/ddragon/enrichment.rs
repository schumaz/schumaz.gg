use super::super::analytics::aram::normalize_opgg_champion_key;
use super::super::models::{ChampionBuild, DataDragonCache, ItemBlockKind, ItemMetadata};
use super::super::spells::{
    SPELL_CLARITY_ID, SPELL_MARK_ID, ddragon_mode_for_queue, sanitize_spell_choices_for_mode,
    sanitize_spell_pair_for_mode,
};
use super::metadata::{resolve_rune_metadata_for_locale, resolve_spell_metadata_for_locale};
use crate::lcu_injector::FlashKeyPreference;
use crate::models::{AppLocale, is_arena_queue};
use std::collections::HashMap;

fn infer_mode_from_build(build: &ChampionBuild) -> Option<&'static str> {
    if !build.supports_spells {
        Some("CHERRY")
    } else if !build.active_position.trim().is_empty() {
        Some("CLASSIC")
    } else if !build.supports_runes {
        Some("KIWI")
    } else if build.spell1_id == SPELL_MARK_ID
        || build.spell2_id == SPELL_MARK_ID
        || build.spell1_id == SPELL_CLARITY_ID
        || build.spell2_id == SPELL_CLARITY_ID
    {
        Some("ARAM")
    } else {
        None
    }
}

/// Translates item block titles and build option labels according to `locale`.
pub fn localize_build_labels(build: &mut ChampionBuild, locale: AppLocale) {
    for block in &mut build.item_blocks {
        if block.kind == ItemBlockKind::Other {
            block.kind = ItemBlockKind::infer_from_title(&block.title);
        }
        block.title = match (locale, block.kind) {
            (AppLocale::EnUs, ItemBlockKind::Starter) => "Starter Items".to_string(),
            (AppLocale::EnUs, ItemBlockKind::Prismatic) => "Prismatic Items".to_string(),
            (AppLocale::EnUs, ItemBlockKind::Core) => "First 3 Items".to_string(),
            (AppLocale::EnUs, ItemBlockKind::Boots) => "Top 3 Boots".to_string(),
            (AppLocale::EnUs, ItemBlockKind::Final) => "Final Build".to_string(),
            (AppLocale::EnUs, ItemBlockKind::Situational) => {
                "Situational / Utility Items".to_string()
            }
            (AppLocale::PtBr, ItemBlockKind::Starter) => "Itens Iniciais".to_string(),
            (AppLocale::PtBr, ItemBlockKind::Prismatic) => "Itens Prismáticos".to_string(),
            (AppLocale::PtBr, ItemBlockKind::Core) => "Primeiros 3 Itens".to_string(),
            (AppLocale::PtBr, ItemBlockKind::Boots) => "3 Botas Mais Feitas".to_string(),
            (AppLocale::PtBr, ItemBlockKind::Final) => "Build Final".to_string(),
            (AppLocale::PtBr, ItemBlockKind::Situational) => {
                "Itens Situacionais / Úteis".to_string()
            }
            (_, ItemBlockKind::Other) => block.title.clone(),
        };
    }
    if locale == AppLocale::EnUs {
        for opt in &mut build.build_options {
            if opt.name == "Build 1 (Mais Feita)" {
                opt.name = "Build 1 (Most Popular)".to_string();
            }
        }
    }
}

/// Sanitizes spells for the active queue mode, applies Flash key preference (D vs F) and Jungle
/// Smite rule, and enriches `spells` and `spell_choices` using DataDragon cache and `locale`.
pub fn apply_build_spells_for_queue_with_preference_and_locale(
    build: &mut ChampionBuild,
    cache: &DataDragonCache,
    flash_key: FlashKeyPreference,
    locale: AppLocale,
    queue_id: Option<i32>,
) {
    if !build.supports_spells || queue_id.is_some_and(is_arena_queue) {
        build.spell1_id = 0;
        build.spell2_id = 0;
        build.spells.clear();
        build.spell_choices.clear();
        return;
    }

    let is_jungle = build.active_position.eq_ignore_ascii_case("jungle");
    let mode = queue_id
        .map(ddragon_mode_for_queue)
        .or_else(|| infer_mode_from_build(build));
    let existing_choice_ids: Vec<i32> = build.spell_choices.iter().map(|s| s.id).collect();

    let (clean_s1, clean_s2) = sanitize_spell_pair_for_mode(
        build.spell1_id,
        build.spell2_id,
        &existing_choice_ids,
        is_jungle,
        mode,
        Some(&cache.spells),
    );
    let (s1, s2) = crate::lcu_injector::apply_jungle_smite_and_flash_preference(
        clean_s1, clean_s2, flash_key, is_jungle,
    );
    build.spell1_id = s1;
    build.spell2_id = s2;

    build.spells = vec![
        resolve_spell_metadata_for_locale(cache, build.spell1_id, locale),
        resolve_spell_metadata_for_locale(cache, build.spell2_id, locale),
    ];

    let sanitized_choices = sanitize_spell_choices_for_mode(
        build.spell1_id,
        build.spell2_id,
        &existing_choice_ids,
        is_jungle,
        mode,
        Some(&cache.spells),
    );
    build.spell_choices = sanitized_choices
        .into_iter()
        .map(|id| resolve_spell_metadata_for_locale(cache, id, locale))
        .collect();
}

/// Applies Flash key preference (D vs F) and Jungle Smite rule to a ChampionBuild's spell slots
/// and enriches `spells` and `spell_choices` using DataDragon cache and `locale`.
pub fn apply_build_spells_with_preference_and_locale(
    build: &mut ChampionBuild,
    cache: &DataDragonCache,
    flash_key: FlashKeyPreference,
    locale: AppLocale,
) {
    apply_build_spells_for_queue_with_preference_and_locale(build, cache, flash_key, locale, None);
}

/// Applies Flash key preference (D vs F) and Jungle Smite rule to a ChampionBuild's spell slots.
pub fn apply_build_spells_with_preference(
    build: &mut ChampionBuild,
    cache: &DataDragonCache,
    flash_key: FlashKeyPreference,
) {
    apply_build_spells_with_preference_and_locale(build, cache, flash_key, AppLocale::PtBr);
}

fn item_icon_urls(version: &str, item_ids: &[i32]) -> Vec<String> {
    item_ids
        .iter()
        .map(|id| {
            format!(
                "https://ddragon.leagueoflegends.com/cdn/{}/img/item/{}.png",
                version, id
            )
        })
        .collect()
}

fn collect_build_item_metadata(
    build: &ChampionBuild,
    items_catalog: &HashMap<i32, ItemMetadata>,
) -> HashMap<i32, ItemMetadata> {
    let mut resolved = HashMap::new();
    let mut insert_ids = |ids: &[i32]| {
        for &id in ids {
            if id > 0
                && !resolved.contains_key(&id)
                && let Some(meta) = items_catalog.get(&id)
            {
                resolved.insert(id, meta.clone());
            }
        }
    };
    for block in &build.item_blocks {
        insert_ids(&block.item_ids);
    }
    for opt in &build.build_options {
        insert_ids(&opt.core_item_ids);
        insert_ids(&opt.final_item_ids);
        insert_ids(&opt.situational_item_ids);
    }
    resolved
}

/// Enriches a ChampionBuild with visual assets from DataDragon using explicit queue mode,
/// `FlashKeyPreference`, and `AppLocale`.
pub fn enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
    build: &mut ChampionBuild,
    cache: &DataDragonCache,
    flash_key: FlashKeyPreference,
    locale: AppLocale,
    queue_id: Option<i32>,
) {
    if let Some(champ) = cache.champions.get(&build.champion_id) {
        build.champion_icon_url = champ.icon_url.clone();
        if build.champion_name.is_empty() || build.champion_name == "Unknown" {
            build.champion_name = champ.display_name.clone();
        }
    } else if !build.champion_name.is_empty() {
        build.champion_icon_url = format!(
            "https://ddragon.leagueoflegends.com/cdn/{}/img/champion/{}.png",
            cache.version, build.champion_name
        );
    }

    if let Some(primary) = cache.runes.get(&build.primary_style_id) {
        build.primary_style_name = primary.name.clone();
        build.primary_style_icon_url = primary.icon_url.clone();
    }
    if let Some(sub) = cache.runes.get(&build.sub_style_id) {
        build.sub_style_name = sub.name.clone();
        build.sub_style_icon_url = sub.icon_url.clone();
    }

    build.perks = build
        .selected_perk_ids
        .iter()
        .map(|id| resolve_rune_metadata_for_locale(&cache.runes, *id, locale))
        .collect();

    for page in &mut build.rune_pages {
        if let Some(primary) = cache.runes.get(&page.primary_style_id) {
            page.primary_style_name = primary.name.clone();
            page.primary_style_icon_url = primary.icon_url.clone();
        }
        if let Some(sub) = cache.runes.get(&page.sub_style_id) {
            page.sub_style_name = sub.name.clone();
            page.sub_style_icon_url = sub.icon_url.clone();
        }
        page.perks = page
            .selected_perk_ids
            .iter()
            .map(|id| resolve_rune_metadata_for_locale(&cache.runes, *id, locale))
            .collect();
    }

    apply_build_spells_for_queue_with_preference_and_locale(
        build, cache, flash_key, locale, queue_id,
    );

    for block in &mut build.item_blocks {
        block.item_icon_urls = item_icon_urls(&cache.version, &block.item_ids);
    }

    for opt in &mut build.build_options {
        opt.core_item_icon_urls = item_icon_urls(&cache.version, &opt.core_item_ids);
        opt.final_item_icon_urls = item_icon_urls(&cache.version, &opt.final_item_ids);
        opt.situational_item_icon_urls = item_icon_urls(&cache.version, &opt.situational_item_ids);
    }

    if !cache.items.is_empty() {
        build.item_metadata = collect_build_item_metadata(build, &cache.items);
    }

    for aug in &mut build.arena_augments {
        if let Some(meta) = cache.arena_augments.get(&aug.id) {
            aug.name = meta.name.clone();
            if !meta.desc.is_empty() {
                aug.desc = meta.desc.clone();
            }
            if !meta.icon_url.is_empty() {
                aug.icon_url = meta.icon_url.clone();
            }
            if aug.rarity == 0 && meta.rarity > 0 {
                aug.rarity = meta.rarity;
            }
        } else if aug.name.is_empty() {
            aug.name = match locale {
                AppLocale::PtBr => format!("Aprimoramento {}", aug.id),
                AppLocale::EnUs => format!("Augment {}", aug.id),
            };
        }
    }

    for syn in &mut build.arena_synergies {
        if let Some(meta) = cache.champions.get(&syn.champion_id) {
            syn.name = meta.display_name.clone();
            syn.key = normalize_opgg_champion_key(&meta.key_name);
            if !meta.icon_url.is_empty() {
                syn.icon_url = meta.icon_url.clone();
            }
        } else if syn.name.is_empty() {
            syn.name = format!("Champion {}", syn.champion_id);
        }
    }

    super::super::skills::enrich_skill_order_with_ddragon(build, cache);
    localize_build_labels(build, locale);
}

/// Enriches a ChampionBuild with visual assets from DataDragon using explicit `FlashKeyPreference` and `AppLocale`.
pub fn enrich_build_with_ddragon_with_flash_key_and_locale(
    build: &mut ChampionBuild,
    cache: &DataDragonCache,
    flash_key: FlashKeyPreference,
    locale: AppLocale,
) {
    enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
        build, cache, flash_key, locale, None,
    );
}

/// Enriches a ChampionBuild with visual assets from DataDragon (avatars, icons, spells, items)
/// using an explicit `FlashKeyPreference` (PT-BR default for backwards compatibility in tests).
pub fn enrich_build_with_ddragon_with_flash_key(
    build: &mut ChampionBuild,
    cache: &DataDragonCache,
    flash_key: FlashKeyPreference,
) {
    enrich_build_with_ddragon_with_flash_key_and_locale(build, cache, flash_key, AppLocale::PtBr);
}

/// Enriches a ChampionBuild with visual assets from DataDragon using the active user preferences,
/// locale, and queue-specific spell filtering.
pub fn enrich_build_with_ddragon_for_queue(
    build: &mut ChampionBuild,
    cache: &DataDragonCache,
    queue_id: i32,
) {
    let prefs = crate::lcu_injector::get_user_preferences();
    enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
        build,
        cache,
        prefs.flash_key,
        prefs.language,
        Some(queue_id),
    );
}

/// Enriches a ChampionBuild with visual assets from DataDragon using the active user preferences & locale.
pub fn enrich_build_with_ddragon(build: &mut ChampionBuild, cache: &DataDragonCache) {
    let prefs = crate::lcu_injector::get_user_preferences();
    enrich_build_with_ddragon_with_flash_key_and_locale(
        build,
        cache,
        prefs.flash_key,
        prefs.language,
    );
}

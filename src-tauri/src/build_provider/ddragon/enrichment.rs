use super::super::models::{ChampionBuild, DataDragonCache};
use super::metadata::{resolve_rune_metadata_for_locale, resolve_spell_metadata_for_locale};
use crate::lcu_injector::FlashKeyPreference;
use crate::models::AppLocale;

/// Translates item block titles and build option labels according to `locale`.
pub fn localize_build_labels(build: &mut ChampionBuild, locale: AppLocale) {
    if locale != AppLocale::EnUs {
        return;
    }
    for block in &mut build.item_blocks {
        block.title = match block.title.as_str() {
            "Itens Iniciais" => "Starter Items".to_string(),
            "Primeiros 3 Itens" => "First 3 Items".to_string(),
            "3 Botas Mais Feitas" => "Top 3 Boots".to_string(),
            "Build Final" => "Final Build".to_string(),
            "Itens Situacionais / Úteis" => "Situational / Utility Items".to_string(),
            _ => block.title.clone(),
        };
    }
    for opt in &mut build.build_options {
        if opt.name == "Build 1 (Mais Feita)" {
            opt.name = "Build 1 (Most Popular)".to_string();
        }
    }
}

/// Applies Flash key preference (D vs F) and Jungle Smite rule to a ChampionBuild's spell slots
/// and enriches `spells` and `spell_choices` using DataDragon cache and `locale`.
pub fn apply_build_spells_with_preference_and_locale(
    build: &mut ChampionBuild,
    cache: &DataDragonCache,
    flash_key: FlashKeyPreference,
    locale: AppLocale,
) {
    let is_jungle = build.active_position.eq_ignore_ascii_case("jungle");
    let (s1, s2) = crate::lcu_injector::apply_jungle_smite_and_flash_preference(
        build.spell1_id,
        build.spell2_id,
        flash_key,
        is_jungle,
    );
    build.spell1_id = s1;
    build.spell2_id = s2;

    build.spells = vec![
        resolve_spell_metadata_for_locale(cache, build.spell1_id, locale),
        resolve_spell_metadata_for_locale(cache, build.spell2_id, locale),
    ];

    if build.spell_choices.is_empty() {
        let non_primary = if is_jungle {
            if build.spell1_id == 11 {
                build.spell2_id
            } else {
                build.spell1_id
            }
        } else if build.spell1_id == 4 {
            build.spell2_id
        } else {
            build.spell1_id
        };
        let second_alt = if is_jungle {
            if non_primary == 4 { 6 } else { 4 }
        } else if non_primary == 21 {
            3
        } else if non_primary == 7 {
            21
        } else if non_primary == 32 {
            7
        } else if non_primary == 14 {
            12
        } else {
            14
        };
        build.spell_choices = vec![
            resolve_spell_metadata_for_locale(cache, non_primary, locale),
            resolve_spell_metadata_for_locale(cache, second_alt, locale),
        ];
    } else {
        build.spell_choices = build
            .spell_choices
            .iter()
            .map(|s| resolve_spell_metadata_for_locale(cache, s.id, locale))
            .collect();
    }
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

/// Enriches a ChampionBuild with visual assets from DataDragon using explicit `FlashKeyPreference` and `AppLocale`.
pub fn enrich_build_with_ddragon_with_flash_key_and_locale(
    build: &mut ChampionBuild,
    cache: &DataDragonCache,
    flash_key: FlashKeyPreference,
    locale: AppLocale,
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

    apply_build_spells_with_preference_and_locale(build, cache, flash_key, locale);

    for block in &mut build.item_blocks {
        block.item_icon_urls = item_icon_urls(&cache.version, &block.item_ids);
    }

    for opt in &mut build.build_options {
        opt.core_item_icon_urls = item_icon_urls(&cache.version, &opt.core_item_ids);
        opt.final_item_icon_urls = item_icon_urls(&cache.version, &opt.final_item_ids);
        opt.situational_item_icon_urls = item_icon_urls(&cache.version, &opt.situational_item_ids);
    }

    localize_build_labels(build, locale);
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

#[cfg(test)]
#[path = "enrichment_tests.rs"]
mod tests;

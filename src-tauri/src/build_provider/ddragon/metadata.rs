use super::super::models::{DataDragonCache, RuneMetadata, SpellMetadata};
use crate::models::AppLocale;
use std::collections::HashMap;

/// Returns metadata for a known stat shard ID localized for `locale`.
pub fn get_stat_shard_metadata_for_locale(id: i32, locale: AppLocale) -> Option<RuneMetadata> {
    let (pt_name, en_name, icon) = match id {
        5008 => (
            "Força Adaptativa",
            "Adaptive Force",
            "perk-images/StatMods/StatModsAdaptiveForceIcon.png",
        ),
        5005 => (
            "Velocidade de Ataque",
            "Attack Speed",
            "perk-images/StatMods/StatModsAttackSpeedIcon.png",
        ),
        5007 => (
            "Aceleração de Habilidade",
            "Ability Haste",
            "perk-images/StatMods/StatModsCDRScalingIcon.png",
        ),
        5002 => (
            "Armadura",
            "Armor",
            "perk-images/StatMods/StatModsArmorIcon.png",
        ),
        5003 => (
            "Resistência Mágica",
            "Magic Resist",
            "perk-images/StatMods/StatModsMagicResIcon.png",
        ),
        5001 | 5013 => (
            "Vida Escalonável",
            "Health Scaling",
            "perk-images/StatMods/StatModsHealthScalingIcon.png",
        ),
        5010 => (
            "Velocidade de Movimento",
            "Move Speed",
            "perk-images/StatMods/StatModsMovementSpeedIcon.png",
        ),
        5011 => (
            "Vida",
            "Health",
            "perk-images/StatMods/StatModsHealthPlusIcon.png",
        ),
        5012 => (
            "Tenacidade",
            "Tenacity and Slow Resist",
            "perk-images/StatMods/StatModsTenacityIcon.png",
        ),
        _ => return None,
    };
    let name = match locale {
        AppLocale::PtBr => pt_name,
        AppLocale::EnUs => en_name,
    };
    Some(RuneMetadata {
        id,
        key: format!("StatShard_{}", id),
        name: name.to_string(),
        icon_url: format!("https://ddragon.leagueoflegends.com/cdn/img/{}", icon),
    })
}

/// Returns metadata for a known stat shard ID if valid (PT-BR default fallback).
pub fn get_stat_shard_metadata(id: i32) -> Option<RuneMetadata> {
    get_stat_shard_metadata_for_locale(id, AppLocale::PtBr)
}

/// Resolves rune or shard metadata from the DataDragon runes cache or shard catalog for `locale`.
pub fn resolve_rune_metadata_for_locale(
    runes_map: &HashMap<i32, RuneMetadata>,
    perk_id: i32,
    locale: AppLocale,
) -> RuneMetadata {
    if let Some(meta) = runes_map.get(&perk_id) {
        return meta.clone();
    }
    if let Some(shard) = get_stat_shard_metadata_for_locale(perk_id, locale) {
        return shard;
    }
    let fallback_name = match locale {
        AppLocale::PtBr => format!("Runa {}", perk_id),
        AppLocale::EnUs => format!("Rune {}", perk_id),
    };
    RuneMetadata {
        id: perk_id,
        key: format!("perk_{}", perk_id),
        name: fallback_name,
        icon_url: String::new(),
    }
}

/// Resolves rune or shard metadata from the DataDragon runes cache or shard catalog.
pub fn resolve_rune_metadata(runes_map: &HashMap<i32, RuneMetadata>, perk_id: i32) -> RuneMetadata {
    resolve_rune_metadata_for_locale(runes_map, perk_id, AppLocale::PtBr)
}

/// Resolves summoner spell metadata and official Riot CDN icon localized for `locale`.
pub fn get_spell_metadata_for_locale(
    version: &str,
    spell_id: i32,
    locale: AppLocale,
) -> SpellMetadata {
    let (pt_name, en_name, icon_name) = match spell_id {
        4 => ("Flash", "Flash", "SummonerFlash"),
        14 => ("Incendiar", "Ignite", "SummonerDot"),
        12 => ("Teleporte", "Teleport", "SummonerTeleport"),
        11 => ("Golpear", "Smite", "SummonerSmite"),
        7 => ("Curar", "Heal", "SummonerHeal"),
        21 => ("Barreira", "Barrier", "SummonerBarrier"),
        3 => ("Exaustão", "Exhaust", "SummonerExhaust"),
        6 => ("Fantasma", "Ghost", "SummonerHaste"),
        1 => ("Purificar", "Cleanse", "SummonerBoost"),
        13 => ("Clareza", "Clarity", "SummonerMana"),
        32 => ("Marca", "Mark", "SummonerSnowball"),
        _ => ("Feitiço", "Spell", "SummonerFlash"),
    };
    let name = match locale {
        AppLocale::PtBr => pt_name,
        AppLocale::EnUs => en_name,
    };
    SpellMetadata {
        id: spell_id,
        name: name.to_string(),
        icon_url: format!(
            "https://ddragon.leagueoflegends.com/cdn/{}/img/spell/{}.png",
            version, icon_name
        ),
    }
}

/// Resolves summoner spell metadata and official Riot CDN icon (official PT-BR fallback).
pub fn get_spell_metadata(version: &str, spell_id: i32) -> SpellMetadata {
    get_spell_metadata_for_locale(version, spell_id, AppLocale::PtBr)
}

/// Resolves summoner spell metadata preferring DataDragon `{locale}/summoner.json` when available.
pub fn resolve_spell_metadata_for_locale(
    cache: &DataDragonCache,
    spell_id: i32,
    locale: AppLocale,
) -> SpellMetadata {
    if let Some(meta) = cache.spells.get(&spell_id) {
        return meta.clone();
    }
    get_spell_metadata_for_locale(&cache.version, spell_id, locale)
}

/// Resolves summoner spell metadata preferring DataDragon `summoner.json` when available.
pub fn resolve_spell_metadata(cache: &DataDragonCache, spell_id: i32) -> SpellMetadata {
    resolve_spell_metadata_for_locale(cache, spell_id, AppLocale::PtBr)
}

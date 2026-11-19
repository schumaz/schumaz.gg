use super::super::models::{DataDragonCache, RuneMetadata, SpellMetadata};
use super::super::spells::default_spell_modes;
use crate::models::AppLocale;
use std::collections::HashMap;
use std::path::Path;

/// Scans `dir` for cached DataDragon envelopes (`metadata_*.lz4` or `skins_*.lz4`)
/// and returns the most recently stored non-empty DataDragon version on disk.
pub fn latest_cached_ddragon_version_from_dir(dir: &Path) -> Option<String> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut best: Option<(u64, String)> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with(".lz4")
            || (!name.starts_with("metadata_") && !name.starts_with("skins_"))
        {
            continue;
        }
        let Some((created_at, patch)) = crate::storage::read_lz4_envelope_meta(&path) else {
            continue;
        };
        let trimmed = patch.trim();
        if trimmed.is_empty() {
            continue;
        }
        if best.as_ref().is_none_or(|(ts, _)| created_at >= *ts) {
            best = Some((created_at, trimmed.to_string()));
        }
    }
    best.map(|(_, version)| version)
}

/// Returns the latest DataDragon version stored on disk (`Cache\ddragon\`) or in RAM.
pub fn latest_cached_ddragon_version() -> Option<String> {
    if let Some(paths) = crate::storage::global_paths()
        && let Some(disk_version) = latest_cached_ddragon_version_from_dir(&paths.ddragon_dir)
    {
        return Some(disk_version);
    }
    super::cache::get_ddragon_cache_store()
        .read()
        .ok()
        .and_then(|guard| {
            guard
                .values()
                .map(|c| c.version.trim())
                .find(|v| !v.is_empty())
                .map(str::to_string)
        })
}

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
    let icon_url = if version.trim().is_empty() {
        String::new()
    } else {
        format!(
            "https://ddragon.leagueoflegends.com/cdn/{}/img/spell/{}.png",
            version, icon_name
        )
    };
    let modes = default_spell_modes(spell_id)
        .map(|list| list.iter().map(|m| (*m).to_string()).collect())
        .unwrap_or_default();
    SpellMetadata {
        id: spell_id,
        name: name.to_string(),
        icon_url,
        modes,
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

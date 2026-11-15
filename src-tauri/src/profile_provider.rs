use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};
use std::time::Duration;

use crate::build_provider::{self, DDRAGON_BASE_URL, ddragon::resolve_spell_metadata_for_locale};
use crate::lcu_connector::LcuConnection;
use crate::models::{
    AppLocale, CachedProfileResult, ChampionMasteryInfo, DataDragonCache, PlayerProfile,
    RankedQueueStats, RecentMatchInfo,
};

static SKIN_NAME_CACHE: OnceLock<RwLock<HashMap<AppLocale, HashMap<i32, String>>>> =
    OnceLock::new();

fn skin_cache() -> &'static RwLock<HashMap<AppLocale, HashMap<i32, String>>> {
    SKIN_NAME_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn skins_file_path(dir: &Path, locale: AppLocale) -> PathBuf {
    dir.join(format!("skins_{}.lz4", locale.as_str()))
}

pub fn prewarm_skins_from_dir(dir: &Path, locale: AppLocale) -> usize {
    let path = skins_file_path(dir, locale);
    let Some(env) = crate::storage::read_lz4_envelope::<HashMap<i32, String>>(&path) else {
        return 0;
    };
    let count = env.payload.len();
    if let Ok(mut guard) = skin_cache().write() {
        let entry = guard.entry(locale).or_default();
        for (k, v) in env.payload {
            entry.entry(k).or_insert(v);
        }
    }
    count
}

pub fn prewarm_skins_from_disk(locale: AppLocale) -> usize {
    let Some(paths) = crate::storage::global_paths() else {
        return 0;
    };
    prewarm_skins_from_dir(&paths.ddragon_dir, locale)
}

pub fn lookup_cached_skin_name(locale: AppLocale, background_skin_id: i32) -> Option<String> {
    if background_skin_id <= 0 {
        return None;
    }
    if let Ok(guard) = skin_cache().read()
        && let Some(locale_map) = guard.get(&locale)
        && let Some(name) = locale_map.get(&background_skin_id)
    {
        return Some(name.clone());
    }
    None
}

fn save_skins_map_to_dir(dir: &Path, locale: AppLocale, version: &str) {
    let snapshot = skin_cache()
        .read()
        .ok()
        .and_then(|g| g.get(&locale).cloned());
    if let Some(map) = snapshot {
        let path = skins_file_path(dir, locale);
        let _ = crate::storage::write_lz4_envelope(&path, version, &map);
    }
}

pub fn profile_cache_file_path(dir: &Path, summoner_id: u64) -> PathBuf {
    dir.join(format!("summoner_{summoner_id}.lz4"))
}

pub fn save_profile_to_dir(dir: &Path, profile: &PlayerProfile, patch: &str) -> Result<(), String> {
    if profile.summoner.summoner_id == 0 {
        return Ok(());
    }
    let path = profile_cache_file_path(dir, profile.summoner.summoner_id);
    crate::storage::write_lz4_envelope(&path, patch, profile)
}

pub fn load_profile_from_dir(
    dir: &Path,
    summoner_id: u64,
    now_unix: u64,
) -> Option<CachedProfileResult> {
    if summoner_id == 0 {
        return None;
    }
    let path = profile_cache_file_path(dir, summoner_id);
    let env = crate::storage::read_lz4_envelope::<PlayerProfile>(&path)?;
    if !crate::storage::is_envelope_fresh(
        env.created_at_unix,
        crate::storage::PROFILE_CACHE_TTL_SECS,
        now_unix,
    ) {
        return None;
    }
    let is_stale = !crate::storage::is_envelope_fresh(
        env.created_at_unix,
        crate::storage::PROFILE_SWR_SECS,
        now_unix,
    );
    Some(CachedProfileResult {
        profile: env.payload,
        created_at_unix: env.created_at_unix,
        is_stale,
    })
}

/// Re-localizes cached ranked tier labels, queue mode labels, champion names,
/// and cached skin name in-memory when switching UI language without waiting on LCU.
pub fn relocalize_player_profile(
    profile: &mut PlayerProfile,
    locale: AppLocale,
    ddragon: Option<&DataDragonCache>,
) {
    for stat in &mut profile.ranked_stats {
        stat.tier_label = translate_ranked_tier_for_locale(&stat.tier, locale);
    }

    for m in &mut profile.recent_matches {
        let (_, label) = map_queue_mode_for_locale(m.queue_id, &m.game_mode_key, locale);
        m.game_mode_label = label;
        if let Some(cache) = ddragon
            && let Some(champ) = cache.champions.get(&m.champion_id)
        {
            m.champion_name = champ.display_name.clone();
        }
    }

    if let Some(cache) = ddragon {
        for mastery in &mut profile.top_masteries {
            if let Some(champ) = cache.champions.get(&mastery.champion_id) {
                mastery.champion_name = champ.display_name.clone();
            }
        }
    }

    if let Some(skin_name) = lookup_cached_skin_name(locale, profile.background_skin_id) {
        profile.background_skin_name = skin_name;
    } else if profile.background_skin_id > 0
        && let Some(cache) = ddragon
    {
        let (_, _, default_label, _, _) =
            resolve_background_splash_urls_for_locale(profile.background_skin_id, cache, locale);
        if profile.background_skin_name.is_empty()
            || profile.background_skin_name.ends_with("(Clássica)")
            || profile.background_skin_name.ends_with("(Classic)")
        {
            profile.background_skin_name = default_label;
        }
    }
}

pub fn get_cached_profile_for_summoner_id(
    summoner_id: u64,
    locale: AppLocale,
) -> Option<CachedProfileResult> {
    let paths = crate::storage::global_paths()?;
    prewarm_skins_from_dir(&paths.ddragon_dir, locale);
    let now = crate::storage::now_unix_secs();
    let mut cached = load_profile_from_dir(&paths.profile_dir, summoner_id, now)?;
    let dd_cache = build_provider::get_ddragon_cache_store()
        .read()
        .ok()
        .and_then(|g| g.get(&locale).cloned())
        .or_else(|| build_provider::ddragon::prewarm_ddragon_from_disk(locale));
    relocalize_player_profile(&mut cached.profile, locale, dd_cache.as_deref());
    Some(cached)
}

pub fn get_cached_profile_for_last_summoner(locale: AppLocale) -> Option<CachedProfileResult> {
    let last_id = crate::storage::get_global_settings()
        .navigation
        .last_summoner_id?;
    get_cached_profile_for_summoner_id(last_id, locale)
}

/// Extracts `backgroundSkinId` from `/lol-summoner/v1/current-summoner/summoner-profile`.
pub fn extract_background_skin_id(profile_json: &serde_json::Value) -> i32 {
    profile_json
        .get("backgroundSkinId")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .unwrap_or(0)
}

pub fn resolve_background_splash_urls_for_locale(
    background_skin_id: i32,
    cache: &DataDragonCache,
    locale: AppLocale,
) -> (String, String, String, String, i32) {
    if background_skin_id <= 0 {
        return (
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            0,
        );
    }

    let champion_id = background_skin_id / 1000;
    let skin_num = background_skin_id % 1000;

    if let Some(champ) = cache.champions.get(&champion_id) {
        let splash_url = format!(
            "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/{}_{}.jpg",
            champ.key_name, skin_num
        );
        let fallback_url = format!(
            "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/{}_0.jpg",
            champ.key_name
        );
        let classic_suffix = match locale {
            AppLocale::PtBr => "Clássica",
            AppLocale::EnUs => "Classic",
        };
        let default_label = if skin_num == 0 {
            format!("{} ({})", champ.display_name, classic_suffix)
        } else {
            format!("{} #{}", champ.display_name, skin_num)
        };
        (
            splash_url,
            fallback_url,
            default_label,
            champ.key_name.clone(),
            skin_num,
        )
    } else {
        (
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            skin_num,
        )
    }
}

/// Resolves DataDragon splash URLs and default label from a `backgroundSkinId` (`championId * 1000 + skinNum`).
pub fn resolve_background_splash_urls(
    background_skin_id: i32,
    cache: &DataDragonCache,
) -> (String, String, String, String, i32) {
    resolve_background_splash_urls_for_locale(
        background_skin_id,
        cache,
        crate::lcu_injector::get_active_locale(),
    )
}

pub async fn fetch_skin_name_for_locale_with_base(
    client: &reqwest::Client,
    base_url: &str,
    version: &str,
    key_name: &str,
    skin_num: i32,
    background_skin_id: i32,
    locale: AppLocale,
    disk_dir: Option<&Path>,
) -> Option<String> {
    if version.is_empty() || key_name.is_empty() || background_skin_id <= 0 {
        return None;
    }

    if let Some(cached) = lookup_cached_skin_name(locale, background_skin_id) {
        return Some(cached);
    }

    if let Some(dir) = disk_dir {
        prewarm_skins_from_dir(dir, locale);
        if let Some(cached) = lookup_cached_skin_name(locale, background_skin_id) {
            return Some(cached);
        }
    }

    let url = format!(
        "{}/cdn/{}/data/{}/champion/{}.json",
        base_url.trim_end_matches('/'),
        version,
        locale.ddragon_code(),
        key_name
    );

    let res = tokio::time::timeout(Duration::from_millis(1800), client.get(&url).send())
        .await
        .ok()?
        .ok()?;
    if !res.status().is_success() {
        return None;
    }

    let json = res.json::<serde_json::Value>().await.ok()?;
    let skins = json
        .pointer(&format!("/data/{}/skins", key_name))
        .and_then(|v| v.as_array())?;

    let matched = skins.iter().find(|skin| {
        let num = skin.get("num").and_then(|v| v.as_i64()).map(|n| n as i32);
        let id = skin.get("id").and_then(|v| v.as_i64()).map(|n| n as i32);
        num == Some(skin_num) || id == Some(background_skin_id)
    })?;

    let raw_name = matched.get("name").and_then(|v| v.as_str())?;
    let champ_name = json
        .pointer(&format!("/data/{}/name", key_name))
        .and_then(|v| v.as_str())
        .unwrap_or(key_name);

    let formatted = if raw_name.eq_ignore_ascii_case("default") {
        let classic = match locale {
            AppLocale::PtBr => "Clássica",
            AppLocale::EnUs => "Classic",
        };
        format!("{} ({})", champ_name, classic)
    } else {
        raw_name.to_string()
    };

    if let Ok(mut guard) = skin_cache().write() {
        guard
            .entry(locale)
            .or_default()
            .insert(background_skin_id, formatted.clone());
    }
    if let Some(dir) = disk_dir {
        save_skins_map_to_dir(dir, locale, version);
    }

    Some(formatted)
}

/// Fetches the localized skin name from DataDragon `champion/{key_name}.json` with caching and short timeout.
pub async fn fetch_skin_name_for_locale(
    client: &reqwest::Client,
    version: &str,
    key_name: &str,
    skin_num: i32,
    background_skin_id: i32,
    locale: AppLocale,
) -> Option<String> {
    let dir = crate::storage::global_paths().map(|p| p.ddragon_dir.as_path());
    fetch_skin_name_for_locale_with_base(
        client,
        DDRAGON_BASE_URL,
        version,
        key_name,
        skin_num,
        background_skin_id,
        locale,
        dir,
    )
    .await
}

pub async fn fetch_skin_name(
    client: &reqwest::Client,
    version: &str,
    key_name: &str,
    skin_num: i32,
    background_skin_id: i32,
) -> Option<String> {
    fetch_skin_name_for_locale(
        client,
        version,
        key_name,
        skin_num,
        background_skin_id,
        crate::lcu_injector::get_active_locale(),
    )
    .await
}

/// Translates LCU ranked tier keys to localized labels for `locale`.
pub fn translate_ranked_tier_for_locale(tier: &str, locale: AppLocale) -> String {
    match locale {
        AppLocale::PtBr => match tier.trim().to_ascii_uppercase().as_str() {
            "IRON" => "Ferro",
            "BRONZE" => "Bronze",
            "SILVER" => "Prata",
            "GOLD" => "Ouro",
            "PLATINUM" => "Platina",
            "EMERALD" => "Esmeralda",
            "DIAMOND" => "Diamante",
            "MASTER" => "Mestre",
            "GRANDMASTER" => "Grão-Mestre",
            "CHALLENGER" => "Desafiante",
            _ => "Sem Ranque",
        },
        AppLocale::EnUs => match tier.trim().to_ascii_uppercase().as_str() {
            "IRON" => "Iron",
            "BRONZE" => "Bronze",
            "SILVER" => "Silver",
            "GOLD" => "Gold",
            "PLATINUM" => "Platinum",
            "EMERALD" => "Emerald",
            "DIAMOND" => "Diamond",
            "MASTER" => "Master",
            "GRANDMASTER" => "Grandmaster",
            "CHALLENGER" => "Challenger",
            _ => "Unranked",
        },
    }
    .to_string()
}

/// Translates LCU ranked tier keys to localized PT-BR labels.
pub fn translate_ranked_tier(tier: &str) -> String {
    translate_ranked_tier_for_locale(tier, AppLocale::PtBr)
}

fn parse_single_ranked_entry(
    queue_type: &str,
    queue_label: &str,
    entry: Option<&serde_json::Value>,
    locale: AppLocale,
) -> RankedQueueStats {
    let Some(val) = entry else {
        return RankedQueueStats {
            queue_type: queue_type.to_string(),
            queue_label: queue_label.to_string(),
            tier: "UNRANKED".to_string(),
            tier_label: translate_ranked_tier_for_locale("UNRANKED", locale),
            division: String::new(),
            league_points: 0,
            wins: 0,
            losses: 0,
        };
    };

    let raw_tier = val
        .get("tier")
        .and_then(|v| v.as_str())
        .unwrap_or("UNRANKED")
        .trim()
        .to_ascii_uppercase();

    let is_unranked =
        raw_tier.is_empty() || raw_tier == "NONE" || raw_tier == "NA" || raw_tier == "UNRANKED";
    let tier = if is_unranked {
        "UNRANKED".to_string()
    } else {
        raw_tier
    };

    let division = if is_unranked {
        String::new()
    } else {
        val.get("division")
            .and_then(|v| v.as_str())
            .filter(|d| !d.eq_ignore_ascii_case("NA") && !d.eq_ignore_ascii_case("NONE"))
            .unwrap_or("")
            .to_string()
    };

    RankedQueueStats {
        queue_type: queue_type.to_string(),
        queue_label: queue_label.to_string(),
        tier_label: translate_ranked_tier_for_locale(&tier, locale),
        tier,
        division,
        league_points: val
            .get("leaguePoints")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32,
        wins: val.get("wins").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        losses: val.get("losses").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
    }
}

/// Parses `/lol-ranked/v1/current-ranked-stats` for `locale`.
pub fn parse_ranked_stats_for_locale(
    ranked_json: &serde_json::Value,
    locale: AppLocale,
) -> Vec<RankedQueueStats> {
    let find_queue = |target_type: &str| -> Option<&serde_json::Value> {
        if let Some(queues) = ranked_json.get("queues").and_then(|q| q.as_array())
            && let Some(found) = queues.iter().find(|item| {
                item.get("queueType")
                    .and_then(|t| t.as_str())
                    .is_some_and(|t| t == target_type)
            })
        {
            return Some(found);
        }
        ranked_json
            .get("queueMap")
            .and_then(|qm| qm.get(target_type))
    };

    vec![
        parse_single_ranked_entry(
            "RANKED_SOLO_5x5",
            "Solo/Duo",
            find_queue("RANKED_SOLO_5x5"),
            locale,
        ),
        parse_single_ranked_entry(
            "RANKED_FLEX_SR",
            "Flex",
            find_queue("RANKED_FLEX_SR"),
            locale,
        ),
    ]
}

/// Parses `/lol-ranked/v1/current-ranked-stats` supporting both `queues` array and `queueMap` object.
pub fn parse_ranked_stats(ranked_json: &serde_json::Value) -> Vec<RankedQueueStats> {
    parse_ranked_stats_for_locale(ranked_json, AppLocale::PtBr)
}

/// Parses LCU champion mastery payloads (array or wrapped object) and returns Top N sorted by points.
pub fn parse_top_masteries(
    masteries_json: &serde_json::Value,
    cache: &DataDragonCache,
    limit: usize,
) -> Vec<ChampionMasteryInfo> {
    let items_opt = masteries_json
        .as_array()
        .or_else(|| masteries_json.get("masteries").and_then(|v| v.as_array()))
        .or_else(|| {
            masteries_json
                .get("championMasteries")
                .and_then(|v| v.as_array())
        })
        .or_else(|| masteries_json.get("data").and_then(|v| v.as_array()));

    let Some(items) = items_opt else {
        return Vec::new();
    };

    let mut parsed: Vec<ChampionMasteryInfo> = items
        .iter()
        .filter_map(|entry| {
            let champion_id = entry
                .get("championId")
                .or_else(|| entry.get("champion_id"))
                .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))?
                as i32;
            if champion_id <= 0 {
                return None;
            }
            let champion_level = entry
                .get("championLevel")
                .or_else(|| entry.get("champion_level"))
                .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
                .unwrap_or(1) as u32;
            let champion_points = entry
                .get("championPoints")
                .or_else(|| entry.get("champion_points"))
                .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
                .unwrap_or(0);

            let (champion_name, champion_icon_url) = match cache.champions.get(&champion_id) {
                Some(meta) => (meta.display_name.clone(), meta.icon_url.clone()),
                None => (format!("Campeão {}", champion_id), String::new()),
            };

            Some(ChampionMasteryInfo {
                champion_id,
                champion_name,
                champion_icon_url,
                champion_level,
                champion_points,
            })
        })
        .collect();

    parsed.sort_by(|a, b| {
        b.champion_points
            .cmp(&a.champion_points)
            .then_with(|| b.champion_level.cmp(&a.champion_level))
    });
    parsed.truncate(limit);
    parsed
}

/// Returns true when an LCU match history entry is Practice Tool and must be strictly excluded.
pub fn is_practice_tool_match(game_json: &serde_json::Value) -> bool {
    let queue_id = game_json
        .get("queueId")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;
    let game_mode = game_json
        .get("gameMode")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_ascii_uppercase();
    let game_type = game_json
        .get("gameType")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_ascii_uppercase();
    let participants_len = game_json
        .get("participants")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    if game_mode == "PRACTICETOOL" || game_mode.contains("PRACTICE") {
        return true;
    }

    if queue_id < 0 {
        return true;
    }

    // Fallback safety: custom solo sandbox session with queueId 0
    if queue_id == 0 && game_type == "CUSTOM_GAME" && participants_len <= 1 {
        return true;
    }

    false
}

/// Maps LCU `queueId` and `gameMode` to normalized filter key and localized mode label.
pub fn map_queue_mode_for_locale(
    queue_id: i32,
    game_mode: &str,
    locale: AppLocale,
) -> (String, String) {
    match queue_id {
        420 => ("ranked_solo".to_string(), "Solo/Duo".to_string()),
        440 => ("ranked_flex".to_string(), "Flex".to_string()),
        400 | 430 | 480 | 490 => ("normal".to_string(), "Normal".to_string()),
        450 => ("aram".to_string(), "ARAM".to_string()),
        2400 => ("aram_mayhem".to_string(), "ARAM Mayhem".to_string()),
        1700 | 1710 => ("arena".to_string(), "Arena".to_string()),
        900 | 1010 | 1900 => ("urf".to_string(), "URF".to_string()),
        1300 => (
            "nexus_blitz".to_string(),
            match locale {
                AppLocale::PtBr => "Blitz do Nexus",
                AppLocale::EnUs => "Nexus Blitz",
            }
            .to_string(),
        ),
        1400 => (
            "ultbook".to_string(),
            match locale {
                AppLocale::PtBr => "Livro Supremo",
                AppLocale::EnUs => "Ultimate Spellbook",
            }
            .to_string(),
        ),
        830..=890 => (
            "coop_vs_ai".to_string(),
            match locale {
                AppLocale::PtBr => "Co-op vs IA",
                AppLocale::EnUs => "Co-op vs AI",
            }
            .to_string(),
        ),
        _ => {
            let mode_upper = game_mode.trim().to_ascii_uppercase();
            if mode_upper.contains("MAYHEM") || mode_upper == "KIWI" {
                ("aram_mayhem".to_string(), "ARAM Mayhem".to_string())
            } else if mode_upper == "ARAM" {
                ("aram".to_string(), "ARAM".to_string())
            } else if mode_upper == "CHERRY" || mode_upper == "ARENA" {
                ("arena".to_string(), "Arena".to_string())
            } else if mode_upper.contains("URF") {
                ("urf".to_string(), "URF".to_string())
            } else if mode_upper == "CLASSIC" || mode_upper == "SWIFTPLAY" {
                ("normal".to_string(), "Normal".to_string())
            } else if mode_upper.is_empty() {
                (
                    "other".to_string(),
                    match locale {
                        AppLocale::PtBr => "Outro",
                        AppLocale::EnUs => "Other",
                    }
                    .to_string(),
                )
            } else {
                (mode_upper.to_ascii_lowercase(), mode_upper)
            }
        }
    }
}

/// Maps LCU `queueId` and `gameMode` to normalized filter key and localized PT-BR mode label.
pub fn map_queue_mode(queue_id: i32, game_mode: &str) -> (String, String) {
    map_queue_mode_for_locale(queue_id, game_mode, AppLocale::PtBr)
}

fn find_local_participant<'a>(
    game_json: &'a serde_json::Value,
    summoner_id: u64,
) -> Option<&'a serde_json::Value> {
    let participants = game_json.get("participants")?.as_array()?;
    if participants.is_empty() {
        return None;
    }

    if summoner_id > 0
        && let Some(identities) = game_json
            .get("participantIdentities")
            .and_then(|v| v.as_array())
        && let Some(local_identity) = identities.iter().find(|ident| {
            ident
                .pointer("/player/summonerId")
                .and_then(|id| id.as_u64())
                == Some(summoner_id)
        })
        && let Some(part_id) = local_identity.get("participantId").and_then(|v| v.as_i64())
        && let Some(matched) = participants
            .iter()
            .find(|p| p.get("participantId").and_then(|v| v.as_i64()) == Some(part_id))
    {
        return Some(matched);
    }

    participants.first()
}

/// Parses LCU match history JSON, excludes Practice Tool matches, and enriches up to `limit` matches.
pub fn parse_recent_matches_for_locale(
    history_json: &serde_json::Value,
    summoner_id: u64,
    cache: &DataDragonCache,
    limit: usize,
    locale: AppLocale,
) -> Vec<RecentMatchInfo> {
    let games = history_json
        .pointer("/games/games")
        .and_then(|v| v.as_array())
        .or_else(|| history_json.get("games").and_then(|v| v.as_array()));

    let Some(games_arr) = games else {
        return Vec::new();
    };

    let mut matches: Vec<RecentMatchInfo> = games_arr
        .iter()
        .filter(|game| !is_practice_tool_match(game))
        .filter_map(|game| {
            let participant = find_local_participant(game, summoner_id)?;
            let stats = participant.get("stats").unwrap_or(&serde_json::Value::Null);

            let game_id = game.get("gameId").and_then(|v| v.as_u64()).unwrap_or(0);
            let queue_id = game.get("queueId").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let game_mode = game.get("gameMode").and_then(|v| v.as_str()).unwrap_or("");
            let (game_mode_key, game_mode_label) =
                map_queue_mode_for_locale(queue_id, game_mode, locale);

            let champion_id = participant
                .get("championId")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            let (champion_name, champion_icon_url) = match cache.champions.get(&champion_id) {
                Some(meta) => (meta.display_name.clone(), meta.icon_url.clone()),
                None => (
                    match locale {
                        AppLocale::PtBr => format!("Campeão {}", champion_id),
                        AppLocale::EnUs => format!("Champion {}", champion_id),
                    },
                    String::new(),
                ),
            };

            let spell1_id = participant
                .get("spell1Id")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            let spell2_id = participant
                .get("spell2Id")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            let spell1_icon_url = if spell1_id > 0 {
                resolve_spell_metadata_for_locale(cache, spell1_id, locale).icon_url
            } else {
                String::new()
            };
            let spell2_icon_url = if spell2_id > 0 {
                resolve_spell_metadata_for_locale(cache, spell2_id, locale).icon_url
            } else {
                String::new()
            };

            let mut item_ids = Vec::with_capacity(6);
            let mut item_icon_urls = Vec::with_capacity(6);
            for slot in 0..6 {
                let key = format!("item{}", slot);
                let item_id = stats.get(&key).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                item_ids.push(item_id);
                if item_id > 0 && !cache.version.is_empty() {
                    item_icon_urls.push(format!(
                        "https://ddragon.leagueoflegends.com/cdn/{}/img/item/{}.png",
                        cache.version, item_id
                    ));
                } else {
                    item_icon_urls.push(String::new());
                }
            }

            let kills = stats.get("kills").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let deaths = stats.get("deaths").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let assists = stats.get("assists").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let minions = stats
                .get("totalMinionsKilled")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                + stats
                    .get("neutralMinionsKilled")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);

            Some(RecentMatchInfo {
                game_id,
                queue_id,
                game_mode_key,
                game_mode_label,
                champion_id,
                champion_name,
                champion_icon_url,
                win: stats.get("win").and_then(|v| v.as_bool()).unwrap_or(false),
                kills,
                deaths,
                assists,
                cs: minions as i32,
                game_duration_secs: game
                    .get("gameDuration")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                game_creation_ms: game
                    .get("gameCreation")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                spell1_id,
                spell1_icon_url,
                spell2_id,
                spell2_icon_url,
                item_ids,
                item_icon_urls,
                gold_earned: stats
                    .get("goldEarned")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                total_damage_dealt_to_champions: stats
                    .get("totalDamageDealtToChampions")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                vision_score: stats
                    .get("visionScore")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                wards_placed: stats
                    .get("wardsPlaced")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32,
                champ_level: stats
                    .get("champLevel")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1) as i32,
                double_kills: stats
                    .get("doubleKills")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32,
                triple_kills: stats
                    .get("tripleKills")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32,
                quadra_kills: stats
                    .get("quadraKills")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32,
                penta_kills: stats
                    .get("pentaKills")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32,
            })
        })
        .collect();

    // Ensure most recent matches come first
    matches.sort_by(|a, b| b.game_creation_ms.cmp(&a.game_creation_ms));
    matches.truncate(limit);
    matches
}

/// Parses LCU match history JSON, excludes Practice Tool matches, and enriches up to `limit` matches.
pub fn parse_recent_matches(
    history_json: &serde_json::Value,
    summoner_id: u64,
    cache: &DataDragonCache,
    limit: usize,
) -> Vec<RecentMatchInfo> {
    parse_recent_matches_for_locale(history_json, summoner_id, cache, limit, AppLocale::PtBr)
}

/// Fetches and assembles the full `PlayerProfile` from an active LCU connection + DataDragon cache,
/// supporting disk `.lz4` caching (24h TTL, SWR > 5m, `force_refresh` bypass) and `LastSummonerId` persistence.
pub async fn load_player_profile_with_cache(
    conn: &LcuConnection,
    force_refresh: bool,
) -> Result<PlayerProfile, String> {
    let locale = crate::lcu_injector::get_active_locale();
    let (summoner, puuid) = conn.get_current_summoner_with_puuid().await?;
    let http_client = build_provider::shared_http_client();

    if summoner.summoner_id > 0 {
        let current_last = crate::storage::get_global_settings()
            .navigation
            .last_summoner_id;
        if current_last != Some(summoner.summoner_id) {
            let _ = crate::storage::update_global_settings(|s| {
                s.navigation.last_summoner_id = Some(summoner.summoner_id);
            });
        }
    }

    if !force_refresh
        && let Some(cached) = get_cached_profile_for_summoner_id(summoner.summoner_id, locale)
        && !cached.is_stale
    {
        return Ok(cached.profile);
    }

    let default_cache = DataDragonCache {
        version: "16.22.1".to_string(),
        ..Default::default()
    };
    let cache =
        match build_provider::get_or_init_ddragon_cache_for_locale(http_client, locale).await {
            Ok(c) => c.as_ref().clone(),
            Err(_) => default_cache,
        };

    let (profile_res, masteries_res, ranked_res, history_res) = tokio::join!(
        conn.get_summoner_profile(),
        conn.get_champion_masteries(summoner.summoner_id, puuid.as_deref()),
        conn.get_ranked_stats(),
        conn.get_match_history()
    );

    let background_skin_id = profile_res
        .as_ref()
        .map(extract_background_skin_id)
        .unwrap_or(0);

    let (background_splash_url, background_splash_fallback_url, default_label, key_name, skin_num) =
        resolve_background_splash_urls_for_locale(background_skin_id, &cache, locale);

    let background_skin_name = if background_skin_id > 0 && !key_name.is_empty() {
        fetch_skin_name_for_locale(
            http_client,
            &cache.version,
            &key_name,
            skin_num,
            background_skin_id,
            locale,
        )
        .await
        .unwrap_or(default_label)
    } else {
        String::new()
    };

    let ranked_stats = ranked_res
        .as_ref()
        .map(|val| parse_ranked_stats_for_locale(val, locale))
        .unwrap_or_else(|_| parse_ranked_stats_for_locale(&serde_json::Value::Null, locale));

    let top_masteries = masteries_res
        .as_ref()
        .map(|val| parse_top_masteries(val, &cache, 3))
        .unwrap_or_default();

    let recent_matches = history_res
        .as_ref()
        .map(|val| parse_recent_matches_for_locale(val, summoner.summoner_id, &cache, 10, locale))
        .unwrap_or_default();

    let profile_icon_url = if !cache.version.is_empty() {
        format!(
            "https://ddragon.leagueoflegends.com/cdn/{}/img/profileicon/{}.png",
            cache.version, summoner.profile_icon_id
        )
    } else {
        String::new()
    };

    let profile = PlayerProfile {
        summoner,
        profile_icon_url,
        background_skin_id,
        background_skin_name,
        background_splash_url,
        background_splash_fallback_url,
        ranked_stats,
        top_masteries,
        recent_matches,
    };

    if let Some(paths) = crate::storage::global_paths() {
        let _ = save_profile_to_dir(&paths.profile_dir, &profile, &cache.version);
    }

    Ok(profile)
}

pub async fn load_player_profile(conn: &LcuConnection) -> Result<PlayerProfile, String> {
    load_player_profile_with_cache(conn, false).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ChampionMetadata, SpellMetadata, SummonerInfo};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_cache() -> DataDragonCache {
        let mut champions = HashMap::new();
        champions.insert(
            103,
            ChampionMetadata {
                champion_id: 103,
                key_name: "Ahri".to_string(),
                display_name: "Ahri".to_string(),
                icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Ahri.png"
                    .to_string(),
            },
        );
        champions.insert(
            238,
            ChampionMetadata {
                champion_id: 238,
                key_name: "Zed".to_string(),
                display_name: "Zed".to_string(),
                icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Zed.png"
                    .to_string(),
            },
        );
        champions.insert(
            157,
            ChampionMetadata {
                champion_id: 157,
                key_name: "Yasuo".to_string(),
                display_name: "Yasuo".to_string(),
                icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Yasuo.png"
                    .to_string(),
            },
        );
        champions.insert(
            64,
            ChampionMetadata {
                champion_id: 64,
                key_name: "LeeSin".to_string(),
                display_name: "Lee Sin".to_string(),
                icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/LeeSin.png"
                    .to_string(),
            },
        );

        let mut spells = HashMap::new();
        spells.insert(
            4,
            SpellMetadata {
                id: 4,
                name: "Flash".to_string(),
                icon_url:
                    "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/spell/SummonerFlash.png"
                        .to_string(),
            },
        );
        spells.insert(
            14,
            SpellMetadata {
                id: 14,
                name: "Incendiar".to_string(),
                icon_url:
                    "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/spell/SummonerDot.png"
                        .to_string(),
            },
        );

        DataDragonCache {
            version: "16.22.1".to_string(),
            champions,
            runes: HashMap::new(),
            spells,
        }
    }

    #[test]
    fn test_resolve_background_splash_urls_extracts_champion_and_skin_num() {
        let cache = sample_cache();
        let (splash, fallback, label, key, skin_num) =
            resolve_background_splash_urls(103015, &cache);

        assert_eq!(
            splash,
            "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_15.jpg"
        );
        assert_eq!(
            fallback,
            "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_0.jpg"
        );
        assert_eq!(label, "Ahri #15");
        assert_eq!(key, "Ahri");
        assert_eq!(skin_num, 15);
    }

    #[test]
    fn test_parse_top_masteries_sorts_and_limits_to_top_3() {
        let cache = sample_cache();
        let raw = serde_json::json!([
            { "championId": 157, "championLevel": 10, "championPoints": 120000 },
            { "championId": 103, "championLevel": 25, "championPoints": 450000 },
            { "championId": 64, "championLevel": 7, "championPoints": 80000 },
            { "championId": 238, "championLevel": 18, "championPoints": 310000 }
        ]);

        let top3 = parse_top_masteries(&raw, &cache, 3);
        assert_eq!(top3.len(), 3);
        assert_eq!(top3[0].champion_id, 103);
        assert_eq!(top3[0].champion_name, "Ahri");
        assert_eq!(top3[0].champion_points, 450000);
        assert_eq!(top3[1].champion_id, 238);
        assert_eq!(top3[2].champion_id, 157);

        let wrapped = serde_json::json!({ "masteries": raw });
        let wrapped_top3 = parse_top_masteries(&wrapped, &cache, 3);
        assert_eq!(wrapped_top3.len(), 3);
        assert_eq!(wrapped_top3[0].champion_id, 103);
    }

    #[test]
    fn test_parse_ranked_stats_handles_queues_and_unranked() {
        let raw = serde_json::json!({
            "queues": [
                {
                    "queueType": "RANKED_SOLO_5x5",
                    "tier": "EMERALD",
                    "division": "II",
                    "leaguePoints": 67,
                    "wins": 50,
                    "losses": 40
                },
                {
                    "queueType": "RANKED_FLEX_SR",
                    "tier": "NONE",
                    "division": "NA",
                    "leaguePoints": 0,
                    "wins": 0,
                    "losses": 0
                }
            ]
        });

        let stats = parse_ranked_stats(&raw);
        assert_eq!(stats.len(), 2);
        assert_eq!(stats[0].queue_label, "Solo/Duo");
        assert_eq!(stats[0].tier_label, "Esmeralda");
        assert_eq!(stats[0].division, "II");
        assert_eq!(stats[0].league_points, 67);
        assert_eq!(stats[1].queue_label, "Flex");
        assert_eq!(stats[1].tier, "UNRANKED");
        assert_eq!(stats[1].tier_label, "Sem Ranque");
    }

    #[test]
    fn test_parse_recent_matches_strictly_excludes_practice_tool_and_caps_at_10() {
        let cache = sample_cache();
        let mut games = vec![serde_json::json!({
            "gameId": 999,
            "queueId": 0,
            "gameMode": "PRACTICETOOL",
            "gameType": "CUSTOM_GAME",
            "gameDuration": 300,
            "gameCreation": 2000000000000i64,
            "participants": [{
                "championId": 103,
                "spell1Id": 4,
                "spell2Id": 14,
                "stats": { "win": true, "kills": 0, "deaths": 0, "assists": 0 }
            }]
        })];

        for i in 1..=12 {
            games.push(serde_json::json!({
                "gameId": i,
                "queueId": if i % 2 == 0 { 420 } else { 450 },
                "gameMode": if i % 2 == 0 { "CLASSIC" } else { "ARAM" },
                "gameType": "MATCHED_GAME",
                "gameDuration": 1500 + i * 10,
                "gameCreation": 1700000000000i64 + i * 100000,
                "participants": [{
                    "championId": 103,
                    "spell1Id": 4,
                    "spell2Id": 14,
                    "stats": {
                        "win": i % 3 != 0,
                        "kills": 8,
                        "deaths": 2,
                        "assists": 10,
                        "totalMinionsKilled": 180,
                        "neutralMinionsKilled": 15,
                        "item0": 6655,
                        "item1": 3020,
                        "item2": 3089,
                        "item3": 0,
                        "item4": 0,
                        "item5": 0,
                        "goldEarned": 14500,
                        "totalDamageDealtToChampions": 29800,
                        "visionScore": 22,
                        "wardsPlaced": 9,
                        "champLevel": 16,
                        "doubleKills": 1
                    }
                }]
            }));
        }

        let history = serde_json::json!({ "games": { "games": games } });
        let parsed = parse_recent_matches(&history, 0, &cache, 10);

        assert_eq!(parsed.len(), 10);
        assert!(parsed.iter().all(|m| m.game_id != 999));
        assert!(parsed.iter().all(|m| m.game_mode_key != "practicetool"));
        // Most recent first (gameId 12 down to 3)
        assert_eq!(parsed[0].game_id, 12);
        assert_eq!(parsed[0].game_mode_label, "Solo/Duo");
        assert_eq!(parsed[0].cs, 195);
        assert_eq!(parsed[0].item_ids.len(), 6);
        assert_eq!(parsed[0].double_kills, 1);
    }

    #[test]
    fn test_profile_localization_for_en_us() {
        assert_eq!(
            translate_ranked_tier_for_locale("EMERALD", AppLocale::EnUs),
            "Emerald"
        );
        assert_eq!(
            translate_ranked_tier_for_locale("NONE", AppLocale::EnUs),
            "Unranked"
        );
        assert_eq!(
            map_queue_mode_for_locale(1400, "", AppLocale::EnUs).1,
            "Ultimate Spellbook"
        );
        assert_eq!(
            map_queue_mode_for_locale(1300, "", AppLocale::EnUs).1,
            "Nexus Blitz"
        );
    }

    #[tokio::test]
    async fn test_skin_cache_lz4_incremental_and_profile_persistence_relocalization() {
        let dir = std::env::temp_dir().join(format!(
            "schumazgg_profile_cache_test_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let ddragon_dir = dir.join("ddragon");
        let profile_dir = dir.join("profile");
        std::fs::create_dir_all(&ddragon_dir).unwrap();
        std::fs::create_dir_all(&profile_dir).unwrap();

        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/cdn/16.22.1/data/pt_BR/champion/Ahri.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "Ahri": {
                        "name": "Ahri",
                        "skins": [
                            { "id": 103027, "num": 27, "name": "Ahri Florescer Espiritual" }
                        ]
                    }
                }
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let skin_name = fetch_skin_name_for_locale_with_base(
            &client,
            &mock_server.uri(),
            "16.22.1",
            "Ahri",
            27,
            103027,
            AppLocale::PtBr,
            Some(&ddragon_dir),
        )
        .await
        .unwrap();
        assert_eq!(skin_name, "Ahri Florescer Espiritual");
        assert!(skins_file_path(&ddragon_dir, AppLocale::PtBr).exists());

        // Clear RAM map and reload from disk to prove 0-network disk hydration
        if let Ok(mut g) = skin_cache().write() {
            g.remove(&AppLocale::PtBr);
        }
        assert_eq!(prewarm_skins_from_dir(&ddragon_dir, AppLocale::PtBr), 1);
        assert_eq!(
            lookup_cached_skin_name(AppLocale::PtBr, 103027).as_deref(),
            Some("Ahri Florescer Espiritual")
        );

        // Save & reload profile with SWR detection + EN-US relocalization
        let profile = PlayerProfile {
            summoner: SummonerInfo {
                summoner_id: 55501, game_name: "Tester".to_string(),
                display_name: "Tester".to_string(),
                tag_line: "BR1".to_string(),
                profile_icon_id: 123,
                summoner_level: 100,
            },
            profile_icon_url:
                "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/profileicon/123.png"
                    .to_string(),
            background_skin_id: 103027,
            background_skin_name: skin_name,
            background_splash_url:
                "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_27.jpg"
                    .to_string(),
            background_splash_fallback_url:
                "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_0.jpg".to_string(),
            ranked_stats: vec![RankedQueueStats {
                queue_type: "RANKED_SOLO_5x5".to_string(),
                queue_label: "Solo/Duo".to_string(),
                tier: "EMERALD".to_string(),
                tier_label: "Esmeralda".to_string(),
                division: "II".to_string(),
                league_points: 50,
                wins: 10,
                losses: 8,
            }],
            top_masteries: vec![],
            recent_matches: vec![],
        };

        save_profile_to_dir(&profile_dir, &profile, "16.22.1").unwrap();
        let now = crate::storage::now_unix_secs();
        let loaded_fresh = load_profile_from_dir(&profile_dir, 55501, now).unwrap();
        assert!(!loaded_fresh.is_stale);

        let mut loaded_stale = load_profile_from_dir(&profile_dir, 55501, now + 400).unwrap();
        assert!(loaded_stale.is_stale);

        // Relocalize cached profile to en_US
        let dd = sample_cache();
        relocalize_player_profile(&mut loaded_stale.profile, AppLocale::EnUs, Some(&dd));
        assert_eq!(loaded_stale.profile.ranked_stats[0].tier_label, "Emerald");

        // Expired after 24h
        assert!(load_profile_from_dir(&profile_dir, 55501, now + 25 * 3600).is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }
}

use crate::build_provider::ddragon::resolve_spell_metadata_for_locale;
use crate::models::{AppLocale, DataDragonCache, MAP_ARENA, RecentMatchInfo};

use super::match_metadata::{
    extract_arena_placement, extract_match_augments, extract_match_runes, extract_match_trinket,
    is_remake_match,
};
use super::queue_mapper::map_queue_mode_for_locale;

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

fn find_local_participant(
    game_json: &serde_json::Value,
    summoner_id: u64,
) -> Option<&serde_json::Value> {
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

/// Parsed match history page preserving the raw LCU cursor across Practice Tool filtering.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedMatchHistoryPage {
    pub matches: Vec<RecentMatchInfo>,
    pub raw_games_count: u32,
    pub next_beg_index: u32,
    pub has_more: bool,
}

fn extract_games_array(history_json: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
    history_json
        .pointer("/games/games")
        .and_then(|v| v.as_array())
        .or_else(|| history_json.get("games").and_then(|v| v.as_array()))
}

fn parse_single_match(
    game: &serde_json::Value,
    summoner_id: u64,
    cache: &DataDragonCache,
    locale: AppLocale,
) -> Option<RecentMatchInfo> {
    let participant = find_local_participant(game, summoner_id)?;
    let stats = participant.get("stats").unwrap_or(&serde_json::Value::Null);

    let game_id = game.get("gameId").and_then(|v| v.as_u64()).unwrap_or(0);
    let queue_id = game.get("queueId").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let game_mode = game.get("gameMode").and_then(|v| v.as_str()).unwrap_or("");
    let (game_mode_key, game_mode_label) =
        if game.get("mapId").and_then(|v| v.as_i64()) == Some(MAP_ARENA as i64) {
            ("arena".to_string(), "Arena".to_string())
        } else {
            map_queue_mode_for_locale(queue_id, game_mode, locale)
        };

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
    let game_duration_secs = game
        .get("gameDuration")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let remake = is_remake_match(stats, game_duration_secs, kills, deaths, assists);
    let arena_placement = extract_arena_placement(stats, &game_mode_key);
    let runes = extract_match_runes(stats, cache);
    let augment_ids =
        extract_match_augments(stats, queue_id, &game_mode_key, runes.primary_rune_id);
    let (trinket_id, trinket_icon_url) = extract_match_trinket(stats, &cache.version);

    Some(RecentMatchInfo {
        game_id,
        queue_id,
        game_mode_key,
        game_mode_label,
        champion_id,
        champion_name,
        champion_icon_url,
        win: stats.get("win").and_then(|v| v.as_bool()).unwrap_or(false),
        remake,
        arena_placement,
        kills,
        deaths,
        assists,
        cs: minions as i32,
        game_duration_secs,
        game_creation_ms: game
            .get("gameCreation")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        spell1_id,
        spell1_icon_url,
        spell2_id,
        spell2_icon_url,
        primary_rune_id: runes.primary_rune_id,
        primary_rune_icon_url: runes.primary_rune_icon_url,
        primary_style_id: runes.primary_style_id,
        primary_style_icon_url: runes.primary_style_icon_url,
        sub_style_id: runes.sub_style_id,
        sub_style_icon_url: runes.sub_style_icon_url,
        augment_ids,
        item_ids,
        item_icon_urls,
        trinket_id,
        trinket_icon_url,
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
}

/// Parses a paginated LCU match history window while advancing the raw cursor regardless of Practice Tool exclusions.
pub fn parse_match_history_page_for_locale(
    history_json: &serde_json::Value,
    summoner_id: u64,
    cache: &DataDragonCache,
    beg_index: u32,
    page_size: u32,
    locale: AppLocale,
) -> ParsedMatchHistoryPage {
    let Some(games_arr) = extract_games_array(history_json) else {
        return ParsedMatchHistoryPage {
            matches: Vec::new(),
            raw_games_count: 0,
            next_beg_index: beg_index,
            has_more: false,
        };
    };

    let effective_page_size = page_size.max(1) as usize;
    let (mut matches, raw_games_count, has_more) = if games_arr.len() <= effective_page_size {
        let raw_count = u32::try_from(games_arr.len()).unwrap_or(u32::MAX);
        let parsed = games_arr
            .iter()
            .filter(|game| !is_practice_tool_match(game))
            .filter_map(|game| parse_single_match(game, summoner_id, cache, locale))
            .collect::<Vec<_>>();
        (parsed, raw_count, games_arr.len() >= effective_page_size)
    } else {
        // Fallback when an unparameterized endpoint returns more than one page on beg_index=0.
        let mut ordered_refs: Vec<&serde_json::Value> = games_arr.iter().collect();
        ordered_refs.sort_by_key(|g| {
            std::cmp::Reverse(g.get("gameCreation").and_then(|v| v.as_i64()).unwrap_or(0))
        });
        let raw_count = u32::try_from(effective_page_size).unwrap_or(u32::MAX);
        let parsed = ordered_refs
            .into_iter()
            .take(effective_page_size)
            .filter(|game| !is_practice_tool_match(game))
            .filter_map(|game| parse_single_match(game, summoner_id, cache, locale))
            .collect::<Vec<_>>();
        (parsed, raw_count, true)
    };

    matches.sort_by_key(|m| std::cmp::Reverse(m.game_creation_ms));
    ParsedMatchHistoryPage {
        matches,
        raw_games_count,
        next_beg_index: beg_index.saturating_add(raw_games_count),
        has_more,
    }
}

/// Parses LCU match history JSON, excludes Practice Tool matches, and enriches up to `limit` matches.
pub fn parse_recent_matches_for_locale(
    history_json: &serde_json::Value,
    summoner_id: u64,
    cache: &DataDragonCache,
    limit: usize,
    locale: AppLocale,
) -> Vec<RecentMatchInfo> {
    let Some(games_arr) = extract_games_array(history_json) else {
        return Vec::new();
    };

    let mut matches: Vec<RecentMatchInfo> = games_arr
        .iter()
        .filter(|game| !is_practice_tool_match(game))
        .filter_map(|game| parse_single_match(game, summoner_id, cache, locale))
        .collect();

    // Ensure most recent matches come first
    matches.sort_by_key(|m| std::cmp::Reverse(m.game_creation_ms));
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

#[cfg(test)]
#[path = "matches_parser_tests.rs"]
mod tests;

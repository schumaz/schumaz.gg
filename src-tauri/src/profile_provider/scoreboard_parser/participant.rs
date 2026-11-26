use std::collections::HashMap;

use crate::models::{AppLocale, DataDragonCache, ScoreboardParticipant};
use crate::storage::is_allowed_asset_url;

use super::super::match_metadata::{
    extract_arena_placement, extract_match_augments, extract_match_runes, extract_match_trinket,
    resolve_allowed_spell_icon,
};

#[derive(Debug, Default)]
pub(super) struct ParticipantIdentity {
    pub summoner_id: u64,
    pub puuid: String,
    pub game_name: String,
    pub tag_line: String,
    pub summoner_name: String,
}

pub(super) struct ParseParticipantCtx<'a> {
    pub identities: &'a HashMap<i32, ParticipantIdentity>,
    pub local_summoner_id: u64,
    pub local_puuid: Option<&'a str>,
    pub queue_id: i32,
    pub game_mode_key: &'a str,
    pub cache: &'a DataDragonCache,
    pub locale: AppLocale,
}

pub(super) fn str_field(val: &serde_json::Value, key: &str) -> String {
    val.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

pub(super) fn i32_field(val: &serde_json::Value, key: &str) -> i32 {
    val.get(key)
        .and_then(|v| v.as_i64())
        .map(|n| n.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
        .unwrap_or(0)
}

pub(super) fn i64_field(val: &serde_json::Value, key: &str) -> i64 {
    val.get(key).and_then(|v| v.as_i64()).unwrap_or(0)
}

pub(super) fn parse_identities(game_json: &serde_json::Value) -> HashMap<i32, ParticipantIdentity> {
    let mut map = HashMap::new();
    let Some(identities) = game_json.get("participantIdentities").and_then(|v| v.as_array()) else {
        return map;
    };
    for ident in identities {
        let part_id = i32_field(ident, "participantId");
        if part_id <= 0 {
            continue;
        }
        let player = ident.get("player").unwrap_or(ident);
        map.insert(
            part_id,
            ParticipantIdentity {
                summoner_id: player.get("summonerId").and_then(|v| v.as_u64()).unwrap_or(0),
                puuid: str_field(player, "puuid"),
                game_name: str_field(player, "gameName"),
                tag_line: str_field(player, "tagLine"),
                summoner_name: str_field(player, "summonerName"),
            },
        );
    }
    map
}

fn resolve_champion_visuals(
    cache: &DataDragonCache,
    champion_id: i32,
    locale: AppLocale,
) -> (String, String) {
    if let Some(meta) = cache.champions.get(&champion_id) {
        let icon = if is_allowed_asset_url(&meta.icon_url) {
            meta.icon_url.clone()
        } else {
            String::new()
        };
        return (meta.display_name.clone(), icon);
    }
    let fallback_name = match locale {
        AppLocale::PtBr => format!("Campeão {}", champion_id),
        AppLocale::EnUs => format!("Champion {}", champion_id),
    };
    (fallback_name, String::new())
}

pub(super) fn parse_participant(
    p: &serde_json::Value,
    idx: usize,
    ctx: &ParseParticipantCtx<'_>,
) -> (ScoreboardParticipant, bool) {
    let stats = p.get("stats").unwrap_or(&serde_json::Value::Null);
    let raw_pid = i32_field(p, "participantId");
    let participant_id = if raw_pid > 0 { raw_pid } else { (idx + 1) as i32 };
    let team_id = i32_field(p, "teamId");
    let subteam_id = i32_field(stats, "playerSubteamId")
        .max(i32_field(p, "playerSubteamId"))
        .max(0);
    let arena_placement = extract_arena_placement(stats, ctx.game_mode_key)
        .or_else(|| extract_arena_placement(p, ctx.game_mode_key));

    let ident = ctx.identities.get(&participant_id);
    let summoner_id = ident
        .map(|i| i.summoner_id)
        .filter(|&id| id > 0)
        .or_else(|| p.get("summonerId").and_then(|v| v.as_u64()))
        .unwrap_or(0);
    let puuid = ident
        .map(|i| i.puuid.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| str_field(p, "puuid"));
    let is_local_player = (ctx.local_summoner_id > 0 && summoner_id == ctx.local_summoner_id)
        || ctx.local_puuid.is_some_and(|lp| !lp.is_empty() && puuid == lp);

    let champion_id = i32_field(p, "championId");
    let (champion_name, champion_icon_url) =
        resolve_champion_visuals(ctx.cache, champion_id, ctx.locale);

    let game_name = ident
        .map(|i| i.game_name.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| str_field(p, "riotIdGameName"));
    let tag_line = ident
        .map(|i| i.tag_line.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| str_field(p, "riotIdTagline"));
    let summoner_name = ident
        .map(|i| i.summoner_name.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| str_field(p, "summonerName"));

    let (riot_id_name, riot_id_tag) = if !game_name.is_empty() {
        (game_name, tag_line)
    } else if !summoner_name.is_empty() {
        (summoner_name, String::new())
    } else {
        (champion_name.clone(), String::new())
    };

    let spell1_id = i32_field(p, "spell1Id");
    let spell2_id = i32_field(p, "spell2Id");
    let runes = extract_match_runes(stats, ctx.cache);
    let augment_ids =
        extract_match_augments(stats, ctx.queue_id, ctx.game_mode_key, runes.primary_rune_id);
    let (trinket_id, trinket_icon_url) = extract_match_trinket(stats, &ctx.cache.version);

    let dd_version = ctx.cache.version.trim();
    let mut item_ids = Vec::with_capacity(6);
    let mut item_icon_urls = Vec::with_capacity(6);
    for slot in 0..6 {
        let item_id = i32_field(stats, &format!("item{slot}")).max(0);
        item_ids.push(item_id);
        let url = if item_id > 0 && !dd_version.is_empty() {
            format!("https://ddragon.leagueoflegends.com/cdn/{dd_version}/img/item/{item_id}.png")
        } else {
            String::new()
        };
        item_icon_urls.push(url);
    }

    let early_surrender = stats
        .get("gameEndedInEarlySurrender")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let participant = ScoreboardParticipant {
        participant_id,
        team_id,
        subteam_id,
        arena_placement,
        is_local_player,
        riot_id_name,
        riot_id_tag,
        champion_id,
        champion_name,
        champion_icon_url,
        champ_level: i32_field(stats, "champLevel").max(1),
        win: stats.get("win").and_then(|v| v.as_bool()).unwrap_or(false),
        kills: i32_field(stats, "kills"),
        deaths: i32_field(stats, "deaths"),
        assists: i32_field(stats, "assists"),
        kp_percent: 0,
        cs: i32_field(stats, "totalMinionsKilled") + i32_field(stats, "neutralMinionsKilled"),
        gold_earned: i64_field(stats, "goldEarned"),
        total_damage_dealt_to_champions: i64_field(stats, "totalDamageDealtToChampions"),
        vision_score: i64_field(stats, "visionScore"),
        wards_placed: i32_field(stats, "wardsPlaced"),
        control_wards_bought: i32_field(stats, "visionWardsBoughtInGame"),
        spell1_id,
        spell1_icon_url: resolve_allowed_spell_icon(ctx.cache, spell1_id, ctx.locale),
        spell2_id,
        spell2_icon_url: resolve_allowed_spell_icon(ctx.cache, spell2_id, ctx.locale),
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
    };
    (participant, early_surrender)
}

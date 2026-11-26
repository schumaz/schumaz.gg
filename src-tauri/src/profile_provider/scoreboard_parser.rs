use std::collections::{BTreeMap, HashMap};

use crate::models::{
    AppLocale, DataDragonCache, MAP_ARENA, MatchScoreboardDetail, ScoreboardArenaTeam,
    ScoreboardParticipant, ScoreboardTeamSummary, is_arena_game_mode, is_arena_queue,
};

use super::match_metadata::{
    collect_augment_metadata_from_ids, collect_item_metadata_from_ids,
    collect_rune_metadata_from_ids, collect_spell_metadata_from_ids,
};
use super::queue_mapper::map_queue_mode_for_locale;

mod participant;
use participant::{
    ParseParticipantCtx, i32_field, i64_field, parse_identities, parse_participant, str_field,
};

#[derive(Debug, Default)]
struct RawTeamMeta {
    win: Option<bool>,
    towers: i32,
    dragons: i32,
    barons: i32,
    heralds: i32,
    voidgrubs: i32,
    inhibitors: i32,
}

fn parse_team_win(team_val: &serde_json::Value) -> Option<bool> {
    let win_val = team_val.get("win")?;
    if let Some(b) = win_val.as_bool() {
        return Some(b);
    }
    win_val.as_str().and_then(|s| {
        if s.eq_ignore_ascii_case("win") {
            Some(true)
        } else if s.eq_ignore_ascii_case("fail") {
            Some(false)
        } else {
            None
        }
    })
}

fn parse_teams_meta(game_json: &serde_json::Value) -> HashMap<i32, RawTeamMeta> {
    let mut map = HashMap::new();
    let Some(teams) = game_json.get("teams").and_then(|v| v.as_array()) else {
        return map;
    };
    for team in teams {
        let team_id = i32_field(team, "teamId");
        if team_id <= 0 {
            continue;
        }
        let voidgrubs = team
            .get("hordeKills")
            .or_else(|| team.get("voidgrubKills"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        map.insert(
            team_id,
            RawTeamMeta {
                win: parse_team_win(team),
                towers: i32_field(team, "towerKills"),
                dragons: i32_field(team, "dragonKills"),
                barons: i32_field(team, "baronKills"),
                heralds: i32_field(team, "riftHeraldKills"),
                voidgrubs,
                inhibitors: i32_field(team, "inhibitorKills"),
            },
        );
    }
    map
}

fn compute_kp_percent(kills: i32, assists: i32, team_kills: i32) -> i32 {
    if team_kills <= 0 {
        return 0;
    }
    let ratio = ((kills + assists).max(0) as f64 / team_kills as f64) * 100.0;
    ratio.round().clamp(0.0, 100.0) as i32
}

fn build_arena_teams(participants: Vec<ScoreboardParticipant>) -> Vec<ScoreboardArenaTeam> {
    let mut grouped: BTreeMap<i32, Vec<ScoreboardParticipant>> = BTreeMap::new();
    for p in participants {
        let key = if p.subteam_id > 0 {
            p.subteam_id
        } else if let Some(place) = p.arena_placement {
            100 + place
        } else {
            1000 + p.participant_id
        };
        grouped.entry(key).or_default().push(p);
    }

    let mut arena_teams: Vec<ScoreboardArenaTeam> = grouped
        .into_iter()
        .map(|(group_key, mut members)| {
            members.sort_by_key(|p| p.participant_id);
            let total_kills: i32 = members.iter().map(|p| p.kills).sum();
            let total_deaths: i32 = members.iter().map(|p| p.deaths).sum();
            let total_assists: i32 = members.iter().map(|p| p.assists).sum();
            let total_damage: i64 = members.iter().map(|p| p.total_damage_dealt_to_champions).sum();
            for p in &mut members {
                p.kp_percent = compute_kp_percent(p.kills, p.assists, total_kills);
            }
            let placement = members.iter().find_map(|p| p.arena_placement).unwrap_or(0);
            let subteam_id = members
                .first()
                .map(|p| p.subteam_id)
                .filter(|&id| id > 0)
                .unwrap_or(group_key);
            let win = if placement > 0 {
                placement <= 4
            } else {
                members.iter().any(|p| p.win)
            };
            ScoreboardArenaTeam {
                subteam_id,
                placement,
                win,
                total_kills,
                total_deaths,
                total_assists,
                total_damage,
                participants: members,
            }
        })
        .collect();

    arena_teams.sort_by_key(|t| {
        let rank = if (1..=8).contains(&t.placement) { t.placement } else { 99 };
        (rank, t.subteam_id, t.participants.first().map_or(0, |p| p.participant_id))
    });
    arena_teams
}

fn build_standard_teams(
    participants: Vec<ScoreboardParticipant>,
    teams_meta: &HashMap<i32, RawTeamMeta>,
) -> Vec<ScoreboardTeamSummary> {
    let mut grouped: BTreeMap<i32, Vec<ScoreboardParticipant>> = BTreeMap::new();
    for p in participants {
        grouped.entry(p.team_id).or_default().push(p);
    }

    grouped
        .into_iter()
        .map(|(team_id, mut members)| {
            members.sort_by_key(|p| p.participant_id);
            let total_kills: i32 = members.iter().map(|p| p.kills).sum();
            let total_deaths: i32 = members.iter().map(|p| p.deaths).sum();
            let total_assists: i32 = members.iter().map(|p| p.assists).sum();
            let total_gold: i64 = members.iter().map(|p| p.gold_earned).sum();
            for p in &mut members {
                p.kp_percent = compute_kp_percent(p.kills, p.assists, total_kills);
            }
            let meta = teams_meta.get(&team_id);
            let win = meta
                .and_then(|m| m.win)
                .unwrap_or_else(|| members.first().is_some_and(|p| p.win));
            ScoreboardTeamSummary {
                team_id,
                win,
                total_kills,
                total_deaths,
                total_assists,
                total_gold,
                towers: meta.map_or(0, |m| m.towers),
                dragons: meta.map_or(0, |m| m.dragons),
                barons: meta.map_or(0, |m| m.barons),
                heralds: meta.map_or(0, |m| m.heralds),
                voidgrubs: meta.map_or(0, |m| m.voidgrubs),
                inhibitors: meta.map_or(0, |m| m.inhibitors),
                participants: members,
            }
        })
        .collect()
}

/// Parses full LCU `/lol-match-history/v1/games/{gameId}` JSON into a 5v5 or Arena scoreboard.
pub fn parse_match_scoreboard_detail_for_locale(
    game_json: &serde_json::Value,
    local_summoner_id: u64,
    local_puuid: Option<&str>,
    cache: &DataDragonCache,
    locale: AppLocale,
) -> Result<MatchScoreboardDetail, String> {
    let raw_participants = game_json
        .get("participants")
        .and_then(|v| v.as_array())
        .filter(|arr| !arr.is_empty())
        .ok_or_else(|| "Match scoreboard payload has no participants".to_string())?;

    let game_id = game_json.get("gameId").and_then(|v| v.as_u64()).unwrap_or(0);
    let queue_id = i32_field(game_json, "queueId");
    let map_id = i32_field(game_json, "mapId");
    let game_mode = str_field(game_json, "gameMode");
    let game_duration_secs = i64_field(game_json, "gameDuration");
    let game_creation_ms = i64_field(game_json, "gameCreation");

    let is_arena = map_id == MAP_ARENA || is_arena_queue(queue_id) || is_arena_game_mode(&game_mode);
    let (game_mode_key, game_mode_label) = if is_arena {
        ("arena".to_string(), "Arena".to_string())
    } else {
        map_queue_mode_for_locale(queue_id, &game_mode, locale)
    };

    let identities = parse_identities(game_json);
    let ctx = ParseParticipantCtx {
        identities: &identities,
        local_summoner_id,
        local_puuid,
        queue_id,
        game_mode_key: &game_mode_key,
        cache,
        locale,
    };

    let mut parsed = Vec::with_capacity(raw_participants.len());
    let mut any_early_surrender = false;
    for (idx, p) in raw_participants.iter().enumerate() {
        let (participant, early) = parse_participant(p, idx, &ctx);
        any_early_surrender |= early;
        parsed.push(participant);
    }

    let max_damage_dealt_to_champions = parsed
        .iter()
        .map(|p| p.total_damage_dealt_to_champions)
        .max()
        .unwrap_or(0)
        .max(0);
    let total_kills_all: i32 = parsed.iter().map(|p| p.kills).sum();
    let remake = any_early_surrender
        || (game_duration_secs > 0 && game_duration_secs < 210 && total_kills_all == 0);

    let item_metadata = collect_item_metadata_from_ids(
        parsed
            .iter()
            .flat_map(|p| p.item_ids.iter().copied().chain(std::iter::once(p.trinket_id))),
        &cache.items,
    );
    let rune_metadata = collect_rune_metadata_from_ids(
        parsed
            .iter()
            .flat_map(|p| [p.primary_rune_id, p.primary_style_id, p.sub_style_id]),
        &cache.runes,
    );
    let spell_metadata = collect_spell_metadata_from_ids(
        parsed.iter().flat_map(|p| [p.spell1_id, p.spell2_id]),
        cache,
        locale,
    );
    let augment_metadata = collect_augment_metadata_from_ids(
        parsed.iter().flat_map(|p| p.augment_ids.iter().copied()),
        &cache.arena_augments,
    );

    let (teams, arena_teams) = if is_arena {
        (Vec::new(), build_arena_teams(parsed))
    } else {
        let teams_meta = parse_teams_meta(game_json);
        (build_standard_teams(parsed, &teams_meta), Vec::new())
    };

    Ok(MatchScoreboardDetail {
        game_id,
        queue_id,
        map_id,
        game_mode_key,
        game_mode_label,
        game_duration_secs,
        game_creation_ms,
        is_arena,
        remake,
        max_damage_dealt_to_champions,
        teams,
        arena_teams,
        item_metadata,
        rune_metadata,
        spell_metadata,
        augment_metadata,
    })
}

#[cfg(test)]
#[path = "scoreboard_parser_tests.rs"]
mod tests;

use std::collections::HashMap;

use crate::models::{AppLocale, ChampionMasteryInfo, DataDragonCache};

pub fn localized_unknown_champion_name(champion_id: i32, locale: AppLocale) -> String {
    match locale {
        AppLocale::PtBr => format!("Campeão {champion_id}"),
        AppLocale::EnUs => format!("Champion {champion_id}"),
    }
}

fn read_i64_field(entry: &serde_json::Value, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| {
        entry
            .get(*key)
            .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
    })
}

fn read_u64_field(entry: &serde_json::Value, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| {
        entry.get(*key).and_then(|v| {
            v.as_u64()
                .or_else(|| v.as_i64().map(|i| i.max(0) as u64))
                .or_else(|| v.as_f64().map(|f| f.max(0.0) as u64))
        })
    })
}

/// Parses LCU champion mastery payloads (array or wrapped object), deduplicates by `champion_id`,
/// and returns all entries sorted by points desc, level desc, then localized name asc.
pub fn parse_all_masteries_for_locale(
    masteries_json: &serde_json::Value,
    cache: &DataDragonCache,
    locale: AppLocale,
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

    let mut deduped: HashMap<i32, ChampionMasteryInfo> = HashMap::new();

    for entry in items {
        let Some(raw_id) = read_i64_field(entry, &["championId", "champion_id"]) else {
            continue;
        };
        let champion_id = raw_id as i32;
        if champion_id <= 0 {
            continue;
        }

        let champion_level =
            read_u64_field(entry, &["championLevel", "champion_level"]).unwrap_or(1) as u32;
        let champion_points =
            read_u64_field(entry, &["championPoints", "champion_points"]).unwrap_or(0);
        let last_play_time_ms =
            read_i64_field(entry, &["lastPlayTime", "last_play_time", "last_play_time_ms"])
                .unwrap_or(0)
                .max(0);
        let champion_points_since_last_level = read_u64_field(
            entry,
            &[
                "championPointsSinceLastLevel",
                "champion_points_since_last_level",
            ],
        )
        .unwrap_or(0);
        let champion_points_until_next_level = read_u64_field(
            entry,
            &[
                "championPointsUntilNextLevel",
                "champion_points_until_next_level",
            ],
        )
        .unwrap_or(0);
        let tokens_earned =
            read_u64_field(entry, &["tokensEarned", "tokens_earned"]).unwrap_or(0) as u32;
        let mark_required_for_next_level = read_u64_field(
            entry,
            &["markRequiredForNextLevel", "mark_required_for_next_level"],
        )
        .unwrap_or(0) as u32;

        let (champion_name, champion_icon_url) = match cache.champions.get(&champion_id) {
            Some(meta) => (meta.display_name.clone(), meta.icon_url.clone()),
            None => (localized_unknown_champion_name(champion_id, locale), String::new()),
        };

        let candidate = ChampionMasteryInfo {
            champion_id,
            champion_name,
            champion_icon_url,
            champion_level,
            champion_points,
            last_play_time_ms,
            champion_points_since_last_level,
            champion_points_until_next_level,
            tokens_earned,
            mark_required_for_next_level,
        };

        match deduped.get(&champion_id) {
            Some(existing)
                if (existing.champion_points, existing.champion_level)
                    >= (candidate.champion_points, candidate.champion_level) => {}
            _ => {
                deduped.insert(champion_id, candidate);
            }
        }
    }

    let mut parsed: Vec<ChampionMasteryInfo> = deduped.into_values().collect();
    parsed.sort_by(|a, b| {
        b.champion_points
            .cmp(&a.champion_points)
            .then_with(|| b.champion_level.cmp(&a.champion_level))
            .then_with(|| a.champion_name.cmp(&b.champion_name))
    });
    parsed
}

/// Parses LCU champion mastery payloads and returns Top N sorted by points for `locale`.
pub fn parse_top_masteries_for_locale(
    masteries_json: &serde_json::Value,
    cache: &DataDragonCache,
    limit: usize,
    locale: AppLocale,
) -> Vec<ChampionMasteryInfo> {
    let mut parsed = parse_all_masteries_for_locale(masteries_json, cache, locale);
    parsed.truncate(limit);
    parsed
}

/// Parses LCU champion mastery payloads (array or wrapped object) and returns Top N sorted by points.
pub fn parse_top_masteries(
    masteries_json: &serde_json::Value,
    cache: &DataDragonCache,
    limit: usize,
) -> Vec<ChampionMasteryInfo> {
    parse_top_masteries_for_locale(masteries_json, cache, limit, AppLocale::PtBr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ChampionMetadata;

    fn sample_cache() -> DataDragonCache {
        let mut champions = HashMap::new();
        for (id, key, display) in [
            (103, "Ahri", "Ahri"),
            (238, "Zed", "Zed"),
            (157, "Yasuo", "Yasuo"),
            (64, "LeeSin", "Lee Sin"),
        ] {
            champions.insert(
                id,
                ChampionMetadata {
                    champion_id: id,
                    key_name: key.to_string(),
                    display_name: display.to_string(),
                    icon_url: format!(
                        "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/{key}.png"
                    ),
                },
            );
        }
        DataDragonCache {
            version: "16.22.1".to_string(),
            champions,
            runes: HashMap::new(),
            spells: HashMap::new(),
            ..Default::default()
        }
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
    fn test_parse_all_masteries_deduplicates_extracts_progress_and_localizes_fallback() {
        let cache = sample_cache();
        let raw = serde_json::json!({
            "championMasteries": [
                {
                    "championId": 103,
                    "championLevel": 25,
                    "championPoints": 450000,
                    "lastPlayTime": 1732500000000_i64,
                    "championPointsSinceLastLevel": 18000,
                    "championPointsUntilNextLevel": 4000,
                    "tokensEarned": 2,
                    "markRequiredForNextLevel": 3
                },
                {
                    "championId": 103,
                    "championLevel": 24,
                    "championPoints": 420000
                },
                {
                    "championId": 9999,
                    "championLevel": 4,
                    "championPoints": 12500,
                    "championPointsUntilNextLevel": -100,
                    "lastPlayTime": -5
                },
                { "championId": 0, "championLevel": 1, "championPoints": 100 }
            ]
        });

        let pt_all = parse_all_masteries_for_locale(&raw, &cache, AppLocale::PtBr);
        assert_eq!(pt_all.len(), 2);
        assert_eq!(pt_all[0].champion_id, 103);
        assert_eq!(pt_all[0].champion_points, 450000);
        assert_eq!(pt_all[0].last_play_time_ms, 1732500000000);
        assert_eq!(pt_all[0].champion_points_since_last_level, 18000);
        assert_eq!(pt_all[0].champion_points_until_next_level, 4000);
        assert_eq!(pt_all[0].tokens_earned, 2);
        assert_eq!(pt_all[0].mark_required_for_next_level, 3);

        assert_eq!(pt_all[1].champion_id, 9999);
        assert_eq!(pt_all[1].champion_name, "Campeão 9999");
        assert!(pt_all[1].champion_icon_url.is_empty());
        assert_eq!(pt_all[1].champion_points_until_next_level, 0);
        assert_eq!(pt_all[1].last_play_time_ms, 0);

        let en_all = parse_all_masteries_for_locale(&raw, &cache, AppLocale::EnUs);
        assert_eq!(en_all[1].champion_name, "Champion 9999");
    }
}

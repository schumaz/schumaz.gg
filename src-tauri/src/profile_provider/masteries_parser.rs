use crate::models::{ChampionMasteryInfo, DataDragonCache};

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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

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
}

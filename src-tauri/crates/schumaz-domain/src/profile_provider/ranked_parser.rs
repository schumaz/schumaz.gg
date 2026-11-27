use crate::models::{AppLocale, RankedQueueStats};

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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_ranked_tier_localization_for_en_us() {
        assert_eq!(
            translate_ranked_tier_for_locale("EMERALD", AppLocale::EnUs),
            "Emerald"
        );
        assert_eq!(
            translate_ranked_tier_for_locale("NONE", AppLocale::EnUs),
            "Unranked"
        );
    }
}

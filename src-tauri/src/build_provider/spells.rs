use std::collections::HashMap;

pub fn extract_summoner_spell_choices(
    analysis: &serde_json::Value,
    is_jungle: bool,
) -> ((i32, i32), Vec<i32>) {
    let mut combo_entries: Vec<(i32, i32, i64)> = Vec::new();

    if let Some(arr) = analysis
        .get("summoner_spells")
        .or_else(|| analysis.get("spells"))
        .and_then(|v| v.as_array())
    {
        for item in arr {
            if let Some(ids) = item.get("ids").and_then(|i| i.as_array())
                && ids.len() >= 2
            {
                let s1 = ids[0].as_i64().unwrap_or(0) as i32;
                let s2 = ids[1].as_i64().unwrap_or(0) as i32;
                let play = item
                    .get("play")
                    .and_then(|p| p.as_i64().or_else(|| p.as_f64().map(|f| f as i64)))
                    .unwrap_or(1);
                if s1 > 0 && s2 > 0 {
                    combo_entries.push((s1, s2, play));
                }
            } else if let Some(ids) = item.as_array()
                && ids.len() >= 2
            {
                let s1 = ids[0].as_i64().unwrap_or(0) as i32;
                let s2 = ids[1].as_i64().unwrap_or(0) as i32;
                if s1 > 0 && s2 > 0 {
                    combo_entries.push((s1, s2, 1));
                }
            }
        }
    }

    combo_entries.sort_by_key(|e| std::cmp::Reverse(e.2));

    let raw_top_combo = if !combo_entries.is_empty() {
        (combo_entries[0].0, combo_entries[0].1)
    } else {
        extract_most_used_summoner_spells(analysis)
    };

    // Normalize numerically-ordered OP.GG pairs (e.g. [1, 4], [3, 4], [32, 4]) so Flash (4)
    // defaults to the primary slot before FlashKeyPreference (D vs F) is applied.
    let top_combo = if !is_jungle && raw_top_combo.1 == 4 && raw_top_combo.0 != 4 {
        (4, raw_top_combo.0)
    } else {
        raw_top_combo
    };

    let mut secondary_choices: Vec<i32> = Vec::new();

    if is_jungle {
        // Smite (11) is mandatory; collect top 2 unique non-Smite companion spells ordered by play count
        let mut companion_counts: HashMap<i32, i64> = HashMap::new();
        for (s1, s2, play) in &combo_entries {
            let companion = if *s1 == 11 {
                Some(*s2)
            } else if *s2 == 11 {
                Some(*s1)
            } else {
                None
            };
            if let Some(comp) = companion
                && comp > 0
                && comp != 11
            {
                *companion_counts.entry(comp).or_insert(0) += *play;
            }
        }
        let mut sorted_companions: Vec<(i32, i64)> = companion_counts.into_iter().collect();
        sorted_companions.sort_by_key(|e| std::cmp::Reverse(e.1));
        for (comp, _) in sorted_companions {
            if !secondary_choices.contains(&comp) {
                secondary_choices.push(comp);
            }
        }
        if secondary_choices.is_empty() {
            let other = if top_combo.0 == 11 {
                top_combo.1
            } else {
                top_combo.0
            };
            if other != 11 && other > 0 {
                secondary_choices.push(other);
            }
        }
        for fallback in [4, 6, 14] {
            if secondary_choices.len() < 2 && !secondary_choices.contains(&fallback) {
                secondary_choices.push(fallback);
            }
        }
    } else {
        // Standard laners / ARAM: find primary anchor spell (Flash 4 if present in top combo, else top_combo.0)
        let primary_spell = if top_combo.0 == 4 || top_combo.1 == 4 {
            4
        } else {
            top_combo.0
        };

        // Determine secondary spell from top combo
        let top_secondary = if top_combo.0 == primary_spell {
            top_combo.1
        } else {
            top_combo.0
        };
        secondary_choices.push(top_secondary);

        // Find alternative secondary spell from remaining combos
        for (s1, s2, _) in &combo_entries {
            let alt = if *s1 == primary_spell {
                *s2
            } else if *s2 == primary_spell {
                *s1
            } else {
                continue;
            };
            if alt > 0 && !secondary_choices.contains(&alt) {
                secondary_choices.push(alt);
                break;
            }
        }

        // Fallbacks if only 1 combo was played in data
        let default_alts = [21, 3, 14, 12, 7, 6, 1]; // Barrier, Exhaust, Ignite, Teleport, Heal, Ghost, Cleanse
        for fallback in default_alts {
            if secondary_choices.len() < 2
                && !secondary_choices.contains(&fallback)
                && fallback != primary_spell
            {
                secondary_choices.push(fallback);
            }
        }
    }

    secondary_choices.truncate(2);
    (top_combo, secondary_choices)
}

/// Extracts the most used summoner spells from OP.GG analysis JSON.
pub fn extract_most_used_summoner_spells(analysis: &serde_json::Value) -> (i32, i32) {
    if let Some(arr) = analysis
        .get("summoner_spells")
        .or_else(|| analysis.get("spells"))
        .and_then(|v| v.as_array())
    {
        let mut entries: Vec<(i32, i32, i64, f64)> = Vec::new();
        for item in arr {
            if let Some(ids) = item.get("ids").and_then(|i| i.as_array())
                && ids.len() >= 2
            {
                let s1 = ids[0].as_i64().unwrap_or(0) as i32;
                let s2 = ids[1].as_i64().unwrap_or(0) as i32;
                let play = item
                    .get("play")
                    .and_then(|p| p.as_i64().or_else(|| p.as_f64().map(|f| f as i64)))
                    .unwrap_or(1);
                let win = item.get("win").and_then(|w| w.as_i64()).unwrap_or(0);
                let win_rate = if play > 0 {
                    win as f64 / play as f64
                } else {
                    0.0
                };
                if s1 > 0 && s2 > 0 {
                    entries.push((s1, s2, play, win_rate));
                }
            } else if let Some(ids) = item.as_array()
                && ids.len() >= 2
            {
                let s1 = ids[0].as_i64().unwrap_or(0) as i32;
                let s2 = ids[1].as_i64().unwrap_or(0) as i32;
                if s1 > 0 && s2 > 0 {
                    return (s1, s2);
                }
            }
        }

        if !entries.is_empty() {
            // Sort by most played combination
            entries.sort_by_key(|e| std::cmp::Reverse(e.2));
            return (entries[0].0, entries[0].1);
        }
    }

    // Direct flat array: "spells": [4, 14] or "summoner_spells": [4, 14]
    if let Some(arr) = analysis.get("spells").and_then(|v| v.as_array())
        && arr.len() >= 2
        && let (Some(s1), Some(s2)) = (arr[0].as_i64(), arr[1].as_i64())
    {
        return (s1 as i32, s2 as i32);
    }

    if let Some(arr) = analysis
        .pointer("/summoner_spells/ids")
        .and_then(|v| v.as_array())
        && arr.len() >= 2
    {
        let s1 = arr[0].as_i64().unwrap_or(4) as i32;
        let s2 = arr[1].as_i64().unwrap_or(14) as i32;
        return (s1, s2);
    }

    // Single object or pointer
    let s1 = analysis
        .pointer("/summoner_spells/0/ids/0")
        .or_else(|| analysis.pointer("/spells/0"))
        .and_then(|v| v.as_i64())
        .unwrap_or(4) as i32;

    let s2 = analysis
        .pointer("/summoner_spells/0/ids/1")
        .or_else(|| analysis.pointer("/spells/1"))
        .and_then(|v| v.as_i64())
        .unwrap_or(14) as i32;

    (s1, s2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_provider::*;

    #[test]
    fn test_extract_summoner_spell_choices_yunara_and_jungle() {
        // Yunara: Flash (4) + Barrier (21) (61,208) vs Exhaust (3) + Flash (4) (17,302)
        let yunara_json = serde_json::json!({
            "summoner_spells": [
                { "ids": [4, 21], "play": 61208, "win": 30365 },
                { "ids": [3, 4], "play": 17302, "win": 8780 }
            ]
        });
        let (top_spells, choices) = extract_summoner_spell_choices(&yunara_json, false);
        assert_eq!(top_spells, (4, 21));
        assert_eq!(choices, vec![21, 3]); // Option 1: Barrier (21), Option 2: Exhaust (3)

        // Jungle (Hecarim): Ghost (6) + Smite (11) (45,000) vs Flash (4) + Smite (11) (12,000)
        let hecarim_json = serde_json::json!({
            "summoner_spells": [
                { "ids": [6, 11], "play": 45000, "win": 23000 },
                { "ids": [4, 11], "play": 12000, "win": 6000 }
            ]
        });
        let (jng_spells, jng_choices) = extract_summoner_spell_choices(&hecarim_json, true);
        assert_eq!(jng_spells, (6, 11));
        assert_eq!(jng_choices, vec![6, 4]); // Option 1: Ghost (6), Option 2: Flash (4)

        // Jungle (Shaco): Ignite (14) + Smite (11) (38,000) vs Flash (4) + Smite (11) (5,000)
        let shaco_json = serde_json::json!({
            "summoner_spells": [
                { "ids": [14, 11], "play": 38000, "win": 20000 },
                { "ids": [4, 11], "play": 5000, "win": 2500 }
            ]
        });
        let (_, shaco_choices) = extract_summoner_spell_choices(&shaco_json, true);
        assert_eq!(shaco_choices, vec![14, 4]); // Option 1: Ignite (14), Option 2: Flash (4)
    }

    #[test]
    fn test_extract_summoner_spell_choices_aram_tristana_and_numeric_opgg_order() {
        // Tristana in ARAM: Flash + Heal (7) is #1, Flash + Barrier (21) is #2, Flash + Mark (32) is #3
        let tristana_aram = serde_json::json!({
            "summoner_spells": [
                { "ids": [4, 32], "play": 8500, "win": 4300 },
                { "ids": [4, 7], "play": 42000, "win": 22500 },
                { "ids": [4, 21], "play": 19500, "win": 10300 }
            ]
        });
        let (top_spells, choices) = extract_summoner_spell_choices(&tristana_aram, false);
        assert_eq!(top_spells, (4, 7)); // Flash + Heal, NOT Mark (32)
        assert_eq!(choices, vec![7, 21]); // Primary: Heal (7), Secondary: Barrier (21)

        // Numerically-ordered OP.GG pairs: [1, 4] (Cleanse + Flash) and [3, 4] (Exhaust + Flash)
        let cleanse_flash = serde_json::json!({
            "summoner_spells": [
                { "ids": [1, 4], "play": 28000, "win": 14500 },
                { "ids": [3, 4], "play": 12000, "win": 6100 }
            ]
        });
        let (cleanse_top, cleanse_choices) = extract_summoner_spell_choices(&cleanse_flash, false);
        assert_eq!(cleanse_top, (4, 1)); // Flash normalized to first slot
        assert_eq!(cleanse_choices, vec![1, 3]); // Cleanse (1), Exhaust (3)
    }

    #[test]
    fn test_extract_most_used_summoner_spells_from_opgg_rest_structure() {
        let json = serde_json::json!({
            "data": {
                "win_rate": 49.5,
                "pick_rate": 12.1,
                "runes": {
                    "primary_style_id": 8000,
                    "sub_style_id": 8300,
                    "primary_perk_ids": [8008, 9111, 9104, 8017],
                    "sub_perk_ids": [8304, 8345],
                    "stat_shards": [5005, 5008, 5011]
                },
                "summoner_spells": [
                    {
                        "ids": [4, 21], // Flash + Barrier (most played: 62087 games)
                        "win": 30822,
                        "play": 62087,
                        "pick_rate": 0.7006
                    },
                    {
                        "ids": [3, 4], // Exhaust + Flash (second: 17529 games)
                        "win": 8894,
                        "play": 17529,
                        "pick_rate": 0.1978
                    },
                    {
                        "ids": [4, 14], // Flash + Ignite (minority: 500 games)
                        "win": 250,
                        "play": 500,
                        "pick_rate": 0.005
                    }
                ],
                "items": {
                    "starters": [1055, 2003],
                    "core": [3031, 3094, 3036]
                }
            }
        });

        let build = parse_opgg_analysis_payload(&json, 804, "Yunara")
            .expect("Parsing Yunara with OP.GG summoner_spells array should succeed");

        // The most played combination is Flash (4) + Barrier (21), NOT Ignite (14)!
        assert_eq!(build.spell1_id, 4);
        assert_eq!(build.spell2_id, 21);
    }
}

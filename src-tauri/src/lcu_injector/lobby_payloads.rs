use serde_json::json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LobbySlotPatch {
    pub slot_index: Option<u32>,
    pub champion_id: i32,
    pub runes: Option<(i32, i32, Vec<i32>)>,
    pub spells: Option<(i32, i32)>,
}

fn validate_rune_parts(
    primary_style_id: i32,
    sub_style_id: i32,
    selected_perk_ids: &[i32],
) -> Result<(), String> {
    if selected_perk_ids.len() != 9 {
        return Err(format!(
            "Invalid perk count: expected 9 perks, found {}",
            selected_perk_ids.len()
        ));
    }
    if primary_style_id == sub_style_id {
        return Err("Primary style and sub style cannot be identical".to_string());
    }
    Ok(())
}

fn parse_existing_perks(existing_perks: Option<&str>) -> Option<(i32, i32, Vec<i32>, bool)> {
    let raw = existing_perks?.trim();
    if raw.is_empty() {
        return None;
    }
    let val: serde_json::Value = serde_json::from_str(raw).ok()?;
    let obj = val.as_object()?;
    let uses_legacy_selected_format =
        obj.contains_key("selectedPerkIds") && !obj.contains_key("perkIds");

    let primary = obj
        .get("perkStyle")
        .or_else(|| obj.get("primaryStyleId"))
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)?;
    let sub = obj
        .get("perkSubStyle")
        .or_else(|| obj.get("subStyleId"))
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)?;
    let perks: Vec<i32> = obj
        .get("perkIds")
        .or_else(|| obj.get("selectedPerkIds"))
        .and_then(|v| v.as_array())?
        .iter()
        .filter_map(|v| v.as_i64().map(|n| n as i32))
        .collect();

    Some((primary, sub, perks, uses_legacy_selected_format))
}

fn perks_string_matches(
    existing_perks: Option<&str>,
    primary_style_id: i32,
    sub_style_id: i32,
    selected_perk_ids: &[i32],
) -> bool {
    let Some((p, s, ids, _)) = parse_existing_perks(existing_perks) else {
        return false;
    };
    p == primary_style_id && s == sub_style_id && ids.as_slice() == selected_perk_ids
}

/// Builds the serialized `perks` JSON string stored inside a Swiftplay/Quickplay `playerSlots[]` entry.
/// Preserves `selectedPerkIds`/`primaryStyleId` format when already used by the client, otherwise
/// emits Riot's standard `perkIds`/`perkStyle`/`perkSubStyle` format.
pub fn build_lobby_slot_perks_string(
    primary_style_id: i32,
    sub_style_id: i32,
    selected_perk_ids: &[i32],
    existing_perks: Option<&str>,
) -> Result<String, String> {
    validate_rune_parts(primary_style_id, sub_style_id, selected_perk_ids)?;

    let uses_legacy_selected = existing_perks
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .and_then(|v| {
            let obj = v.as_object()?;
            Some(obj.contains_key("selectedPerkIds") && !obj.contains_key("perkIds"))
        })
        .unwrap_or(false);

    let payload = if uses_legacy_selected {
        json!({
            "primaryStyleId": primary_style_id,
            "subStyleId": sub_style_id,
            "selectedPerkIds": selected_perk_ids,
        })
    } else {
        json!({
            "perkIds": selected_perk_ids,
            "perkStyle": primary_style_id,
            "perkSubStyle": sub_style_id,
        })
    };

    Ok(payload.to_string())
}

/// Clones `current_slots` (`localMember.playerSlots` array) and applies rune/spell patches
/// strictly on matching champion slots while preserving `skinId`, `skinVariant`, `positionPreference`, etc.
/// Returns `(patched_slots, changed)` so no-op updates skip redundant `PUT` requests.
pub fn patch_lobby_player_slots_value(
    current_slots: &serde_json::Value,
    patches: &[LobbySlotPatch],
) -> Result<(serde_json::Value, bool), String> {
    let source_arr = current_slots
        .as_array()
        .ok_or_else(|| "Lobby playerSlots payload is not an array".to_string())?;
    let mut next_slots = source_arr.clone();
    let mut changed = false;

    for (idx, slot_val) in next_slots.iter_mut().enumerate() {
        let Some(slot_obj) = slot_val.as_object_mut() else {
            continue;
        };
        let slot_champ_id = slot_obj
            .get("championId")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        if slot_champ_id <= 0 {
            continue;
        }

        let matched_patch = patches
            .iter()
            .find(|p| p.slot_index == Some(idx as u32) && p.champion_id == slot_champ_id)
            .or_else(|| {
                patches
                    .iter()
                    .find(|p| p.slot_index.is_none() && p.champion_id == slot_champ_id)
            });

        let Some(patch) = matched_patch else {
            continue;
        };

        if let Some((primary_style_id, sub_style_id, ref perk_ids)) = patch.runes {
            let existing_perks = slot_obj.get("perks").and_then(|v| v.as_str());
            let next_perks = build_lobby_slot_perks_string(
                primary_style_id,
                sub_style_id,
                perk_ids,
                existing_perks,
            )?;
            if !perks_string_matches(existing_perks, primary_style_id, sub_style_id, perk_ids) {
                slot_obj.insert("perks".to_string(), json!(next_perks));
                changed = true;
            }
        }

        if let Some((spell1, spell2)) = patch.spells {
            if spell1 <= 0 || spell2 <= 0 || spell1 == spell2 {
                return Err("Lobby slot summoner spells must be distinct positive IDs".to_string());
            }
            let cur_s1 = slot_obj.get("spell1").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let cur_s2 = slot_obj.get("spell2").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            if cur_s1 != spell1 || cur_s2 != spell2 {
                slot_obj.insert("spell1".to_string(), json!(spell1));
                slot_obj.insert("spell2".to_string(), json!(spell2));
                changed = true;
            }
        }
    }

    Ok((serde_json::Value::Array(next_slots), changed))
}

#[cfg(test)]
#[path = "lobby_payloads_tests.rs"]
mod tests;

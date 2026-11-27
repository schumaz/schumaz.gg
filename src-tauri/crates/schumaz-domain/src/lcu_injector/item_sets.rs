use super::payloads::{SCHUMAZ_PAGE_SUFFIX, build_managed_item_sets};
use crate::build_provider::ChampionBuild;
use crate::lcu_parsers::is_lcu_error_payload;
use crate::logging;

/// Returns `true` when an item set entry is managed by schumaz.gg (`" | schumaz.gg"` in `title`).
pub fn is_schumaz_item_set(set: &serde_json::Value) -> bool {
    set.get("title")
        .and_then(|t| t.as_str())
        .is_some_and(|title| title.contains(SCHUMAZ_PAGE_SUFFIX))
}

fn str_field_matches(
    existing: &serde_json::Value,
    desired: &serde_json::Value,
    key: &str,
) -> bool {
    let Some(cur) = existing.get(key).and_then(|v| v.as_str()) else {
        return false;
    };
    let Some(target) = desired.get(key).and_then(|v| v.as_str()) else {
        return false;
    };
    cur == target
}

fn i64_array_matches(
    existing: Option<&serde_json::Value>,
    desired: Option<&serde_json::Value>,
) -> bool {
    let (Some(cur), Some(target)) = (
        existing.and_then(|v| v.as_array()),
        desired.and_then(|v| v.as_array()),
    ) else {
        return false;
    };
    if cur.len() != target.len() {
        return false;
    }
    cur.iter()
        .zip(target.iter())
        .all(|(a, b)| matches!((a.as_i64(), b.as_i64()), (Some(x), Some(y)) if x == y))
}

fn item_block_matches(
    existing_block: &serde_json::Value,
    desired_block: &serde_json::Value,
) -> bool {
    if !str_field_matches(existing_block, desired_block, "type") {
        return false;
    }
    let (Some(cur_items), Some(target_items)) = (
        existing_block.get("items").and_then(|v| v.as_array()),
        desired_block.get("items").and_then(|v| v.as_array()),
    ) else {
        return false;
    };
    if cur_items.len() != target_items.len() {
        return false;
    }
    cur_items.iter().zip(target_items.iter()).all(|(cur, target)| {
        str_field_matches(cur, target, "id")
            && matches!(
                (
                    cur.get("count").and_then(|v| v.as_i64()),
                    target.get("count").and_then(|v| v.as_i64()),
                ),
                (Some(a), Some(b)) if a == b
            )
    })
}

/// Compares all app-controlled fields of an item set while ignoring LCU-populated metadata
/// (`uid`, `map`, `sortrank`, `startedFrom`, `preferredItemSlots`, and block defaults).
fn managed_item_set_matches(
    existing_set: &serde_json::Value,
    desired_set: &serde_json::Value,
) -> bool {
    if !str_field_matches(existing_set, desired_set, "title")
        || !str_field_matches(existing_set, desired_set, "mode")
        || !str_field_matches(existing_set, desired_set, "type")
        || !i64_array_matches(
            existing_set.get("associatedChampions"),
            desired_set.get("associatedChampions"),
        )
        || !i64_array_matches(
            existing_set.get("associatedMaps"),
            desired_set.get("associatedMaps"),
        )
    {
        return false;
    }

    let (Some(cur_blocks), Some(target_blocks)) = (
        existing_set.get("blocks").and_then(|v| v.as_array()),
        desired_set.get("blocks").and_then(|v| v.as_array()),
    ) else {
        return false;
    };
    if cur_blocks.len() != target_blocks.len() {
        return false;
    }
    cur_blocks
        .iter()
        .zip(target_blocks.iter())
        .all(|(cur, target)| item_block_matches(cur, target))
}

/// Merges app-managed `" | schumaz.gg"` item sets into an existing `/lol-item-sets` payload,
/// preserving all player-created item sets verbatim and returning `(payload, changed)`.
pub fn merge_item_sets_payload(
    summoner_id: u64,
    existing: &serde_json::Value,
    builds: &[&ChampionBuild],
) -> Result<(serde_json::Value, bool), String> {
    if is_lcu_error_payload(existing) {
        return Err("LCU returned an error payload when reading item sets".to_string());
    }
    let Some(existing_obj) = existing.as_object() else {
        return Err("LCU item sets payload is not a JSON object".to_string());
    };

    let empty_sets: [serde_json::Value; 0] = [];
    let existing_sets: &[serde_json::Value] = match existing_obj.get("itemSets") {
        Some(serde_json::Value::Array(arr)) => arr.as_slice(),
        Some(serde_json::Value::Null) => &empty_sets,
        None if existing_obj.contains_key("accountId") => &empty_sets,
        _ => {
            return Err("LCU item sets payload is missing a valid 'itemSets' array".to_string());
        }
    };

    let desired_managed = build_managed_item_sets(builds);
    let insert_idx = existing_sets
        .iter()
        .position(is_schumaz_item_set)
        .unwrap_or(existing_sets.len());

    let mut user_sets_before = Vec::new();
    let mut user_sets_after = Vec::new();
    let mut existing_managed_count = 0usize;

    for (idx, set) in existing_sets.iter().enumerate() {
        if is_schumaz_item_set(set) {
            existing_managed_count += 1;
        } else if idx < insert_idx {
            user_sets_before.push(set.clone());
        } else {
            user_sets_after.push(set.clone());
        }
    }

    let is_unchanged = existing_managed_count == desired_managed.len()
        && insert_idx
            .checked_add(desired_managed.len())
            .and_then(|end| existing_sets.get(insert_idx..end))
            .is_some_and(|slice| {
                slice.iter().all(is_schumaz_item_set)
                    && slice
                        .iter()
                        .zip(desired_managed.iter())
                        .all(|(cur, desired)| managed_item_set_matches(cur, desired))
            });

    if is_unchanged {
        return Ok((existing.clone(), false));
    }

    let mut merged_sets =
        Vec::with_capacity(user_sets_before.len() + desired_managed.len() + user_sets_after.len());
    merged_sets.extend(user_sets_before);
    merged_sets.extend(desired_managed);
    merged_sets.extend(user_sets_after);

    let mut merged_obj = existing_obj.clone();
    let has_valid_account_id = merged_obj
        .get("accountId")
        .and_then(|v| v.as_u64())
        .is_some_and(|id| id > 0);
    if !has_valid_account_id {
        merged_obj.insert("accountId".to_string(), serde_json::json!(summoner_id));
    }
    merged_obj.insert("itemSets".to_string(), serde_json::Value::Array(merged_sets));

    Ok((serde_json::Value::Object(merged_obj), true))
}

/// Abstraction over LCU item-set storage for zero-I/O read-modify-write unit tests.
pub trait ItemSetsStore: Send + Sync {
    fn get_item_sets(
        &self,
        summoner_id: u64,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;

    fn put_item_sets(
        &self,
        summoner_id: u64,
        payload: &serde_json::Value,
    ) -> impl std::future::Future<Output = Result<(), String>> + Send;
}

/// Fetches existing item sets, merges managed `" | schumaz.gg"` sets while keeping player-created
/// sets untouched, and issues `PUT` only when the resulting payload differs.
pub async fn sync_item_sets_with_store(
    store: &impl ItemSetsStore,
    summoner_id: u64,
    builds: &[&ChampionBuild],
) -> Result<bool, String> {
    if builds.is_empty() {
        return Ok(false);
    }

    let existing = store.get_item_sets(summoner_id).await?;
    let (merged, changed) = merge_item_sets_payload(summoner_id, &existing, builds)?;
    if !changed {
        logging::log_debug(
            "lcu.injector",
            format!("event=item_sets_unchanged_skip_put builds={}", builds.len()),
        );
        return Ok(false);
    }

    store.put_item_sets(summoner_id, &merged).await?;
    Ok(true)
}

#[cfg(test)]
#[path = "item_sets_tests.rs"]
mod tests;

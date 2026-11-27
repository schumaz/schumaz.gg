/// Builds the LCU item sets endpoint for a given `summoner_id`.
pub fn build_item_sets_endpoint(summoner_id: u64) -> String {
    format!("/lol-item-sets/v1/item-sets/{summoner_id}/sets")
}

/// Returns `true` when an LCU JSON payload represents an HTTP/domain error object.
pub fn is_lcu_error_payload(val: &serde_json::Value) -> bool {
    val.get("errorCode").is_some()
        || val
            .get("httpStatus")
            .and_then(|s| s.as_i64())
            .is_some_and(|s| s >= 400)
}

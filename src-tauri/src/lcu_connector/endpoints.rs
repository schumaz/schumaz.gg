pub mod gameflow;
pub mod missions;
pub mod perks_items;
pub mod summoner;

pub use gameflow::*;
pub use missions::*;
pub use perks_items::build_item_sets_endpoint;
pub use summoner::*;

pub(crate) fn is_lcu_error_payload(val: &serde_json::Value) -> bool {
    val.get("errorCode").is_some()
        || val
            .get("httpStatus")
            .and_then(|s| s.as_i64())
            .is_some_and(|s| s >= 400)
}

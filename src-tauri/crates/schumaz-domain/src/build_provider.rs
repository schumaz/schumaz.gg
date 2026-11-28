pub mod analytics;
pub mod cache;
pub mod ddragon;
pub mod fallback;
pub mod items;
pub mod models;
pub mod patch;
pub mod query;
pub mod runes;
pub mod service;
pub mod service_entry_points;
pub mod service_helpers;
pub mod skills;
pub mod spells;

pub use analytics::*;
pub use cache::*;
pub use ddragon::*;
pub use fallback::*;
pub use items::*;
pub use models::*;
pub use patch::*;
pub use query::*;
pub use runes::*;
pub use service::*;
pub use service_entry_points::*;
pub use skills::*;
pub use spells::*;

/// Invalidates every L1 RAM cache owned by `build_provider` (builds, tier lists, DataDragon, and item classifications).
pub fn clear_all_ram_caches() {
    cache::build_provider_caches().clear_all();
    ddragon::clear_ddragon_ram_cache();
    items::clear_dynamic_item_classifications();
}

mod augments;
pub mod cache;
pub mod enrichment;
mod fetch;
pub mod item_parser;
pub mod metadata;
mod parsers;

pub use augments::*;
pub use cache::*;
pub use enrichment::*;
pub use metadata::*;
pub use parsers::*;

use crate::models::AppLocale;

/// Non-blocking lookup of the cached DataDragon version for `locale` (RAM store -> `.lz4` header).
pub fn peek_cached_ddragon_version(locale: AppLocale) -> Option<String> {
    if let Ok(guard) = cache::get_ddragon_cache_store().read() {
        if let Some(entry) = guard.get(&locale) {
            return Some(entry.version.clone());
        }
        if let Some(any_entry) = guard.values().next() {
            return Some(any_entry.version.clone());
        }
    }
    let paths = crate::storage::global_paths()?;
    let path = cache::ddragon_metadata_file_path(&paths.ddragon_dir, locale);
    let (_created_at, patch) = crate::storage::read_lz4_envelope_meta(&path)?;
    let version = patch.trim();
    (!version.is_empty()).then(|| version.to_string())
}

#[cfg(test)]
#[path = "ddragon/cache_tests.rs"]
mod cache_tests;
#[cfg(test)]
#[path = "ddragon/enrichment_tests.rs"]
mod enrichment_tests;
#[cfg(test)]
#[path = "ddragon/parsers_tests.rs"]
mod parsers_tests;

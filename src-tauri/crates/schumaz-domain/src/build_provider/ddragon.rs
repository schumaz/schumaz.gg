mod augment_interpolate;
mod augments;
pub mod cache;
mod catalog_parsers;
pub mod enrichment;
mod fetch;
pub mod item_parser;
pub mod metadata;
mod parsers;
mod patch_state;
pub mod tooltip_parser;

pub(crate) use augment_interpolate::{has_raw_riot_augment_markup, strip_riot_inline_markup};
pub use augments::*;
pub use cache::*;
pub use catalog_parsers::*;
pub use enrichment::*;
pub use metadata::*;
pub use parsers::*;
pub use tooltip_parser::*;

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
#[cfg(test)]
#[path = "ddragon/tooltip_parser_tests.rs"]
mod tooltip_parser_tests;

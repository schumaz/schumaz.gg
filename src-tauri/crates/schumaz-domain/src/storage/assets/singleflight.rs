use std::path::PathBuf;
use std::sync::OnceLock;

use bytes::Bytes;

pub use crate::storage::singleflight::{FlightStep, Singleflight};

pub(super) type AssetSharedResult = Result<(Bytes, &'static str), u16>;
pub(super) type SingleflightKey = (PathBuf, String);
pub type AssetSingleflight = Singleflight<SingleflightKey, AssetSharedResult>;

static ASSET_SINGLEFLIGHT: OnceLock<AssetSingleflight> = OnceLock::new();

pub(super) fn global_singleflight() -> &'static AssetSingleflight {
    ASSET_SINGLEFLIGHT.get_or_init(AssetSingleflight::new)
}

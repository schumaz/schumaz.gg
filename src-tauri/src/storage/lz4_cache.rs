use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use super::paths::{atomic_write, now_unix_secs, touch_mtime};

pub const CACHE_SCHEMA_VERSION: u32 = 11;
pub const OPGG_CACHE_TTL_SECS: u64 = 48 * 3600;
/// Shorter disk TTL (3h) for patch-version lists (`versions_{region}.lz4` and `arena/versions.lz4`)
/// so app restarts detect new patches quickly (< 500 bytes) and invalidate stale builds immediately.
pub const VERSIONS_DISK_TTL_SECS: u64 = 3 * 3600;
/// Extended disk TTL (72h) for ARAM Mayhem tier list and augments (invalidated immediately on patch change).
pub const ARAM_MAYHEM_DISK_TTL_SECS: u64 = 72 * 3600;
pub const PROFILE_CACHE_TTL_SECS: u64 = 48 * 3600;
pub const PROFILE_SWR_SECS: u64 = 5 * 60;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheEnvelope<T> {
    pub schema_version: u32,
    pub created_at_unix: u64,
    pub patch: String,
    pub payload: T,
}

#[derive(Debug, Deserialize)]
pub(super) struct CacheEnvelopeHeader {
    pub(super) schema_version: u32,
    #[serde(default)]
    pub(super) created_at_unix: u64,
    #[serde(default)]
    pub(super) patch: String,
}

pub fn is_envelope_fresh(created_at_unix: u64, ttl_secs: u64, now_unix: u64) -> bool {
    now_unix.saturating_sub(created_at_unix) < ttl_secs
}

pub fn write_lz4_envelope_at<T: Serialize>(
    path: &Path,
    patch: &str,
    payload: &T,
    created_at_unix: u64,
) -> Result<(), String> {
    let envelope = CacheEnvelope {
        schema_version: CACHE_SCHEMA_VERSION,
        created_at_unix,
        patch: patch.to_string(),
        payload,
    };
    let encoded = rmp_serde::to_vec_named(&envelope)
        .map_err(|e| format!("Failed to serialize MessagePack envelope: {e}"))?;
    let compressed = lz4_flex::compress_prepend_size(&encoded);
    atomic_write(path, &compressed)
}

pub fn write_lz4_envelope<T: Serialize>(
    path: &Path,
    patch: &str,
    payload: &T,
) -> Result<(), String> {
    write_lz4_envelope_at(path, patch, payload, now_unix_secs())
}

pub fn read_lz4_envelope<T: DeserializeOwned>(path: &Path) -> Option<CacheEnvelope<T>> {
    let bytes = fs::read(path).ok()?;
    let Ok(decompressed) = lz4_flex::decompress_size_prepended(&bytes) else {
        let _ = fs::remove_file(path);
        return None;
    };
    let Ok(envelope) = rmp_serde::from_slice::<CacheEnvelope<T>>(&decompressed) else {
        let _ = fs::remove_file(path);
        return None;
    };
    if envelope.schema_version != CACHE_SCHEMA_VERSION {
        let _ = fs::remove_file(path);
        return None;
    }
    touch_mtime(path);
    Some(envelope)
}

pub(super) fn read_lz4_envelope_header(path: &Path) -> Option<CacheEnvelopeHeader> {
    let bytes = fs::read(path).ok()?;
    let decompressed = lz4_flex::decompress_size_prepended(&bytes).ok()?;
    rmp_serde::from_slice::<CacheEnvelopeHeader>(&decompressed).ok()
}

/// Reads `(created_at_unix, patch)` from a valid `.lz4` cache envelope header without touching `mtime`.
pub fn read_lz4_envelope_meta(path: &Path) -> Option<(u64, String)> {
    let header = read_lz4_envelope_header(path)?;
    if header.schema_version != CACHE_SCHEMA_VERSION {
        return None;
    }
    Some((header.created_at_unix, header.patch))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::paths::{AppPaths, next_test_seq};

    fn temp_test_paths(suffix: &str) -> AppPaths {
        let dir = std::env::temp_dir().join(format!(
            "schumazgg_lz4_test_{}_{}_{}",
            std::process::id(),
            suffix,
            next_test_seq()
        ));
        let _ = fs::remove_dir_all(&dir);
        let paths = AppPaths::from_root(dir);
        paths.ensure_dirs().unwrap();
        paths
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct DummyPayload {
        name: String,
        values: Vec<i32>,
    }

    #[test]
    fn test_lz4_envelope_roundtrip_and_corrupt_or_old_schema_discard() {
        let paths = temp_test_paths("envelope");
        let file_path = paths.opgg_builds_dir.join("ahri.lz4");

        let payload = DummyPayload {
            name: "Ahri".to_string(),
            values: vec![1, 2, 3, 4],
        };
        write_lz4_envelope_at(&file_path, "16.22", &payload, 1_700_000_000).unwrap();

        let loaded: CacheEnvelope<DummyPayload> = read_lz4_envelope(&file_path).unwrap();
        assert_eq!(loaded.schema_version, CACHE_SCHEMA_VERSION);
        assert_eq!(loaded.created_at_unix, 1_700_000_000);
        assert_eq!(loaded.patch, "16.22");
        assert_eq!(loaded.payload, payload);

        let header = read_lz4_envelope_header(&file_path).unwrap();
        assert_eq!(header.schema_version, CACHE_SCHEMA_VERSION);
        assert_eq!(header.patch, "16.22");
        assert_eq!(
            read_lz4_envelope_meta(&file_path),
            Some((1_700_000_000, "16.22".to_string()))
        );

        // Old schema version is silently discarded
        let old_env = CacheEnvelope {
            schema_version: CACHE_SCHEMA_VERSION + 99,
            created_at_unix: 1_700_000_000,
            patch: "16.22".to_string(),
            payload: payload.clone(),
        };
        let encoded = rmp_serde::to_vec_named(&old_env).unwrap();
        let compressed = lz4_flex::compress_prepend_size(&encoded);
        atomic_write(&file_path, &compressed).unwrap();
        assert!(read_lz4_envelope::<DummyPayload>(&file_path).is_none());
        assert!(!file_path.exists());

        // Corrupt bytes are silently discarded
        atomic_write(&file_path, b"not-valid-lz4-data").unwrap();
        assert!(read_lz4_envelope::<DummyPayload>(&file_path).is_none());
        assert!(!file_path.exists());

        let _ = fs::remove_dir_all(&paths.root);
    }
}

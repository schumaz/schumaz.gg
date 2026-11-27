use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::time::SystemTime;

use super::paths::{atomic_write, force_touch_mtime_at, now_unix_secs, should_touch_mtime};

pub const CACHE_SCHEMA_VERSION: u32 = 13;
pub const LZ4_ENVELOPE_HEADER_LEN: usize = 64;
const LZ4_HEADER_MAGIC: [u8; 4] = *b"SLZ4";
const LZ4_MAX_PATCH_BYTES: usize = 47;
const MAX_LZ4_UNCOMPRESSED_BYTES: usize = 64 * 1024 * 1024;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CacheEnvelopeHeader {
    pub(super) schema_version: u32,
    pub(super) created_at_unix: u64,
    pub(super) patch: String,
}

fn clamp_utf8_bytes(s: &str, max_bytes: usize) -> &[u8] {
    let mut end = s.len().min(max_bytes);
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s.as_bytes()[..end]
}

fn encode_lz4_header(
    schema_version: u32,
    created_at_unix: u64,
    patch: &str,
) -> [u8; LZ4_ENVELOPE_HEADER_LEN] {
    let mut hdr = [0u8; LZ4_ENVELOPE_HEADER_LEN];
    hdr[0..4].copy_from_slice(&LZ4_HEADER_MAGIC);
    hdr[4..8].copy_from_slice(&schema_version.to_le_bytes());
    hdr[8..16].copy_from_slice(&created_at_unix.to_le_bytes());
    let patch_bytes = clamp_utf8_bytes(patch, LZ4_MAX_PATCH_BYTES);
    hdr[16] = patch_bytes.len() as u8;
    hdr[17..17 + patch_bytes.len()].copy_from_slice(patch_bytes);
    hdr
}

fn decode_lz4_header(hdr: &[u8; LZ4_ENVELOPE_HEADER_LEN]) -> Option<CacheEnvelopeHeader> {
    if hdr[0..4] != LZ4_HEADER_MAGIC {
        return None;
    }
    let schema_version = u32::from_le_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]);
    let created_at_unix = u64::from_le_bytes([
        hdr[8], hdr[9], hdr[10], hdr[11], hdr[12], hdr[13], hdr[14], hdr[15],
    ]);
    let patch_len = hdr[16] as usize;
    if patch_len > LZ4_MAX_PATCH_BYTES {
        return None;
    }
    if hdr[17 + patch_len..].iter().any(|&b| b != 0) {
        return None;
    }
    let patch = std::str::from_utf8(&hdr[17..17 + patch_len])
        .ok()?
        .to_string();
    Some(CacheEnvelopeHeader {
        schema_version,
        created_at_unix,
        patch,
    })
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
    let encoded = rmp_serde::to_vec_named(payload)
        .map_err(|e| format!("Failed to serialize MessagePack payload: {e}"))?;
    if encoded.is_empty() || encoded.len() > MAX_LZ4_UNCOMPRESSED_BYTES {
        return Err(format!(
            "Invalid MessagePack payload length {} (must be 1..={MAX_LZ4_UNCOMPRESSED_BYTES})",
            encoded.len()
        ));
    }

    let max_compressed = lz4_flex::block::get_maximum_output_size(encoded.len());
    let mut out = vec![0u8; LZ4_ENVELOPE_HEADER_LEN + 4 + max_compressed];
    let header = encode_lz4_header(CACHE_SCHEMA_VERSION, created_at_unix, patch);
    out[..LZ4_ENVELOPE_HEADER_LEN].copy_from_slice(&header);
    out[LZ4_ENVELOPE_HEADER_LEN..LZ4_ENVELOPE_HEADER_LEN + 4]
        .copy_from_slice(&(encoded.len() as u32).to_le_bytes());

    let written =
        lz4_flex::block::compress_into(&encoded, &mut out[LZ4_ENVELOPE_HEADER_LEN + 4..])
            .map_err(|e| format!("Failed to LZ4-compress payload: {e}"))?;
    out.truncate(LZ4_ENVELOPE_HEADER_LEN + 4 + written);
    atomic_write(path, &out)
}

pub fn write_lz4_envelope<T: Serialize>(
    path: &Path,
    patch: &str,
    payload: &T,
) -> Result<(), String> {
    write_lz4_envelope_at(path, patch, payload, now_unix_secs())
}

pub fn read_lz4_envelope<T: DeserializeOwned>(path: &Path) -> Option<CacheEnvelope<T>> {
    let mut file = fs::File::open(path).ok()?;
    let meta = file.metadata().ok();
    let mut bytes = Vec::with_capacity(
        meta.as_ref()
            .map(|m| m.len() as usize)
            .unwrap_or(0)
            .min(MAX_LZ4_UNCOMPRESSED_BYTES),
    );
    if file.read_to_end(&mut bytes).is_err() {
        return None;
    }
    drop(file);

    if bytes.len() < LZ4_ENVELOPE_HEADER_LEN + 5 {
        let _ = fs::remove_file(path);
        return None;
    }
    let Ok(hdr) = bytes[..LZ4_ENVELOPE_HEADER_LEN].try_into() else {
        let _ = fs::remove_file(path);
        return None;
    };
    let Some(header) = decode_lz4_header(&hdr) else {
        let _ = fs::remove_file(path);
        return None;
    };
    if header.schema_version != CACHE_SCHEMA_VERSION {
        let _ = fs::remove_file(path);
        return None;
    }
    let uncompressed_len =
        u32::from_le_bytes([bytes[64], bytes[65], bytes[66], bytes[67]]) as usize;
    if uncompressed_len == 0 || uncompressed_len > MAX_LZ4_UNCOMPRESSED_BYTES {
        let _ = fs::remove_file(path);
        return None;
    }
    let Ok(decompressed) = lz4_flex::decompress_size_prepended(&bytes[LZ4_ENVELOPE_HEADER_LEN..])
    else {
        let _ = fs::remove_file(path);
        return None;
    };
    let Ok(payload) = rmp_serde::from_slice::<T>(&decompressed) else {
        let _ = fs::remove_file(path);
        return None;
    };

    if let Some(modified) = meta.and_then(|m| m.modified().ok()) {
        let now = SystemTime::now();
        if should_touch_mtime(modified, now) {
            force_touch_mtime_at(path, now);
        }
    }

    Some(CacheEnvelope {
        schema_version: header.schema_version,
        created_at_unix: header.created_at_unix,
        patch: header.patch,
        payload,
    })
}

/// Reads only the fixed 64-byte uncompressed binary header + 4-byte uncompressed size prefix in O(1),
/// never reading or decompressing the LZ4 payload.
pub(super) fn read_lz4_envelope_header(path: &Path) -> Option<CacheEnvelopeHeader> {
    let mut file = fs::File::open(path).ok()?;
    let mut probe = [0u8; LZ4_ENVELOPE_HEADER_LEN + 5];
    file.read_exact(&mut probe).ok()?;
    let uncompressed_len = u32::from_le_bytes([probe[64], probe[65], probe[66], probe[67]]) as usize;
    if uncompressed_len == 0 || uncompressed_len > MAX_LZ4_UNCOMPRESSED_BYTES {
        return None;
    }
    let hdr: [u8; LZ4_ENVELOPE_HEADER_LEN] = probe[..LZ4_ENVELOPE_HEADER_LEN].try_into().ok()?;
    decode_lz4_header(&hdr)
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

        // O(1) header reader works without decompressing the LZ4 payload body.
        let header_only_path = paths.opgg_builds_dir.join("header_only.lz4");
        let mut fake_file = vec![0u8; LZ4_ENVELOPE_HEADER_LEN + 10];
        fake_file[..LZ4_ENVELOPE_HEADER_LEN].copy_from_slice(&encode_lz4_header(
            CACHE_SCHEMA_VERSION,
            1_700_000_123,
            "16.23",
        ));
        fake_file[64..68].copy_from_slice(&128u32.to_le_bytes());
        atomic_write(&header_only_path, &fake_file).unwrap();
        assert_eq!(
            read_lz4_envelope_meta(&header_only_path),
            Some((1_700_000_123, "16.23".to_string()))
        );

        // Old schema version in header is silently discarded without decompressing
        let mut old_bytes = fs::read(&file_path).unwrap();
        old_bytes[..LZ4_ENVELOPE_HEADER_LEN].copy_from_slice(&encode_lz4_header(
            CACHE_SCHEMA_VERSION + 99,
            1_700_000_000,
            "16.22",
        ));
        atomic_write(&file_path, &old_bytes).unwrap();
        assert!(read_lz4_envelope::<DummyPayload>(&file_path).is_none());
        assert!(!file_path.exists());

        // Legacy headerless or corrupt bytes are silently discarded
        atomic_write(&file_path, b"not-valid-lz4-data").unwrap();
        assert!(read_lz4_envelope::<DummyPayload>(&file_path).is_none());
        assert!(!file_path.exists());

        let _ = fs::remove_dir_all(&paths.root);
    }
}

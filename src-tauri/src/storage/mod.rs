pub mod assets;
pub mod settings;

pub use assets::*;
pub use settings::*;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;

pub const CACHE_SCHEMA_VERSION: u32 = 1;
pub const OPGG_CACHE_TTL_SECS: u64 = 24 * 3600;
pub const PROFILE_CACHE_TTL_SECS: u64 = 24 * 3600;
pub const PROFILE_SWR_SECS: u64 = 5 * 60;
pub const GC_MAX_INACTIVE_SECS: u64 = 7 * 24 * 3600;

static TEMP_FILE_SEQ: AtomicU64 = AtomicU64::new(1);
static GLOBAL_APP_PATHS: OnceLock<AppPaths> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub root: PathBuf,
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub ddragon_dir: PathBuf,
    pub opgg_dir: PathBuf,
    pub opgg_versions_dir: PathBuf,
    pub opgg_positions_dir: PathBuf,
    pub opgg_builds_dir: PathBuf,
    pub opgg_ranked_dir: PathBuf,
    pub opgg_aram_dir: PathBuf,
    pub profile_dir: PathBuf,
    pub assets_dir: PathBuf,
}

impl AppPaths {
    pub fn from_root(root: PathBuf) -> Self {
        let config_dir = root.join("Config");
        let cache_dir = root.join("Cache");
        let ddragon_dir = cache_dir.join("ddragon");
        let opgg_dir = cache_dir.join("opgg");
        let opgg_versions_dir = opgg_dir.join("versions");
        let opgg_positions_dir = opgg_dir.join("positions");
        let opgg_builds_dir = opgg_dir.join("builds");
        let opgg_ranked_dir = opgg_dir.join("ranked");
        let opgg_aram_dir = opgg_dir.join("aram");
        let profile_dir = cache_dir.join("profile");
        let assets_dir = cache_dir.join("assets");

        Self {
            root,
            config_dir,
            cache_dir,
            ddragon_dir,
            opgg_dir,
            opgg_versions_dir,
            opgg_positions_dir,
            opgg_builds_dir,
            opgg_ranked_dir,
            opgg_aram_dir,
            profile_dir,
            assets_dir,
        }
    }

    /// Official Windows directory: `%LOCALAPPDATA%\schumazgg\`
    /// Uses `local_data_dir()?.join("schumazgg")` rather than `app_local_data_dir()`.
    pub fn resolve_from_app(app: &tauri::AppHandle) -> Result<Self, String> {
        let local_data = app
            .path()
            .local_data_dir()
            .map_err(|e| format!("Failed to resolve local_data_dir: {e}"))?;
        Ok(Self::from_root(local_data.join("schumazgg")))
    }

    pub fn ensure_dirs(&self) -> Result<(), String> {
        for dir in [
            &self.root,
            &self.config_dir,
            &self.cache_dir,
            &self.ddragon_dir,
            &self.opgg_dir,
            &self.opgg_versions_dir,
            &self.opgg_positions_dir,
            &self.opgg_builds_dir,
            &self.opgg_ranked_dir,
            &self.opgg_aram_dir,
            &self.profile_dir,
            &self.assets_dir,
        ] {
            fs::create_dir_all(dir)
                .map_err(|e| format!("Failed to create directory {}: {e}", dir.display()))?;
        }
        Ok(())
    }

    pub fn user_settings_ini_path(&self) -> PathBuf {
        self.config_dir.join("UserSettings.ini")
    }
}

pub fn set_global_paths(paths: AppPaths) {
    let _ = GLOBAL_APP_PATHS.set(paths);
}

pub fn global_paths() -> Option<&'static AppPaths> {
    GLOBAL_APP_PATHS.get()
}

pub fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn fnv1a_64(input: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in input.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub fn fnv1a_128_hex(input: &str) -> String {
    let h1 = fnv1a_64(input);
    let mut h2: u64 = 0x9e3779b97f4a7c15;
    for b in input.as_bytes().iter().rev() {
        h2 ^= *b as u64;
        h2 = h2.wrapping_mul(0xbf58476d1ce4e5b9);
    }
    format!("{h1:016x}{h2:016x}")
}

pub fn sanitize_key_component(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len().min(64));
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.' {
            out.push(ch);
        } else {
            out.push('_');
        }
        if out.len() >= 64 {
            break;
        }
    }
    if out.is_empty() {
        "default".to_string()
    } else {
        out
    }
}

/// Atomically writes `bytes` to `path` via a unique `.tmp` sibling file followed by `rename`.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent dir {}: {e}", parent.display()))?;
    }

    let seq = TEMP_FILE_SEQ.fetch_add(1, Ordering::Relaxed);
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("cache");
    let tmp_name = format!("{file_name}.{}.{seq}.tmp", std::process::id());
    let tmp_path = path.with_file_name(tmp_name);

    {
        let mut file = fs::File::create(&tmp_path)
            .map_err(|e| format!("Failed to create temp file {}: {e}", tmp_path.display()))?;
        if let Err(e) = file.write_all(bytes) {
            let _ = fs::remove_file(&tmp_path);
            return Err(format!(
                "Failed to write temp file {}: {e}",
                tmp_path.display()
            ));
        }
        let _ = file.flush();
    }

    if let Err(e) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(format!(
            "Failed to rename {} to {}: {e}",
            tmp_path.display(),
            path.display()
        ));
    }

    Ok(())
}

/// Updates the file's modification timestamp (`mtime`) without relying on NTFS `atime`.
pub fn touch_mtime(path: &Path) {
    if let Ok(file) = fs::OpenOptions::new().write(true).open(path) {
        let _ = file.set_modified(SystemTime::now());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheEnvelope<T> {
    pub schema_version: u32,
    pub created_at_unix: u64,
    pub patch: String,
    pub payload: T,
}

#[derive(Debug, Deserialize)]
struct CacheEnvelopeHeader {
    schema_version: u32,
    #[serde(default)]
    patch: String,
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

fn read_lz4_envelope_header(path: &Path) -> Option<CacheEnvelopeHeader> {
    let bytes = fs::read(path).ok()?;
    let decompressed = lz4_flex::decompress_size_prepended(&bytes).ok()?;
    rmp_serde::from_slice::<CacheEnvelopeHeader>(&decompressed).ok()
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GcStats {
    pub removed_tmp_files: usize,
    pub removed_stale_patch_files: usize,
    pub removed_inactive_files: usize,
    pub removed_corrupt_files: usize,
}

fn collect_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

/// Scans `Cache\` (and `Config\` for orphan `.tmp` files), removing:
/// - Orphan `.tmp` files
/// - `.lz4` files under `Cache\opgg\` whose envelope `patch` is non-empty and not in `active_patches`
/// - Corrupt or outdated-schema `.lz4` files
/// - `.lz4` or cached asset files whose `mtime` has been inactive for > 7 days
pub fn run_garbage_collector(
    paths: &AppPaths,
    active_patches: &[String],
    now_unix: u64,
) -> GcStats {
    let mut stats = GcStats::default();
    let mut all_files = Vec::new();
    collect_files_recursive(&paths.config_dir, &mut all_files);
    collect_files_recursive(&paths.cache_dir, &mut all_files);

    for path in all_files {
        let is_tmp = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("tmp"));
        if is_tmp {
            if fs::remove_file(&path).is_ok() {
                stats.removed_tmp_files += 1;
            }
            continue;
        }

        if !path.starts_with(&paths.cache_dir) {
            continue;
        }

        let is_lz4 = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("lz4"));

        if is_lz4 {
            match read_lz4_envelope_header(&path) {
                Some(header) => {
                    if header.schema_version != CACHE_SCHEMA_VERSION {
                        if fs::remove_file(&path).is_ok() {
                            stats.removed_corrupt_files += 1;
                        }
                        continue;
                    }
                    if path.starts_with(&paths.opgg_dir)
                        && !active_patches.is_empty()
                        && !header.patch.trim().is_empty()
                        && !active_patches.iter().any(|p| p == &header.patch)
                    {
                        if fs::remove_file(&path).is_ok() {
                            stats.removed_stale_patch_files += 1;
                        }
                        continue;
                    }
                }
                None => {
                    if fs::remove_file(&path).is_ok() {
                        stats.removed_corrupt_files += 1;
                    }
                    continue;
                }
            }
        }

        if let Ok(meta) = fs::metadata(&path)
            && let Ok(modified) = meta.modified()
            && let Ok(dur) = modified.duration_since(UNIX_EPOCH)
        {
            let mtime_unix = dur.as_secs();
            if now_unix.saturating_sub(mtime_unix) > GC_MAX_INACTIVE_SECS
                && fs::remove_file(&path).is_ok()
            {
                stats.removed_inactive_files += 1;
            }
        }
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_test_paths(suffix: &str) -> AppPaths {
        let dir = std::env::temp_dir().join(format!(
            "schumazgg_storage_test_{}_{}_{}",
            std::process::id(),
            suffix,
            TEMP_FILE_SEQ.fetch_add(1, Ordering::Relaxed)
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

    #[test]
    fn test_garbage_collector_cleans_tmp_old_patches_corrupt_and_inactive_files() {
        let paths = temp_test_paths("gc");
        let now = now_unix_secs();

        // 1. Orphan .tmp files
        let tmp_cfg = paths.config_dir.join("UserSettings.ini.123.tmp");
        let tmp_cache = paths.opgg_builds_dir.join("build.lz4.456.tmp");
        atomic_write(&tmp_cfg, b"tmp").unwrap();
        atomic_write(&tmp_cache, b"tmp").unwrap();

        // 2. Old patch build file (patch 16.19 not in active ["16.22", "16.21"])
        let old_patch_file = paths.opgg_builds_dir.join("old_patch.lz4");
        write_lz4_envelope_at(&old_patch_file, "16.19", &vec![1, 2], now).unwrap();

        // 3. Active patch build file (should stay)
        let active_patch_file = paths.opgg_builds_dir.join("active_patch.lz4");
        write_lz4_envelope_at(&active_patch_file, "16.22", &vec![3, 4], now).unwrap();

        // 4. Corrupt .lz4
        let corrupt_file = paths.profile_dir.join("corrupt.lz4");
        atomic_write(&corrupt_file, b"broken").unwrap();

        // 5. Inactive asset > 7 days (mtime backdated 8 days)
        let old_asset = paths.assets_dir.join("old_splash.jpg");
        atomic_write(&old_asset, b"jpegdata").unwrap();
        {
            let file = fs::OpenOptions::new().write(true).open(&old_asset).unwrap();
            let eight_days_ago = SystemTime::now() - std::time::Duration::from_secs(8 * 24 * 3600);
            file.set_modified(eight_days_ago).unwrap();
        }

        // 6. Recently hit asset (touch_mtime keeps it alive)
        let fresh_asset = paths.assets_dir.join("fresh_splash.jpg");
        atomic_write(&fresh_asset, b"jpegdata").unwrap();
        {
            let file = fs::OpenOptions::new()
                .write(true)
                .open(&fresh_asset)
                .unwrap();
            let eight_days_ago = SystemTime::now() - std::time::Duration::from_secs(8 * 24 * 3600);
            file.set_modified(eight_days_ago).unwrap();
        }
        touch_mtime(&fresh_asset);

        let stats = run_garbage_collector(&paths, &["16.22".to_string(), "16.21".to_string()], now);

        assert_eq!(stats.removed_tmp_files, 2);
        assert_eq!(stats.removed_stale_patch_files, 1);
        assert_eq!(stats.removed_corrupt_files, 1);
        assert_eq!(stats.removed_inactive_files, 1);
        assert!(!tmp_cfg.exists());
        assert!(!tmp_cache.exists());
        assert!(!old_patch_file.exists());
        assert!(active_patch_file.exists());
        assert!(!corrupt_file.exists());
        assert!(!old_asset.exists());
        assert!(fresh_asset.exists());

        let _ = fs::remove_dir_all(&paths.root);
    }
}

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;

static TEMP_FILE_SEQ: AtomicU64 = AtomicU64::new(1);
static GLOBAL_APP_PATHS: OnceLock<AppPaths> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub root: PathBuf,
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub webview_dir: PathBuf,
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
        let webview_dir = root.join("WebView");
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
            webview_dir,
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
            &self.webview_dir,
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

#[cfg(test)]
pub(super) fn next_test_seq() -> u64 {
    TEMP_FILE_SEQ.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_paths_centralizes_config_cache_and_webview_under_root() {
        let root = std::env::temp_dir().join(format!(
            "schumazgg_paths_test_{}_{}",
            std::process::id(),
            next_test_seq()
        ));
        let _ = fs::remove_dir_all(&root);

        let paths = AppPaths::from_root(root.clone());
        assert_eq!(paths.root, root);
        assert_eq!(paths.config_dir, root.join("Config"));
        assert_eq!(paths.cache_dir, root.join("Cache"));
        assert_eq!(paths.webview_dir, root.join("WebView"));
        assert_eq!(paths.ddragon_dir, root.join("Cache").join("ddragon"));
        assert_eq!(paths.opgg_dir, root.join("Cache").join("opgg"));
        assert_eq!(paths.profile_dir, root.join("Cache").join("profile"));
        assert_eq!(paths.assets_dir, root.join("Cache").join("assets"));
        assert_eq!(
            paths.user_settings_ini_path(),
            root.join("Config").join("UserSettings.ini")
        );

        paths.ensure_dirs().unwrap();
        assert!(paths.config_dir.is_dir());
        assert!(paths.cache_dir.is_dir());
        assert!(paths.webview_dir.is_dir());
        assert!(paths.ddragon_dir.is_dir());
        assert!(paths.opgg_dir.is_dir());
        assert!(paths.profile_dir.is_dir());
        assert!(paths.assets_dir.is_dir());

        let _ = fs::remove_dir_all(&root);
    }
}

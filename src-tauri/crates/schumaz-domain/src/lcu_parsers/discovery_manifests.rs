use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::logging;
use crate::storage::atomic_write;

/// Normalizes path separators and trailing slashes for case-insensitive deduplication on Windows.
pub fn normalize_path_key(path: &Path) -> String {
    let raw = path.to_string_lossy();
    #[cfg(target_os = "windows")]
    {
        raw.replace('/', "\\")
            .trim_end_matches('\\')
            .to_ascii_lowercase()
    }
    #[cfg(not(target_os = "windows"))]
    {
        raw.replace('\\', "/").trim_end_matches('/').to_string()
    }
}

fn dir_to_lockfile_path(raw_dir: &str) -> Option<PathBuf> {
    let trimmed = raw_dir.trim().trim_matches('"').trim_matches('\'');
    let clean = trimmed.trim_end_matches(['/', '\\']);
    if clean.is_empty() {
        return None;
    }
    #[cfg(target_os = "windows")]
    let normalized = clean.replace('/', "\\");
    #[cfg(not(target_os = "windows"))]
    let normalized = clean.replace('\\', "/");

    Some(PathBuf::from(normalized).join("lockfile"))
}

fn push_unique_candidate(
    candidates: &mut Vec<PathBuf>,
    seen: &mut HashSet<String>,
    skip_key: Option<&str>,
    candidate: PathBuf,
) {
    let key = normalize_path_key(&candidate);
    if key.is_empty() || skip_key == Some(key.as_str()) {
        return;
    }
    if seen.insert(key) {
        candidates.push(candidate);
    }
}

/// Reads the persisted last-known valid `lockfile` path from disk.
pub fn read_persisted_lockfile_path(cache_file: &Path) -> Option<PathBuf> {
    let contents = std::fs::read_to_string(cache_file).ok()?;
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(PathBuf::from(trimmed))
}

/// Atomically persists the last valid `lockfile` path to disk for future cold starts.
pub fn persist_lockfile_path(cache_file: &Path, lockfile_path: &Path) {
    #[cfg(target_os = "windows")]
    let serialized = lockfile_path.to_string_lossy().replace('/', "\\");
    #[cfg(not(target_os = "windows"))]
    let serialized = lockfile_path.to_string_lossy().to_string();

    if let Err(err) = atomic_write(cache_file, serialized.as_bytes()) {
        logging::log_debug(
            "lcu.discovery",
            format!("event=persist_lockfile_path_failed err={err}"),
        );
    }
}

/// Parses `RiotClientInstalls.json` into ordered candidate `lockfile` paths (Live League first).
pub fn parse_riot_client_installs_json(raw: &str) -> Vec<PathBuf> {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Vec::new();
    };

    let mut live_league = Vec::new();
    let mut pbe_league = Vec::new();
    let mut other_clients = Vec::new();

    if let Some(map) = val.get("associated_client").and_then(|v| v.as_object()) {
        for raw_dir in map.keys() {
            let Some(lockfile_path) = dir_to_lockfile_path(raw_dir) else {
                continue;
            };
            let lower = raw_dir.to_ascii_lowercase();
            if lower.contains("league") {
                if lower.contains("pbe") {
                    pbe_league.push(lockfile_path);
                } else {
                    live_league.push(lockfile_path);
                }
            } else {
                other_clients.push(lockfile_path);
            }
        }
    }

    let mut rc_fallbacks = Vec::new();
    for field in ["rc_live", "rc_default"] {
        if let Some(rc_exe_raw) = val.get(field).and_then(|v| v.as_str())
            && let Some(rc_dir) = Path::new(rc_exe_raw.trim()).parent()
            && let Some(riot_root) = rc_dir.parent()
            && let Some(candidate) = dir_to_lockfile_path(
                &riot_root.join("League of Legends").to_string_lossy(),
            )
        {
            rc_fallbacks.push(candidate);
        }
    }

    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    for path in live_league
        .into_iter()
        .chain(pbe_league)
        .chain(other_clients)
        .chain(rc_fallbacks)
    {
        push_unique_candidate(&mut candidates, &mut seen, None, path);
    }
    candidates
}

/// Extracts `product_install_full_path` from Riot's `*.product_settings.yaml` manifest.
pub fn parse_product_settings_yaml(raw: &str) -> Option<PathBuf> {
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("product_install_full_path:") {
            return dir_to_lockfile_path(rest);
        }
    }
    None
}

/// Collects candidate `lockfile` paths from Riot manifests under `program_data`, deduplicated
/// and excluding `skip_path` if already checked.
pub fn collect_manifest_lockfile_candidates_from_root(
    program_data: &Path,
    skip_path: Option<&Path>,
) -> Vec<PathBuf> {
    let skip_key = skip_path.map(normalize_path_key);
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    let installs_json_path = program_data.join("Riot Games").join("RiotClientInstalls.json");
    if let Ok(raw) = std::fs::read_to_string(&installs_json_path) {
        for path in parse_riot_client_installs_json(&raw) {
            push_unique_candidate(&mut candidates, &mut seen, skip_key.as_deref(), path);
        }
    }

    for product in ["league_of_legends.live", "league_of_legends.pbe"] {
        let yaml_path = program_data
            .join("Riot Games")
            .join("Metadata")
            .join(product)
            .join(format!("{product}.product_settings.yaml"));
        if let Ok(raw) = std::fs::read_to_string(&yaml_path)
            && let Some(path) = parse_product_settings_yaml(&raw)
        {
            push_unique_candidate(&mut candidates, &mut seen, skip_key.as_deref(), path);
        }
    }

    candidates
}

/// Collects ordered Riot manifest and default-install `lockfile` candidates on this host.
pub fn collect_manifest_lockfile_candidates(skip_path: Option<&Path>) -> Vec<PathBuf> {
    let skip_key = skip_path.map(normalize_path_key);
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    #[cfg(target_os = "windows")]
    let program_data = std::env::var_os("ProgramData")
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from(r"C:\ProgramData")));
    #[cfg(not(target_os = "windows"))]
    let program_data = std::env::var_os("ProgramData").map(PathBuf::from);

    if let Some(pd_root) = program_data {
        for path in collect_manifest_lockfile_candidates_from_root(&pd_root, skip_path) {
            push_unique_candidate(&mut candidates, &mut seen, skip_key.as_deref(), path);
        }
    }

    #[cfg(target_os = "windows")]
    push_unique_candidate(
        &mut candidates,
        &mut seen,
        skip_key.as_deref(),
        PathBuf::from(r"C:\Riot Games\League of Legends\lockfile"),
    );

    candidates
}

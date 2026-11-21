pub mod policy;
pub mod release_probe;

#[cfg(test)]
mod portable_tests;

pub use policy::{
    AUTO_UPDATE_CHECK_COOLDOWN_SECS, extract_release_tag_from_location, is_remote_version_newer,
    normalize_semver_tag, should_run_auto_check,
};
pub use release_probe::{
    ReleaseProbeOutcome, build_no_redirect_client, probe_for_app_update, probe_latest_release_url,
    record_update_check_completed,
};

use std::path::{Path, PathBuf};
use tauri::Manager;

use crate::models::AppDistributionInfo;
use crate::storage;

pub const GITHUB_RELEASES_LATEST_URL: &str =
    "https://github.com/schumaz/schumaz.gg/releases/latest";
pub const PRODUCT_INSTALL_DIR_NAME: &str = "schumazgg";
pub const WINDOWS_UNINSTALL_SUBKEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Uninstall\schumaz.gg";

/// Snapshot of the NSIS `currentUser` Uninstall registry entry on Windows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsUninstallEntry {
    pub install_location: Option<String>,
    pub uninstall_string: Option<String>,
}

fn normalize_windows_like_path(raw: &str) -> String {
    let trimmed = raw.trim().trim_matches('"');
    let without_verbatim = trimmed
        .strip_prefix(r"\\?\")
        .or_else(|| trimmed.strip_prefix("//?/"))
        .unwrap_or(trimmed);
    let unified = without_verbatim.replace('/', "\\");
    unified.trim_end_matches('\\').to_lowercase()
}

/// Returns true when `exe_path` resides directly inside `install_dir` (case-insensitive boundary match).
pub fn is_exe_within_install_dir(exe_path: &Path, install_dir: &Path) -> bool {
    let exe_norm = normalize_windows_like_path(&exe_path.to_string_lossy());
    let dir_norm = normalize_windows_like_path(&install_dir.to_string_lossy());
    if exe_norm.is_empty() || dir_norm.is_empty() {
        return false;
    }
    if exe_norm == dir_norm {
        return true;
    }
    let Some(rest) = exe_norm.strip_prefix(&dir_norm) else {
        return false;
    };
    rest.starts_with('\\')
}

/// Evaluates whether the running binary is in portable mode.
///
/// Returns `true` whenever:
/// - The Cargo `portable` feature is enabled at compile time, OR
/// - On Windows, there is no `HKCU\...\Uninstall\schumaz.gg` entry OR `current_exe()`
///   is located outside both the registered `InstallLocation` and `%LOCALAPPDATA%\schumazgg`.
pub fn evaluate_portable_mode(
    feature_portable: bool,
    is_windows: bool,
    exe_path: Option<&Path>,
    local_app_data: Option<&Path>,
    uninstall_entry: Option<&WindowsUninstallEntry>,
) -> bool {
    if feature_portable {
        return true;
    }

    if !is_windows {
        return false;
    }

    let Some(entry) = uninstall_entry else {
        return true;
    };

    let Some(exe) = exe_path else {
        return true;
    };

    if let Some(install_loc) = entry
        .install_location
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        && is_exe_within_install_dir(exe, Path::new(install_loc))
    {
        return false;
    }

    if let Some(local_data) = local_app_data {
        let default_install_dir = local_data.join(PRODUCT_INSTALL_DIR_NAME);
        if is_exe_within_install_dir(exe, &default_install_dir) {
            return false;
        }
    }

    true
}

#[cfg(windows)]
fn read_windows_uninstall_entry() -> Option<WindowsUninstallEntry> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let subkey = hkcu.open_subkey(WINDOWS_UNINSTALL_SUBKEY).ok()?;
    let install_location: Option<String> = subkey.get_value("InstallLocation").ok();
    let uninstall_string: Option<String> = subkey.get_value("UninstallString").ok();

    if install_location.is_none() && uninstall_string.is_none() {
        return None;
    }

    Some(WindowsUninstallEntry {
        install_location,
        uninstall_string,
    })
}

#[cfg(not(windows))]
fn read_windows_uninstall_entry() -> Option<WindowsUninstallEntry> {
    None
}

/// Detects whether the current process is running as a standalone portable executable.
pub fn detect_is_portable(app: &tauri::AppHandle) -> bool {
    let feature_portable = cfg!(feature = "portable");
    let exe_path = std::env::current_exe().ok();
    let local_app_data: Option<PathBuf> = app
        .path()
        .local_data_dir()
        .ok()
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from));
    let uninstall_entry = read_windows_uninstall_entry();

    evaluate_portable_mode(
        feature_portable,
        cfg!(windows),
        exe_path.as_deref(),
        local_app_data.as_deref(),
        uninstall_entry.as_ref(),
    )
}

/// Collects runtime distribution metadata for the frontend updater flow.
pub fn get_distribution_info(app: &tauri::AppHandle) -> AppDistributionInfo {
    AppDistributionInfo {
        is_portable: detect_is_portable(app),
        current_version: app.package_info().version.to_string(),
        releases_url: GITHUB_RELEASES_LATEST_URL.to_string(),
        last_check_epoch_secs: storage::get_last_update_check_epoch_secs(),
        check_cooldown_secs: AUTO_UPDATE_CHECK_COOLDOWN_SECS,
    }
}

use std::path::Path;

#[cfg(test)]
#[path = "portable_tests.rs"]
mod portable_tests;

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

use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::Manager;

pub use schumaz_domain::updater::*;

static IS_PORTABLE_CACHE: OnceLock<bool> = OnceLock::new();

use crate::models::{AppDistributionInfo, UpdateProbeResult};
use crate::storage;

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

/// Returns true when the binary was compiled with the Cargo `portable` feature.
pub const fn is_compile_time_portable() -> bool {
    cfg!(feature = "portable")
}

/// Detects whether the current process is running as a standalone portable executable,
/// caching the Windows registry check once per process lifetime.
pub fn detect_is_portable(app: &tauri::AppHandle) -> bool {
    *IS_PORTABLE_CACHE.get_or_init(|| {
        let feature_portable = is_compile_time_portable();
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
    })
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

/// Evaluates the 24h persisted cooldown and, when due (or `manual`), performs a lightweight
/// `HEAD /releases/latest` probe before allowing `tauri-plugin-updater` to fetch `latest.json`.
pub async fn probe_for_app_update(app: &tauri::AppHandle, manual: bool) -> UpdateProbeResult {
    let app_for_portable = app.clone();
    let is_portable = tokio::task::spawn_blocking(move || detect_is_portable(&app_for_portable))
        .await
        .unwrap_or_else(|_| detect_is_portable(app));
    let current_version = app.package_info().version.to_string();
    probe_for_app_update_with_context(is_portable, &current_version, manual).await
}

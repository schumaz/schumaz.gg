use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::build_provider;
use crate::lcu_connector;
use crate::logging;
use crate::logging::diagnostics::{DiagnosticInput, build_diagnostic_summary, detect_os_version};
use crate::models::DiagnosticSummary;
use crate::storage::{self, AppPaths, GraphicsState};
use crate::updater;
use crate::ws_listener;

/// Builds a sanitized local diagnostic summary without exposing credentials, paths, or PII.
#[tauri::command]
#[specta::specta]
pub async fn get_diagnostic_summary(
    app_handle: AppHandle,
    graphics: State<'_, GraphicsState>,
) -> Result<DiagnosticSummary, String> {
    let app_version = app_handle.package_info().version.to_string();
    let settings = storage::get_global_settings();
    let active_locale = settings.general.language.unwrap_or_default();
    let gpu_status = graphics.status();
    let lcu_connected = lcu_connector::is_lcu_connected();
    let gameflow_phase = ws_listener::shared_last_gameflow_phase()
        .lock()
        .await
        .clone();
    let ddragon_version = build_provider::ddragon::peek_cached_ddragon_version(active_locale);
    let active_patch = build_provider::analytics::peek_latest_cached_opgg_version("global");
    let metrics = logging::metrics::runtime_metrics_snapshot();
    let in_memory_issues = logging::metrics::recent_issues_snapshot();
    let log_path = logging::active_log_file_path()
        .map(std::path::Path::to_path_buf)
        .or_else(|| storage::global_paths().map(AppPaths::log_file_path));

    tokio::task::spawn_blocking(move || {
        let is_portable = updater::detect_is_portable(&app_handle);
        let input = DiagnosticInput {
            app_version,
            is_portable,
            os_version: detect_os_version(),
            architecture: std::env::consts::ARCH.to_string(),
            locale: active_locale.as_str().to_string(),
            gpu_saved: gpu_status.enabled,
            gpu_active: gpu_status.active_at_startup,
            close_to_tray: settings.general.close_to_tray,
            lcu_connected,
            gameflow_phase,
            ddragon_version,
            active_patch,
            metrics,
            in_memory_issues,
        };
        build_diagnostic_summary(&input, log_path.as_deref())
    })
    .await
    .map_err(|e| format!("Failed to build diagnostic summary: {e}"))
}

/// Ensures `%LOCALAPPDATA%\schumazgg\Logs\` exists, flushes buffered logs, and opens it in Explorer
/// directly from the Rust backend without expanding frontend Tauri capabilities.
#[tauri::command]
#[specta::specta]
pub async fn open_logs_folder(app_handle: AppHandle) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let logs_dir = match storage::global_paths() {
            Some(paths) => paths.logs_dir.clone(),
            None => AppPaths::resolve_from_app(&app_handle)?.logs_dir,
        };
        std::fs::create_dir_all(&logs_dir)
            .map_err(|e| format!("Failed to ensure logs directory: {e}"))?;

        logging::flush_logs();

        app_handle
            .opener()
            .open_path(logs_dir.to_string_lossy().to_string(), None::<&str>)
            .map_err(|e| format!("Failed to open logs folder: {e}"))
    })
    .await
    .map_err(|e| format!("Failed to open logs folder: {e}"))?
}

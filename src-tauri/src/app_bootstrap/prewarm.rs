use std::time::Instant;
use tauri::Manager;

use crate::{build_provider, lcu_connector, logging, models, profile_provider, storage};

use super::BootstrapState;
use super::window_factory::AppLifecycleState;

fn spawn_prewarm_task(
    app_handle: tauri::AppHandle,
    bootstrap_state: BootstrapState,
    paths: storage::AppPaths,
    settings: storage::AppSettings,
    active_locale: models::AppLocale,
) {
    tauri::async_runtime::spawn(async move {
        let started_at = Instant::now();
        let disk_app = app_handle.clone();
        let disk_bootstrap = bootstrap_state.clone();
        let disk_paths = paths.clone();
        let disk_settings = settings;
        let ddragon_opt = tokio::task::spawn_blocking(move || {
            let ddragon_opt = build_provider::ddragon::prewarm_ddragon_from_disk(active_locale);
            disk_bootstrap.update(&disk_app, "ddragon_disk", 40, false, false);

            profile_provider::prewarm_skins_from_disk(active_locale);
            let _ = profile_provider::get_cached_profile_for_last_summoner(active_locale);
            disk_bootstrap.update(&disk_app, "profile_disk", 60, false, false);

            if let Some(ref dd) = ddragon_opt {
                build_provider::analytics::ranked::prewarm_ranked_tier_list_from_dir(
                    &disk_paths.opgg_ranked_dir,
                    &disk_settings.navigation.ranked_filter_region,
                    &disk_settings.navigation.ranked_filter_tier,
                    disk_settings.navigation.ranked_filter_patch.as_deref(),
                    disk_settings.automation.patch_48h_protection,
                    active_locale,
                    dd,
                );
                build_provider::analytics::arena::prewarm_arena_tier_list_from_dir(
                    &disk_paths.opgg_arena_dir,
                    disk_settings.automation.patch_48h_protection,
                    active_locale,
                    dd,
                );
                build_provider::analytics::aram::prewarm_aram_tier_lists_from_dir(
                    &disk_paths.opgg_aram_dir,
                    active_locale,
                    dd,
                );
            }
            disk_bootstrap.update(&disk_app, "tier_lists_disk", 80, true, false);
            ddragon_opt
        })
        .await
        .ok()
        .flatten();

        let client = build_provider::shared_http_client();
        let (ddragon_refresh_res, opgg_versions) = tokio::join!(
            build_provider::refresh_ddragon_if_outdated(client, active_locale),
            build_provider::fetch_opgg_versions(client, "global"),
        );

        let gc_paths = paths.clone();
        let gc_opgg_versions = opgg_versions.clone();
        let gc_ddragon_version = ddragon_refresh_res
            .as_ref()
            .ok()
            .map(|d| d.version.clone());
        let gc_stats = tokio::task::spawn_blocking(move || {
            storage::run_garbage_collector(
                &gc_paths,
                &gc_opgg_versions,
                gc_ddragon_version.as_deref(),
                storage::now_unix_secs(),
            )
        })
        .await
        .unwrap_or_default();
        let ddragon_ver = ddragon_refresh_res
            .as_ref()
            .map(|d| d.version.as_str())
            .or_else(|_| ddragon_opt.as_ref().map(|d| d.version.as_str()).ok_or(()))
            .unwrap_or("none");
        let active_patch = opgg_versions.first().map(String::as_str).unwrap_or("none");
        logging::log_info(
            "app.prewarm",
            format!(
                "event=prewarm_done elapsed_ms={} ddragon={} patch={} gc_tmp={} gc_stale_patch={} gc_inactive={} gc_corrupt={} gc_over_quota={}",
                started_at.elapsed().as_millis(),
                ddragon_ver,
                active_patch,
                gc_stats.removed_tmp_files,
                gc_stats.removed_stale_patch_files,
                gc_stats.removed_inactive_files,
                gc_stats.removed_corrupt_files,
                gc_stats.removed_over_quota_files,
            ),
        );
        bootstrap_state.update(&app_handle, "ready", 100, true, true);
    });
}

/// Ensures the full disk/network prewarm task runs once (on normal startup or first lazy window open).
pub fn ensure_full_prewarm_started(app_handle: &tauri::AppHandle) {
    let Some(lifecycle) = app_handle.try_state::<AppLifecycleState>() else {
        return;
    };
    if !lifecycle.claim_full_prewarm() {
        return;
    }

    let Some(paths) = storage::global_paths().cloned() else {
        return;
    };
    let Some(bootstrap_state) = app_handle.try_state::<BootstrapState>() else {
        return;
    };
    let settings = storage::get_global_settings();
    let active_locale = settings.general.language.unwrap_or_default();

    spawn_prewarm_task(
        app_handle.clone(),
        bootstrap_state.inner().clone(),
        paths,
        settings,
        active_locale,
    );
}

/// While headless after `--autostart`, waits passively for LCU connection and prewarms only DataDragon
/// from disk so Champ Select injection is instant without loading tier lists or WebView2.
pub fn spawn_headless_lcu_ddragon_prewarm(app_handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut rx = lcu_connector::subscribe_lcu_connection();
        loop {
            if *rx.borrow() {
                if let Some(lifecycle) = app_handle.try_state::<AppLifecycleState>()
                    && !lifecycle.is_full_prewarm_started()
                    && lifecycle.claim_ddragon_prewarm()
                {
                    let locale = storage::get_global_settings()
                        .general
                        .language
                        .unwrap_or_default();
                    let started_at = Instant::now();
                    let warmed = tokio::task::spawn_blocking(move || {
                        build_provider::ddragon::prewarm_ddragon_from_disk(locale)
                            .map(|d| d.version.clone())
                    })
                    .await
                    .ok()
                    .flatten();
                    logging::log_info(
                        "app.prewarm",
                        format!(
                            "event=headless_ddragon_prewarm elapsed_ms={} ddragon={}",
                            started_at.elapsed().as_millis(),
                            warmed.as_deref().unwrap_or("miss"),
                        ),
                    );
                }
                break;
            }
            if rx.changed().await.is_err() {
                break;
            }
        }
    });
}

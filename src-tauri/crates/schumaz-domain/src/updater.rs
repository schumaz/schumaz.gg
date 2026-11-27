pub mod policy;
pub mod portable;
pub mod release_probe;

pub use policy::{
    AUTO_UPDATE_CHECK_COOLDOWN_SECS, extract_release_tag_from_location, is_remote_version_newer,
    normalize_semver_tag, should_run_auto_check,
};
pub use portable::{
    GITHUB_RELEASES_LATEST_URL, PRODUCT_INSTALL_DIR_NAME, WINDOWS_UNINSTALL_SUBKEY,
    WindowsUninstallEntry, evaluate_portable_mode, is_exe_within_install_dir,
};
pub use release_probe::{
    ReleaseProbeOutcome, build_no_redirect_client, probe_for_app_update_with_context,
    probe_latest_release_url, record_update_check_completed,
};

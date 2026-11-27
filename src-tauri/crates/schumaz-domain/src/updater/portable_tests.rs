use std::path::Path;

use super::{WindowsUninstallEntry, evaluate_portable_mode};

#[test]
fn test_feature_portable_always_forces_portable_mode() {
    let exe = Path::new(r"C:\Users\Alice\AppData\Local\schumazgg\schumazgg.exe");
    let local = Path::new(r"C:\Users\Alice\AppData\Local");
    let entry = WindowsUninstallEntry {
        install_location: Some(r"C:\Users\Alice\AppData\Local\schumazgg".to_string()),
        uninstall_string: Some(r"C:\Users\Alice\AppData\Local\schumazgg\uninstall.exe".to_string()),
    };

    assert!(evaluate_portable_mode(
        true,
        true,
        Some(exe),
        Some(local),
        Some(&entry)
    ));
}

#[test]
fn test_installed_nsis_current_user_is_not_portable() {
    let exe = Path::new(r"\\?\C:\Users\Alice\AppData\Local\schumazgg\schumazgg.exe");
    let local = Path::new(r"C:\Users\Alice\AppData\Local");
    let entry = WindowsUninstallEntry {
        install_location: Some(r"C:\Users\Alice\AppData\Local\schumazgg\".to_string()),
        uninstall_string: Some(r"C:\Users\Alice\AppData\Local\schumazgg\uninstall.exe".to_string()),
    };

    assert!(!evaluate_portable_mode(
        false,
        true,
        Some(exe),
        Some(local),
        Some(&entry)
    ));
}

#[test]
fn test_installed_nsis_in_custom_install_location_is_not_portable() {
    let exe = Path::new(r"D:\Apps\schumaz.gg\schumazgg.exe");
    let local = Path::new(r"C:\Users\Alice\AppData\Local");
    let entry = WindowsUninstallEntry {
        install_location: Some(r#""D:\Apps\schumaz.gg""#.to_string()),
        uninstall_string: Some(r#""D:\Apps\schumaz.gg\uninstall.exe""#.to_string()),
    };

    assert!(!evaluate_portable_mode(
        false,
        true,
        Some(exe),
        Some(local),
        Some(&entry)
    ));
}

#[test]
fn test_exe_outside_local_appdata_without_uninstall_entry_is_portable() {
    let exe = Path::new(r"D:\Games\schumazgg-portable\schumazgg.exe");
    let local = Path::new(r"C:\Users\Alice\AppData\Local");

    assert!(evaluate_portable_mode(
        false,
        true,
        Some(exe),
        Some(local),
        None
    ));
}

#[test]
fn test_exe_outside_install_dir_even_with_registry_entry_is_portable() {
    let exe = Path::new(r"C:\Users\Alice\Downloads\schumazgg.exe");
    let local = Path::new(r"C:\Users\Alice\AppData\Local");
    let entry = WindowsUninstallEntry {
        install_location: Some(r"C:\Users\Alice\AppData\Local\schumazgg".to_string()),
        uninstall_string: Some(r"C:\Users\Alice\AppData\Local\schumazgg\uninstall.exe".to_string()),
    };

    assert!(evaluate_portable_mode(
        false,
        true,
        Some(exe),
        Some(local),
        Some(&entry)
    ));
}

#[test]
fn test_similar_sibling_folder_name_does_not_false_match_install_dir() {
    let exe = Path::new(r"C:\Users\Alice\AppData\Local\schumazgg-portable\schumazgg.exe");
    let local = Path::new(r"C:\Users\Alice\AppData\Local");
    let entry = WindowsUninstallEntry {
        install_location: Some(r"C:\Users\Alice\AppData\Local\schumazgg".to_string()),
        uninstall_string: None,
    };

    assert!(evaluate_portable_mode(
        false,
        true,
        Some(exe),
        Some(local),
        Some(&entry)
    ));
}

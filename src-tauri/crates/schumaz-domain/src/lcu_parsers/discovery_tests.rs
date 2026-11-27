use super::discovery_manifests::{
    collect_manifest_lockfile_candidates_from_root, parse_product_settings_yaml,
    parse_riot_client_installs_json, persist_lockfile_path, read_persisted_lockfile_path,
};
use super::discovery_parsers::*;
use std::ffi::OsString;
use std::fs;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::path::{Path, PathBuf};

fn temp_test_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "schumazgg_discovery_{tag}_{}_{nanos}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_parse_lockfile_contents_valid_and_invalid() {
    assert_eq!(
        parse_lockfile_contents("LeagueClient:12345:54321:superSecretToken:https"),
        Some((12345, 54321, "superSecretToken".to_string()))
    );
    assert_eq!(
        parse_lockfile_contents("LeagueClientUx:999:61234:uxToken:https"),
        Some((999, 61234, "uxToken".to_string()))
    );
    assert_eq!(
        parse_lockfile_contents("Riot Client:12345:54321:tok:https"),
        None
    );
    assert_eq!(
        parse_lockfile_contents("League of Legends:12345:54321:tok:https"),
        None
    );
    assert_eq!(parse_lockfile_contents("LeagueClient:0:54321:tok:https"), None);
    assert_eq!(parse_lockfile_contents("LeagueClient:1:0:tok:https"), None);
    assert_eq!(
        parse_lockfile_contents("LeagueClient:bad:54321:tok:https"),
        None
    );
    assert_eq!(parse_lockfile_contents("LeagueClient:1:54321::https"), None);
}

#[test]
fn test_extract_credentials_from_cmd() {
    let args = vec![
        OsString::from("LeagueClientUx.exe"),
        OsString::from("--remoting-auth-token=abc123token"),
        OsString::from("--app-port=61234"),
    ];
    assert_eq!(
        extract_credentials_from_cmd(&args),
        Some((61234, "abc123token".to_string()))
    );
}

#[test]
fn test_build_lcu_credentials_encodes_basic_auth() {
    let (addr, header) = build_lcu_credentials(54321, "token").unwrap();
    assert_eq!(addr, SocketAddrV4::new(Ipv4Addr::LOCALHOST, 54321));
    assert_eq!(header.to_str().unwrap(), "Basic cmlvdDp0b2tlbg==");
}

#[test]
fn test_parse_riot_client_installs_json_and_yaml() {
    let raw_json = r#"{
        "associated_client": {
            "D:/Riot Games/VALORANT/live/": "C:/Riot Games/Riot Client/RiotClientServices.exe",
            "D:/Riot Games/League of Legends (PBE)/": "C:/Riot Games/Riot Client/RiotClientServices.exe",
            "C:/Riot Games/League of Legends/": "C:/Riot Games/Riot Client/RiotClientServices.exe"
        },
        "rc_live": "C:/Riot Games/Riot Client/RiotClientServices.exe",
        "rc_default": "C:/Riot Games/Riot Client/RiotClientServices.exe"
    }"#;

    let candidates = parse_riot_client_installs_json(raw_json);
    assert_eq!(candidates.len(), 3);
    assert!(candidates[0].ends_with(Path::new("League of Legends").join("lockfile")));
    assert!(candidates[1].ends_with(Path::new("League of Legends (PBE)").join("lockfile")));
    assert!(candidates[2].ends_with(Path::new("live").join("lockfile")));

    assert_eq!(
        parse_product_settings_yaml("product_install_full_path: \"E:/Games/League of Legends\""),
        Some(PathBuf::from(if cfg!(target_os = "windows") {
            r"E:\Games\League of Legends\lockfile"
        } else {
            "E:/Games/League of Legends/lockfile"
        }))
    );
    assert_eq!(
        parse_product_settings_yaml("  product_install_full_path: 'F:/LoL/'  "),
        Some(PathBuf::from(if cfg!(target_os = "windows") {
            r"F:\LoL\lockfile"
        } else {
            "F:/LoL/lockfile"
        }))
    );
    assert_eq!(parse_product_settings_yaml("other_key: value"), None);
}

#[test]
fn test_persisted_lockfile_roundtrip_and_manifest_collection() {
    let root = temp_test_dir("manifests");
    let cache_file = root.join("lcu_lockfile_path.txt");
    let cached_lf = PathBuf::from(r"D:\Custom\League\lockfile");

    persist_lockfile_path(&cache_file, &cached_lf);
    assert_eq!(read_persisted_lockfile_path(&cache_file), Some(cached_lf.clone()));

    let riot_dir = root.join("Riot Games");
    let meta_live = riot_dir.join("Metadata").join("league_of_legends.live");
    fs::create_dir_all(&meta_live).unwrap();
    fs::write(
        riot_dir.join("RiotClientInstalls.json"),
        r#"{"associated_client":{"D:/Custom/League/":"C:/Riot/RC.exe","D:/Alt/LoL/":"C:/Riot/RC.exe"}}"#,
    )
    .unwrap();
    fs::write(
        meta_live.join("league_of_legends.live.product_settings.yaml"),
        "product_install_full_path: \"D:/Alt/LoL\"",
    )
    .unwrap();

    let collected = collect_manifest_lockfile_candidates_from_root(&root, Some(&cached_lf));
    assert_eq!(collected.len(), 1);
    assert!(collected[0].ends_with(Path::new("LoL").join("lockfile")));

    let _ = fs::remove_dir_all(&root);
}

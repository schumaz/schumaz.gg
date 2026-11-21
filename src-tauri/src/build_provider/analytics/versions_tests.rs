use super::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn test_derive_patch_major_minor_and_emergency_fallback() {
    assert_eq!(
        derive_patch_major_minor("16.24.1"),
        Some("16.24".to_string())
    );
    assert_eq!(derive_patch_major_minor("16.24"), Some("16.24".to_string()));
    assert_eq!(
        derive_patch_major_minor(" 25.1.2 "),
        Some("25.1".to_string())
    );
    assert_eq!(derive_patch_major_minor(""), None);
    assert_eq!(derive_patch_major_minor("16"), None);
    assert_eq!(derive_patch_major_minor("invalid.version"), None);

    assert_eq!(
        emergency_versions_from_ddragon(Some("16.24.1")),
        vec!["16.24".to_string()]
    );
    assert!(emergency_versions_from_ddragon(None).is_empty());
    assert!(emergency_versions_from_ddragon(Some("")).is_empty());
}

#[tokio::test]
async fn test_fetch_opgg_versions_disk_cache_and_stale_while_error() {
    let dir = std::env::temp_dir().join(format!("schumazgg_versions_cache_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/versions_disk_ok/global/champions/ranked/versions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": ["16.25", "16.24"]
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let ok_base = format!("{}/versions_disk_ok", mock_server.uri());
    let v1 = fetch_opgg_versions_with_cache_dir(&client, &ok_base, "global", Some(&dir)).await;
    assert_eq!(v1, vec!["16.25", "16.24"]);
    let disk_file = opgg_versions_file_path(&dir, "global");
    assert!(disk_file.exists());

    // Write an expired disk envelope and use a distinct base URL (not in RAM cache) to verify
    // stale-while-error offline fallback from disk.
    crate::storage::write_lz4_envelope_at(
        &disk_file,
        "16.25",
        &vec!["16.25".to_string(), "16.24".to_string()],
        1, // very old timestamp -> expired TTL
    )
    .unwrap();

    let offline_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/versions_disk_offline/global/champions/ranked/versions",
        ))
        .respond_with(ResponseTemplate::new(503))
        .mount(&offline_server)
        .await;

    let offline_base = format!("{}/versions_disk_offline", offline_server.uri());
    let stale_fallback =
        fetch_opgg_versions_with_cache_dir(&client, &offline_base, "global", Some(&dir)).await;
    assert_eq!(stale_fallback, vec!["16.25", "16.24"]);

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn test_fetch_opgg_versions_emergency_fallback_derives_from_ddragon_without_hardcode() {
    let offline_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/versions_emergency_dd/global/champions/ranked/versions",
        ))
        .respond_with(ResponseTemplate::new(503))
        .mount(&offline_server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/versions_emergency_empty/global/champions/ranked/versions",
        ))
        .respond_with(ResponseTemplate::new(503))
        .mount(&offline_server)
        .await;

    let client = reqwest::Client::new();

    let base_dd = format!("{}/versions_emergency_dd", offline_server.uri());
    let derived =
        fetch_opgg_versions_internal(&client, &base_dd, "global", None, Some("16.29.1")).await;
    assert_eq!(derived, vec!["16.29"]);

    let base_empty = format!("{}/versions_emergency_empty", offline_server.uri());
    let empty = fetch_opgg_versions_internal(&client, &base_empty, "global", None, Some("")).await;
    assert!(empty.is_empty());
}

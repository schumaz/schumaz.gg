use super::*;
use std::time::Duration;
use tokio::task::JoinSet;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn temp_assets_dir(suffix: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "schumazgg_assets_test_{}_{}",
        std::process::id(),
        suffix
    ));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn test_asset_segment_roundtrip_and_allowlist() {
    let url = "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_27.jpg?v=1";
    let seg = encode_asset_segment(url);
    assert!(seg.starts_with("v1_"));
    assert!(seg.ends_with(".jpg"));
    assert!(!seg.contains('/'));

    let decoded = decode_asset_request_segment(&format!("/{seg}")).unwrap();
    assert_eq!(decoded, url);

    assert!(is_allowed_asset_url(url));
    assert!(is_allowed_asset_url(
        "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-shared-components/global/default/images/emerald.png"
    ));
    assert!(!is_allowed_asset_url(
        "https://opgg-static.akamaized.net/meta/images/lol/latest/perk/8112.png"
    ));
    assert!(!is_allowed_asset_url(
        "https://evil.example.com/payload.png"
    ));
    assert!(!is_allowed_asset_url("file:///C:/Windows/win.ini"));
}

#[tokio::test]
async fn test_serve_cached_or_fetch_asset_caches_on_disk_and_blocks_disallowed() {
    let dir = temp_assets_dir("basic");
    let paths = AppPaths::from_root(dir.clone());
    paths.ensure_dirs().unwrap();

    let mock_server = MockServer::start().await;
    let fake_png: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3, 4];

    Mock::given(method("GET"))
        .and(path("/cdn/splash/Ahri_0.jpg"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(fake_png.clone()))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let url = format!("{}/cdn/splash/Ahri_0.jpg", mock_server.uri());

    // Public function rejects non-allowlisted hosts
    let forbidden = serve_cached_or_fetch_asset(&paths, &client, &url).await;
    assert_eq!(forbidden, Err(403));

    // Internal test-policy helper fetches once, caches natively on disk, then hits disk on second call
    let (b1, m1) = serve_cached_or_fetch_asset_with_policy(&paths, &client, &url, true)
        .await
        .unwrap();
    assert_eq!(b1, fake_png);
    assert_eq!(m1, "image/jpeg");
    assert!(asset_cache_path(&paths, &url).exists());

    let (b2, m2) = serve_cached_or_fetch_asset_with_policy(&paths, &client, &url, true)
        .await
        .unwrap();
    assert_eq!(b2, fake_png);
    assert_eq!(m2, "image/jpeg");

    mock_server.verify().await;
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn test_singleflight_coalesces_concurrent_cold_cache_requests() {
    let dir = temp_assets_dir("singleflight_ok");
    let paths = AppPaths::from_root(dir.clone());
    paths.ensure_dirs().unwrap();

    let mock_server = MockServer::start().await;
    let fake_img: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 9, 8, 7, 6];

    Mock::given(method("GET"))
        .and(path("/cdn/champion/Ahri.png"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(fake_img.clone())
                .set_delay(Duration::from_millis(80)),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let url = format!("{}/cdn/champion/Ahri.png", mock_server.uri());
    let flight = std::sync::Arc::new(AssetSingleflight::default());

    let mut join_set = JoinSet::new();
    for _ in 0..8 {
        let paths_c = paths.clone();
        let client_c = client.clone();
        let url_c = url.clone();
        let flight_c = std::sync::Arc::clone(&flight);
        join_set.spawn(async move {
            serve_cached_or_fetch_asset_internal(&flight_c, &paths_c, &client_c, &url_c, true).await
        });
    }

    let mut results = Vec::new();
    while let Some(res) = join_set.join_next().await {
        results.push(res.unwrap());
    }

    assert_eq!(results.len(), 8);
    for res in results {
        let (bytes, mime) = res.unwrap();
        assert_eq!(bytes, fake_img);
        assert_eq!(mime, "image/png");
    }

    mock_server.verify().await;
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn test_singleflight_coalesces_errors_and_allows_subsequent_retry() {
    let dir = temp_assets_dir("singleflight_err");
    let paths = AppPaths::from_root(dir.clone());
    paths.ensure_dirs().unwrap();

    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/cdn/champion/Missing.png"))
        .respond_with(ResponseTemplate::new(404).set_delay(Duration::from_millis(60)))
        .expect(2)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let url = format!("{}/cdn/champion/Missing.png", mock_server.uri());
    let flight = std::sync::Arc::new(AssetSingleflight::default());

    let mut join_set = JoinSet::new();
    for _ in 0..4 {
        let paths_c = paths.clone();
        let client_c = client.clone();
        let url_c = url.clone();
        let flight_c = std::sync::Arc::clone(&flight);
        join_set.spawn(async move {
            serve_cached_or_fetch_asset_internal(&flight_c, &paths_c, &client_c, &url_c, true).await
        });
    }

    while let Some(res) = join_set.join_next().await {
        assert_eq!(res.unwrap(), Err(404));
    }

    // In-flight entry was cleared after completion, so a later call performs a fresh attempt.
    let retry = serve_cached_or_fetch_asset_internal(&flight, &paths, &client, &url, true).await;
    assert_eq!(retry, Err(404));

    mock_server.verify().await;
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn test_fetch_asset_retries_transient_503_and_caches_on_recovery() {
    let dir = temp_assets_dir("retry_503");
    let paths = AppPaths::from_root(dir.clone());
    paths.ensure_dirs().unwrap();

    let mock_server = MockServer::start().await;
    let fake_png: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 4, 3, 2, 1];

    Mock::given(method("GET"))
        .and(path("/cdn/augments/Deft_small.png"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .expect(1)
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/cdn/augments/Deft_small.png"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(fake_png.clone()))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let url = format!("{}/cdn/augments/Deft_small.png", mock_server.uri());
    let (bytes, mime) = serve_cached_or_fetch_asset_with_policy(&paths, &client, &url, true)
        .await
        .unwrap();

    assert_eq!(bytes, fake_png);
    assert_eq!(mime, "image/png");
    assert!(asset_cache_path(&paths, &url).exists());

    mock_server.verify().await;
    let _ = fs::remove_dir_all(&dir);
}

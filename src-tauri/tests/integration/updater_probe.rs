use schumazgg_lib::updater::{
    ReleaseProbeOutcome, build_no_redirect_client, probe_latest_release_url,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_probe_latest_release_detects_newer_version_without_following_redirect() {
    let mock_server = MockServer::start().await;

    Mock::given(method("HEAD"))
        .and(path("/releases/latest"))
        .respond_with(ResponseTemplate::new(302).insert_header(
            "Location",
            "https://github.com/schumaz/schumaz.gg/releases/tag/v0.5.0",
        ))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = build_no_redirect_client().expect("no-redirect client should build");
    let url = format!("{}/releases/latest", mock_server.uri());
    let outcome = probe_latest_release_url(&client, &url, "0.4.0")
        .await
        .expect("probe should succeed");

    assert_eq!(
        outcome,
        ReleaseProbeOutcome::UpdateAvailable("0.5.0".to_string())
    );
}

#[tokio::test]
async fn test_probe_latest_release_returns_up_to_date_for_same_version_or_404() {
    let mock_server = MockServer::start().await;

    Mock::given(method("HEAD"))
        .and(path("/same"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("Location", "/schumaz/schumaz.gg/releases/tag/v0.4.0"),
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("HEAD"))
        .and(path("/missing"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = build_no_redirect_client().expect("no-redirect client should build");

    let same_outcome =
        probe_latest_release_url(&client, &format!("{}/same", mock_server.uri()), "0.4.0")
            .await
            .expect("same version probe should succeed");
    assert_eq!(same_outcome, ReleaseProbeOutcome::UpToDate);

    let missing_outcome =
        probe_latest_release_url(&client, &format!("{}/missing", mock_server.uri()), "0.4.0")
            .await
            .expect("404 should resolve as up-to-date when no release exists");
    assert_eq!(missing_outcome, ReleaseProbeOutcome::UpToDate);
}

#[tokio::test]
async fn test_probe_latest_release_returns_error_on_server_failure() {
    let mock_server = MockServer::start().await;

    Mock::given(method("HEAD"))
        .and(path("/error"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&mock_server)
        .await;

    let client = build_no_redirect_client().expect("no-redirect client should build");
    let result =
        probe_latest_release_url(&client, &format!("{}/error", mock_server.uri()), "0.4.0").await;
    assert!(result.is_err());
}

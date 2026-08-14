use reqwest::StatusCode;
use test_log::test;
use vacs_server::test_utils::TestApp;

/// The endpoint stays unavailable until an allowed OIDC subject is configured, so an
/// unconfigured deployment cannot have its release catalog reloaded by anyone.
#[test(tokio::test)]
async fn releases_reload_without_allowed_subject() {
    let app = TestApp::new().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/admin/releases/reload", app.http_base_url()))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[test(tokio::test)]
async fn releases_reload_without_token() {
    let app =
        TestApp::new_with_admin_releases_sub("repo:vacs-project/vacs:environment:production").await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/admin/releases/reload", app.http_base_url()))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[test(tokio::test)]
async fn releases_reload_with_malformed_token() {
    let app =
        TestApp::new_with_admin_releases_sub("repo:vacs-project/vacs:environment:production").await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/admin/releases/reload", app.http_base_url()))
        .bearer_auth("not-a-jwt")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

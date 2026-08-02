//! HTTP integration tests for `NETRAIL_READONLY=1` (enterprise gate):
//! all mutating endpoints return 403 READONLY_MODE, read endpoints keep
//! working. Runs in its own process (separate integration-test binary) so
//! the env var cannot race with the mutation tests in api_error_codes.rs.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use fernet::Fernet;
use netrail_lib::config::Settings;
use netrail_lib::history::SharedStore;
use netrail_lib::http_client::build_http_client;
use netrail_lib::server::{build_router, AppState};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

fn test_state(settings: Settings) -> AppState {
    let store = Arc::new(SharedStore::new(&settings));
    AppState {
        http_client: build_http_client(),
        settings_fn: Arc::new(move || settings.clone()),
        rate_limiter: netrail_lib::rate_limit::RateLimiter::from_env(),
        store,
    }
}

fn valid_settings_body() -> serde_json::Value {
    json!({
        "search_strategy": "fanout",
        "backend_order": ["ddgs"],
        "ddgs_enabled": true,
        "searxng_url": null,
        "brave_enabled": false,
        "private_mode": false,
        "history_enabled": true,
        "history_encrypt": false,
        "history_ttl_days": 90,
        "max_results": 25,
        "browser_id": null,
        "strict_backend_urls": false,
        "backends": [{"id": "ddgs", "enabled": true}],
    })
}

async fn request_json(
    app: &mut axum::Router,
    method: &str,
    uri: &str,
    body: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    let req_body = if let Some(payload) = body {
        builder = builder.header("content-type", "application/json");
        Body::from(payload.to_string())
    } else {
        Body::empty()
    };
    let response = app
        .oneshot(builder.body(req_body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

#[tokio::test]
async fn put_settings_rejected_in_readonly() {
    std::env::set_var("NETRAIL_READONLY", "1");
    let mut app = build_router(test_state(Settings::default()));
    let (status, json) = request_json(
        &mut app,
        "PUT",
        "/api/settings",
        Some(&valid_settings_body().to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json["code"], "READONLY_MODE");
    std::env::remove_var("NETRAIL_READONLY");
}

#[tokio::test]
async fn purge_history_rejected_in_readonly() {
    std::env::set_var("NETRAIL_READONLY", "1");
    let mut app = build_router(test_state(Settings::default()));
    let (status, json) = request_json(&mut app, "DELETE", "/api/history", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json["code"], "READONLY_MODE");
    std::env::remove_var("NETRAIL_READONLY");
}

#[tokio::test]
async fn delete_history_entry_rejected_in_readonly() {
    std::env::set_var("NETRAIL_READONLY", "1");
    let mut app = build_router(test_state(Settings::default()));
    let (status, json) = request_json(&mut app, "DELETE", "/api/history/1", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json["code"], "READONLY_MODE");
    std::env::remove_var("NETRAIL_READONLY");
}

#[tokio::test]
async fn create_collection_rejected_in_readonly() {
    std::env::set_var("NETRAIL_READONLY", "1");
    let mut app = build_router(test_state(Settings::default()));
    let (status, json) = request_json(
        &mut app,
        "POST",
        "/api/collections",
        Some(r#"{"name":"Research"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json["code"], "READONLY_MODE");
    std::env::remove_var("NETRAIL_READONLY");
}

#[tokio::test]
async fn add_collection_item_rejected_in_readonly() {
    std::env::set_var("NETRAIL_READONLY", "1");
    let mut app = build_router(test_state(Settings::default()));
    let (status, json) = request_json(
        &mut app,
        "POST",
        "/api/collections/1/items",
        Some(r#"{"url":"https://example.com/","title":"A"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json["code"], "READONLY_MODE");
    std::env::remove_var("NETRAIL_READONLY");
}

#[tokio::test]
async fn read_endpoints_keep_working_in_readonly() {
    std::env::set_var("NETRAIL_READONLY", "1");
    let dir = tempfile::TempDir::new().unwrap();
    let key = Fernet::generate_key();
    std::env::set_var("NETRAIL_DB_KEY", &key);
    std::env::set_var(
        "NETRAIL_DB_PATH",
        dir.path().join("n.db").to_string_lossy().as_ref(),
    );
    let mut app = build_router(test_state(Settings::default()));

    let (status, json) = request_json(&mut app, "GET", "/api/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "ok");

    let (status, _) = request_json(&mut app, "GET", "/api/settings", None).await;
    assert_eq!(status, StatusCode::OK);

    let (status, json) = request_json(&mut app, "GET", "/api/history", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["items"].is_array());

    let (status, _) = request_json(&mut app, "GET", "/api/collections", None).await;
    assert_eq!(status, StatusCode::OK);

    std::env::remove_var("NETRAIL_READONLY");
    std::env::remove_var("NETRAIL_DB_KEY");
    std::env::remove_var("NETRAIL_DB_PATH");
}

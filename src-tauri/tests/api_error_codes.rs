//! HTTP integration tests asserting stable `code` fields on API error responses.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use fernet::Fernet;
use netrail_lib::config::Settings;
use netrail_lib::history::init_history_on_startup;
use netrail_lib::http_client::build_http_client;
use netrail_lib::server::{build_router, AppState};
use std::sync::Arc;
use tempfile::TempDir;
use tower::ServiceExt;

fn test_state(settings: Settings) -> AppState {
    init_history_on_startup(&settings);
    AppState {
        http_client: build_http_client(),
        settings_fn: Arc::new(move || settings.clone()),
    }
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

fn assert_api_error(json: &serde_json::Value, expected_code: &str, expected_status: u16) {
    assert_eq!(
        json.get("code").and_then(|v| v.as_str()),
        Some(expected_code),
        "unexpected error body: {json}"
    );
    assert_eq!(
        json.get("status").and_then(|v| v.as_u64()),
        Some(expected_status as u64)
    );
    assert!(json.get("detail").and_then(|v| v.as_str()).is_some());
}

#[tokio::test]
async fn search_empty_query_returns_query_invalid() {
    let settings = Settings::default();
    let mut app = build_router(test_state(settings));
    let (status, json) = request_json(
        &mut app,
        "POST",
        "/api/search",
        Some(r#"{"query":""}"#),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_api_error(&json, "QUERY_INVALID", 400);
}

#[tokio::test]
async fn open_localhost_returns_open_url_localhost() {
    let settings = Settings::default();
    let mut app = build_router(test_state(settings));
    let (status, json) = request_json(
        &mut app,
        "POST",
        "/api/open",
        Some(r#"{"url":"http://127.0.0.1:8080/admin"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_api_error(&json, "OPEN_URL_LOCALHOST", 400);
}

#[tokio::test]
async fn invalid_settings_returns_config_max_results() {
    let settings = Settings::default();
    let mut app = build_router(test_state(settings));
    let body = serde_json::json!({
        "browser_id": null,
        "private_mode": false,
        "max_results": 0,
        "backend_order": ["ddgs"],
        "ddgs_enabled": true,
        "searxng_url": null,
        "brave_enabled": false,
        "search_strategy": "fanout",
        "backends": [],
        "history_enabled": true,
        "history_encrypt": false,
        "history_ttl_days": 90
    });
    let (status, json) = request_json(
        &mut app,
        "PUT",
        "/api/settings",
        Some(&body.to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_api_error(&json, "CONFIG_MAX_RESULTS", 400);
}

#[tokio::test]
async fn unknown_doc_returns_doc_not_found() {
    let settings = Settings::default();
    let mut app = build_router(test_state(settings));
    let (status, json) = request_json(&mut app, "GET", "/api/docs/unknown-slug", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_api_error(&json, "DOC_NOT_FOUND", 404);
}

#[tokio::test]
async fn history_disabled_returns_history_disabled() {
    let settings = Settings {
        history_enabled: false,
        ..Settings::default()
    };
    let mut app = build_router(test_state(settings));
    let (status, json) = request_json(&mut app, "GET", "/api/history", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_api_error(&json, "HISTORY_DISABLED", 400);
}

#[tokio::test]
async fn empty_collection_name_returns_collection_name_invalid() {
    let dir = TempDir::new().unwrap();
    let key = Fernet::generate_key();
    std::env::set_var("NETRAIL_DB_KEY", &key);
    std::env::set_var(
        "NETRAIL_DB_PATH",
        dir.path().join("netrail.db").to_string_lossy().as_ref(),
    );

    let settings = Settings {
        history_enabled: true,
        history_encrypt: true,
        ..Settings::default()
    };
    let mut app = build_router(test_state(settings));
    let (status, json) = request_json(
        &mut app,
        "POST",
        "/api/collections",
        Some(r#"{"name":""}"#),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_api_error(&json, "COLLECTION_NAME_INVALID", 400);

    std::env::remove_var("NETRAIL_DB_KEY");
    std::env::remove_var("NETRAIL_DB_PATH");
}

#[tokio::test]
async fn missing_history_entry_returns_history_entry_not_found() {
    let dir = TempDir::new().unwrap();
    let key = Fernet::generate_key();
    std::env::set_var("NETRAIL_DB_KEY", &key);
    std::env::set_var(
        "NETRAIL_DB_PATH",
        dir.path().join("netrail.db").to_string_lossy().as_ref(),
    );

    let settings = Settings {
        history_enabled: true,
        history_encrypt: true,
        ..Settings::default()
    };
    let mut app = build_router(test_state(settings));
    let (status, json) = request_json(&mut app, "DELETE", "/api/history/999999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_api_error(&json, "HISTORY_ENTRY_NOT_FOUND", 404);

    std::env::remove_var("NETRAIL_DB_KEY");
    std::env::remove_var("NETRAIL_DB_PATH");
}
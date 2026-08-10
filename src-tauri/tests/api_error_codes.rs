//! HTTP integration tests asserting stable `code` fields on API error responses.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use fernet::Fernet;
use netrail_lib::config::{BackendConfig, Settings};
use netrail_lib::history::SharedStore;
use netrail_lib::http_client::build_http_client;
use netrail_lib::server::{build_router, AppState};
use std::sync::Arc;
use tempfile::TempDir;
use tower::ServiceExt;

fn test_state(settings: Settings) -> AppState {
    test_state_with_limits(settings, netrail_lib::rate_limit::RateLimiter::from_env())
}

fn test_state_with_limits(
    settings: Settings,
    rate_limiter: netrail_lib::rate_limit::RateLimiter,
) -> AppState {
    let store = Arc::new(SharedStore::new(&settings));
    AppState {
        http_client: build_http_client(),
        settings_fn: Arc::new(move || settings.clone()),
        rate_limiter,
        store,
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
async fn open_encoded_loopback_returns_open_url_localhost() {
    let settings = Settings::default();
    let mut app = build_router(test_state(settings));
    let (status, json) = request_json(
        &mut app,
        "POST",
        "/api/open",
        Some(r#"{"url":"http://2130706433/"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_api_error(&json, "OPEN_URL_LOCALHOST", 400);
}

#[tokio::test]
async fn open_private_ip_returns_open_url_private() {
    let settings = Settings::default();
    let mut app = build_router(test_state(settings));
    let (status, json) = request_json(
        &mut app,
        "POST",
        "/api/open",
        Some(r#"{"url":"http://192.168.1.1/"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_api_error(&json, "OPEN_URL_PRIVATE", 400);
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

#[tokio::test]
async fn search_total_fanout_failure_returns_fanout_total_failure() {
    let settings = Settings {
        backends: vec![
            BackendConfig {
                id: "searxng".into(),
                enabled: true,
                url: Some("http://127.0.0.1:9".into()),
                api_key_env: None,
            },
            BackendConfig {
                id: "searxng".into(),
                enabled: true,
                url: Some("http://127.0.0.1:10".into()),
                api_key_env: None,
            },
        ],
        search_strategy: "fanout".into(),
        ddgs_enabled: false,
        history_enabled: false,
        ..Settings::default()
    };
    let mut app = build_router(test_state(settings));
    let (status, json) = request_json(
        &mut app,
        "POST",
        "/api/search",
        Some(r#"{"query":"rust"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_api_error(&json, "FANOUT_TOTAL_FAILURE", 502);
}

#[tokio::test]
async fn search_missing_field_returns_typed_query_invalid() {
    let settings = Settings::default();
    let mut app = build_router(test_state(settings));
    let (status, json) = request_json(&mut app, "POST", "/api/search", Some(r#"{}"#)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_api_error(&json, "QUERY_INVALID", 400);
}

#[tokio::test]
async fn search_wrong_type_returns_typed_query_invalid() {
    let settings = Settings::default();
    let mut app = build_router(test_state(settings));
    let (status, json) = request_json(
        &mut app,
        "POST",
        "/api/search",
        Some(r#"{"query":123}"#),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_api_error(&json, "QUERY_INVALID", 400);
}

#[tokio::test]
async fn search_malformed_json_returns_typed_request_invalid() {
    let settings = Settings::default();
    let mut app = build_router(test_state(settings));
    let (status, json) = request_json(&mut app, "POST", "/api/search", Some(r#"{bad"#)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_api_error(&json, "REQUEST_INVALID", 400);
}

#[tokio::test]
async fn search_out_of_range_max_results_returns_config_max_results() {
    let settings = Settings::default();
    let mut app = build_router(test_state(settings));
    let (status, json) = request_json(
        &mut app,
        "POST",
        "/api/search",
        Some(r#"{"query":"rust","max_results":999}"#),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_api_error(&json, "CONFIG_MAX_RESULTS", 400);
}

#[tokio::test]
async fn open_missing_field_returns_typed_open_url_invalid() {
    let settings = Settings::default();
    let mut app = build_router(test_state(settings));
    let (status, json) = request_json(&mut app, "POST", "/api/open", Some(r#"{}"#)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_api_error(&json, "OPEN_URL_INVALID", 400);
}

#[tokio::test]
async fn collection_missing_name_returns_typed_collection_name_invalid() {
    let settings = Settings::default();
    let mut app = build_router(test_state(settings));
    let (status, json) = request_json(&mut app, "POST", "/api/collections", Some(r#"{}"#)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_api_error(&json, "COLLECTION_NAME_INVALID", 400);
}

fn valid_settings_body() -> serde_json::Value {
    serde_json::json!({
        "search_strategy": "fanout",
        "backend_order": ["ddgs"],
        "ddgs_enabled": true,
        "searxng_url": null,
        "brave_enabled": false,
        "private_mode": false,
        "history_enabled": true,
        "history_encrypt": false,
        "history_ttl_days": 90,
        "max_results": 25
    })
}

#[tokio::test]
async fn settings_get_returns_etag() {
    let settings = Settings::default();
    let mut app = build_router(test_state(settings));
    let response = (&mut app)
        .oneshot(Request::builder().uri("/api/settings").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let etag = response.headers().get("etag").expect("GET settings ETag");
    assert!(etag.to_str().unwrap().starts_with('"'));
}

/// Env isolation: these tests mutate process-global `XDG_CONFIG_HOME`/DB env
/// vars, so they must not race each other (or `add_collection_item_respects_
/// mutate_rate_limit`) across the parallel test threads.
#[tokio::test]
#[serial_test::serial]
async fn settings_put_with_stale_if_match_returns_settings_conflict() {
    let dir = TempDir::new().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", dir.path());
    let settings = Settings::default();
    let mut app = build_router(test_state(settings));
    let body = valid_settings_body();
    let response = (&mut app)
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/settings")
                .header("content-type", "application/json")
                .header("if-match", "\"stale-etag\"")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let bytes = axum::body::to_bytes(response.into_body(), 4096).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_api_error(&json, "SETTINGS_CONFLICT", 409);
    std::env::remove_var("XDG_CONFIG_HOME");
}

#[tokio::test]
#[serial_test::serial]
async fn settings_put_with_fresh_if_match_succeeds_and_returns_new_etag() {
    let dir = TempDir::new().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", dir.path());
    let settings = Settings::default();
    let mut app = build_router(test_state(settings));

    let get = (&mut app)
        .oneshot(Request::builder().uri("/api/settings").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let etag = get
        .headers()
        .get("etag")
        .expect("ETag")
        .to_str()
        .unwrap()
        .to_string();

    let body = valid_settings_body();
    let response = (&mut app)
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/settings")
                .header("content-type", "application/json")
                .header("if-match", &etag)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get("etag").is_some());
    std::env::remove_var("XDG_CONFIG_HOME");
}

/// NR-01: collection item add shares the mutate rate-limit bucket.
#[tokio::test]
#[serial_test::serial]
async fn add_collection_item_respects_mutate_rate_limit() {
    let dir = TempDir::new().unwrap();
    std::env::set_var(
        "NETRAIL_DB_PATH",
        dir.path().join("n.db").to_string_lossy().as_ref(),
    );
    std::env::remove_var("NETRAIL_READONLY");
    std::env::remove_var("NETRAIL_DB_KEY");

    let settings = Settings {
        history_enabled: true,
        history_encrypt: false,
        ..Settings::default()
    };
    // mutate=1: create_collection consumes the only slot; add-item must 429.
    let limiter = netrail_lib::rate_limit::RateLimiter::with_limits(0, 0, 1);
    let mut app = build_router(test_state_with_limits(settings, limiter));

    let (status, json) = request_json(
        &mut app,
        "POST",
        "/api/collections",
        Some(r#"{"name":"RateLimitCol"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create collection: {json}");
    let col_id = json["id"].as_i64().expect("collection id");

    let (status, json) = request_json(
        &mut app,
        "POST",
        &format!("/api/collections/{col_id}/items"),
        Some(r#"{"url":"https://example.com/item","title":"Item"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_api_error(&json, "RATE_LIMITED", 429);

    std::env::remove_var("NETRAIL_DB_PATH");
}
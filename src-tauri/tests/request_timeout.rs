//! Request-timeout middleware: slow/stalled requests get a typed 408
//! instead of holding a worker indefinitely (slow-loris).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use netrail_lib::config::Settings;
use netrail_lib::history::SharedStore;
use netrail_lib::http_client::build_http_client;
use netrail_lib::server::{build_router, AppState};
use std::sync::Arc;
use tower::ServiceExt;

fn test_state_no_history() -> AppState {
    let settings = Settings {
        history_enabled: false,
        ..Settings::default()
    };
    let store = Arc::new(SharedStore::new(&settings));
    AppState {
        http_client: build_http_client(),
        settings_fn: Arc::new(move || settings.clone()),
        rate_limiter: netrail_lib::rate_limit::RateLimiter::from_env(),
        store,
    }
}

#[tokio::test]
#[serial_test::serial]
async fn stalled_body_times_out_with_typed_408() {
    std::env::set_var("NETRAIL_REQUEST_TIMEOUT_SECS", "1");
    // History off so the stalled search path never touches the real DB.
    std::env::set_var("NETRAIL_DB_PATH", "/tmp/netrail-timeout-test-nodb.db");
    let mut app = build_router(test_state_no_history());

    // A body stream that never yields: the Json extractor waits forever,
    // so the 1s middleware budget must fire.
    let pending = futures::stream::pending::<Result<axum::body::Bytes, std::convert::Infallible>>();
    let req_body = Body::from_stream(pending);
    let request = Request::builder()
        .method("POST")
        .uri("/api/search")
        .header("content-type", "application/json")
        .body(req_body)
        .unwrap();

    let start = std::time::Instant::now();
    let response = (&mut app).oneshot(request).await.unwrap();
    let elapsed = start.elapsed();
    assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);
    assert!(
        elapsed.as_secs() < 10,
        "timeout must fire near 1s, took {elapsed:?}"
    );
    let bytes = axum::body::to_bytes(response.into_body(), 4096).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json.get("code").and_then(|v| v.as_str()), Some("REQUEST_TIMEOUT"));
    assert_eq!(json.get("status").and_then(|v| v.as_u64()), Some(408));
    assert!(json.get("detail").and_then(|v| v.as_str()).is_some());

    std::env::remove_var("NETRAIL_REQUEST_TIMEOUT_SECS");
    std::env::remove_var("NETRAIL_DB_PATH");
}

#[tokio::test]
#[serial_test::serial]
async fn normal_request_not_affected_by_timeout() {
    std::env::set_var("NETRAIL_REQUEST_TIMEOUT_SECS", "1");
    let mut app = build_router(test_state_no_history());
    let response = (&mut app)
        .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    std::env::remove_var("NETRAIL_REQUEST_TIMEOUT_SECS");
}

#[tokio::test]
#[serial_test::serial]
async fn timeout_zero_disables_middleware() {
    // 0 disables: a fast request still passes (middleware is a no-op).
    std::env::set_var("NETRAIL_REQUEST_TIMEOUT_SECS", "0");
    let mut app = build_router(test_state_no_history());
    let response = (&mut app)
        .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    std::env::remove_var("NETRAIL_REQUEST_TIMEOUT_SECS");
}

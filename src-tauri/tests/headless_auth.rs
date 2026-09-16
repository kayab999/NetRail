//! Phase 2: headless token gate — `netrail-api` refuses to bind without an
//! explicit token decision. The missing-token case exits before bind (safe
//! to spawn in tests); live auth behavior is pinned via `oneshot` router
//! tests (no real socket, no port conflict).

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

#[test]
#[serial_test::serial]
fn headless_without_token_exits_1_with_setup_message() {
    let bin = env!("CARGO_BIN_EXE_netrail-api");
    let mut cmd = std::process::Command::new(bin);
    cmd.env_remove("NETRAIL_API_TOKEN");
    // --sbom bypasses the gate; use a harmless unknown flag-free run that
    // must fail fast before bind. argv is irrelevant to the gate.
    let output = cmd.output().expect("spawn netrail-api");
    assert_eq!(
        output.status.code(),
        Some(1),
        "missing token must exit 1, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("requires an API token"),
        "stderr must carry setup instructions, got: {stderr}"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn router_requires_token_when_configured() {
    std::env::set_var("NETRAIL_API_TOKEN", "headless-test-token");
    let mut app = build_router(test_state_no_history());

    // No credentials → 401 AUTH_REQUIRED (typed contract).
    let response = (&mut app)
        .oneshot(
            Request::builder()
                .uri("/api/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let bytes = axum::body::to_bytes(response.into_body(), 4096).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json.get("code").and_then(|v| v.as_str()), Some("AUTH_REQUIRED"));
    assert_eq!(json.get("status").and_then(|v| v.as_u64()), Some(401));

    // Health stays exempt.
    let response = (&mut app)
        .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Valid Bearer → through to handler.
    let response = (&mut app)
        .oneshot(
            Request::builder()
                .uri("/api/settings")
                .header("authorization", "Bearer headless-test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    std::env::remove_var("NETRAIL_API_TOKEN");
}

#[tokio::test]
#[serial_test::serial]
async fn router_open_without_token_when_explicit_empty() {
    std::env::set_var("NETRAIL_API_TOKEN", "");
    let mut app = build_router(test_state_no_history());
    let response = (&mut app)
        .oneshot(Request::builder().uri("/api/settings").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    std::env::remove_var("NETRAIL_API_TOKEN");
}

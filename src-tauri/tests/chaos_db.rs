//! Chaos / fault-injection tests (Sprint 2, A2/A3): under SQLite write-lock
//! contention, an unwritable database directory, or a locked-then-released
//! database, the API must keep returning the typed `{code, detail, status}`
//! contract, never panic, and recover without a process restart.
//!
//! Runs as a single `#[tokio::test]` so the env-var based DB paths cannot race
//! between tests inside this binary.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use fernet::Fernet;
use netrail_lib::config::Settings;
use netrail_lib::history::SharedStore;
use netrail_lib::http_client::build_http_client;
use netrail_lib::server::{build_router, AppState};
use rusqlite::Connection;
use std::sync::Arc;
use tempfile::TempDir;
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

fn assert_typed_api_error(json: &serde_json::Value, status: StatusCode) {
    assert_eq!(json.get("status").and_then(|v| v.as_u64()), Some(status.as_u16() as u64));
    assert!(
        json.get("code").and_then(|v| v.as_str()).is_some(),
        "error body must carry a typed code: {json}"
    );
    assert!(
        json.get("detail").and_then(|v| v.as_str()).is_some(),
        "error body must carry a typed detail: {json}"
    );
}

fn is_root() -> bool {
    std::process::Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
}

#[tokio::test]
#[serial_test::serial]
async fn db_fault_injection_typed_errors_and_recovery() {
    let dir = TempDir::new().unwrap();
    let key = Fernet::generate_key();
    let db_path = dir.path().join("n.db");
    std::env::set_var("NETRAIL_DB_KEY", &key);
    std::env::set_var("NETRAIL_DB_PATH", db_path.to_string_lossy().as_ref());

    let mut app = build_router(test_state(Settings::default()));

    // Baseline: a mutation succeeds, creating schema + WAL files.
    let (status, _) = request_json(
        &mut app,
        "POST",
        "/api/collections",
        Some(r#"{"name":"seed"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "baseline write must succeed");

    // --- 1) SQLITE_BUSY: hold the write lock from a second connection. ---
    let raw = Connection::open(&db_path).expect("open raw sqlite connection");
    raw.execute_batch("BEGIN IMMEDIATE;")
        .expect("acquire sqlite write lock");

    // WAL readers are unaffected by a concurrent writer.
    let (status, _) = request_json(&mut app, "GET", "/api/history", None).await;
    assert_eq!(status, StatusCode::OK, "reader must work under write lock");

    // A writer waits out busy_timeout, then returns a TYPED 500, never a panic.
    let (status, json) = request_json(
        &mut app,
        "POST",
        "/api/collections",
        Some(r#"{"name":"blocked"}"#),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "locked write must be a typed 500, got {status}: {json}"
    );
    assert_typed_api_error(&json, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(json["code"], "DB_ERROR");

    // --- 2) Recovery WITHOUT restart: release the lock, retry succeeds. ---
    raw.execute_batch("ROLLBACK;").expect("release sqlite write lock");
    let (status, json) = request_json(
        &mut app,
        "POST",
        "/api/collections",
        Some(r#"{"name":"blocked"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "post-unlock write must recover: {json}");
    assert_eq!(json["name"], "blocked");

    std::env::remove_var("NETRAIL_DB_KEY");
    std::env::remove_var("NETRAIL_DB_PATH");
}

#[tokio::test]
#[serial_test::serial]
async fn unwritable_db_dir_degrades_typed_then_recovers() {
    if is_root() {
        return; // chmod-based fault injection is meaningless as root
    }

    let dir = TempDir::new().unwrap();
    let key = Fernet::generate_key();
    let db_dir = dir.path().join("db");
    std::fs::create_dir_all(&db_dir).unwrap();
    std::env::set_var("NETRAIL_DB_KEY", &key);
    std::env::set_var(
        "NETRAIL_DB_PATH",
        db_dir.join("n.db").to_string_lossy().as_ref(),
    );

    use std::os::unix::fs::PermissionsExt;

    // Make the DB directory unwritable BEFORE the store opens: the store must
    // degrade to history-disabled (no panic), and the API keeps its contract.
    std::fs::set_permissions(&db_dir, std::fs::Permissions::from_mode(0o500)).unwrap();
    let mut app = build_router(test_state(Settings::default()));

    let (status, json) = request_json(
        &mut app,
        "POST",
        "/api/collections",
        Some(r#"{"name":"ro"}"#),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "unwritable DB dir must degrade to a typed HISTORY_DISABLED, got {status}: {json}"
    );
    assert_typed_api_error(&json, StatusCode::BAD_REQUEST);
    assert_eq!(json["code"], "HISTORY_DISABLED");

    // Health keeps working (read-only surface).
    let (status, json) = request_json(&mut app, "GET", "/api/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "ok");

    // Restore write access: SharedStore reopens on the next ensure() — recovery
    // without a restart.
    std::fs::set_permissions(&db_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let (status, json) = request_json(
        &mut app,
        "POST",
        "/api/collections",
        Some(r#"{"name":"back"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "post-chmod write must recover: {json}");
    assert_eq!(json["name"], "back");

    std::env::remove_var("NETRAIL_DB_KEY");
    std::env::remove_var("NETRAIL_DB_PATH");
}

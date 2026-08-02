//! Chaos / process-lifecycle tests (Sprint 2, A2): kill the real `netrail-api`
//! binary with SIGKILL mid-session and verify the WAL database survives a
//! restart without corruption; then SIGINT and verify a graceful, clean exit.
//!
//! Single `#[tokio::test]` so both scenarios share the fixed port 7421 without
//! colliding.

use fernet::Fernet;
use netrail_lib::http_client::build_http_client;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

const PORT: u16 = 7421;
const BASE_URL: &str = "http://127.0.0.1:7421";

fn spawn_server(db_path: &PathBuf, key: &str) -> Child {
    Command::new(env!("CARGO_BIN_EXE_netrail-api"))
        .env("NETRAIL_DB_PATH", db_path)
        .env("NETRAIL_DB_KEY", key)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn netrail-api")
}

async fn wait_healthy(client: &reqwest::Client) {
    for _ in 0..50 {
        if client
            .get(format!("{BASE_URL}/api/health"))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    panic!("netrail-api did not become healthy on port {PORT}");
}

async fn post_collection(client: &reqwest::Client, name: &str) -> bool {
    client
        .post(format!("{BASE_URL}/api/collections"))
        .json(&serde_json::json!({ "name": name }))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

#[tokio::test]
async fn sigkill_preserves_wal_then_sigint_shuts_down_cleanly() {
    let dir = TempDir::new().unwrap();
    let key = Fernet::generate_key();
    let db_path = dir.path().join("n.db");
    let client = build_http_client();

    // --- SIGKILL: hard-kill after writes, then restart and verify integrity. ---
    let mut child = spawn_server(&db_path, &key);
    wait_healthy(&client).await;
    assert!(post_collection(&client, "pre").await, "seed write before SIGKILL");

    child.kill().expect("SIGKILL server"); // kill() sends SIGKILL
    let _ = child.wait();

    // Restart on the same database file.
    let mut restarted = spawn_server(&db_path, &key);
    wait_healthy(&client).await;
    let collections = client
        .get(format!("{BASE_URL}/api/collections"))
        .send()
        .await
        .expect("list collections after restart")
        .json::<serde_json::Value>()
        .await
        .expect("collections json after restart");
    let names: Vec<String> = collections
        .as_array()
        .expect("collections is an array")
        .iter()
        .map(|c| c["name"].as_str().unwrap_or("").to_string())
        .collect();
    assert!(
        names.contains(&"pre".to_string()),
        "WAL data must survive SIGKILL (found {names:?})"
    );

    // --- SIGINT: graceful shutdown must exit 0 (in-flight work drains). ---
    assert!(post_collection(&client, "graceful").await, "write before SIGINT");
    let pid = restarted.id();
    let kill = Command::new("kill")
        .args(["-INT", &pid.to_string()])
        .status()
        .expect("send SIGINT");
    assert!(kill.success(), "kill -INT failed");

    let mut exited = false;
    for _ in 0..50 {
        if let Some(status) = restarted.try_wait().expect("try_wait") {
            assert!(
                status.success(),
                "SIGINT must exit cleanly (drain then 0), got {status}"
            );
            exited = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    assert!(exited, "server did not exit after SIGINT");
}

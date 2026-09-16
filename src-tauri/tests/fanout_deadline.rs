//! Phase 1b: pin the QA-10 fanout deadline symmetry.
//!
//! The 20s deadline is abort-on-remainder (partial results kept, remainder
//! aborted). `NETRAIL_FANOUT_DEADLINE_SECS` shortens the budget so these
//! tests run in ~seconds instead of 20s. Mirrors
//! `tests/test_fanout_deadline.py` (partial kept, hung backend bounded,
//! total stall bounded).

use netrail_lib::backends::types::SearchMode;
use netrail_lib::backends::{search_with_fanout, BackendKind};
use netrail_lib::config::{BackendConfig, Settings};
use netrail_lib::http_client::build_http_client;
use std::time::{Duration, Instant};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn searxng_settings(urls: Vec<String>) -> Settings {
    Settings {
        backends: urls
            .into_iter()
            .map(|url| BackendConfig {
                id: "searxng".into(),
                enabled: true,
                url: Some(url),
                api_key_env: None,
            })
            .collect(),
        search_strategy: "fanout".into(),
        ddgs_enabled: false,
        ..Settings::default()
    }
}

fn results_json(url: &str) -> serde_json::Value {
    serde_json::json!({
        "results": [{
            "title": "Fast",
            "url": url,
            "content": "fast backend result"
        }]
    })
}

async fn mount_fast(server: &MockServer, url: &str) {
    Mock::given(method("GET"))
        .and(path("/healthz"))
        .respond_with(ResponseTemplate::new(200))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(results_json(url)))
        .mount(server)
        .await;
}

async fn mount_slow(server: &MockServer) {
    // Health stays fast so the availability pre-check doesn't eat the
    // deadline; only /search hangs past it.
    Mock::given(method("GET"))
        .and(path("/healthz"))
        .respond_with(ResponseTemplate::new(200))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(30))
                .set_body_json(results_json("https://slow.example/1")),
        )
        .mount(server)
        .await;
}

#[tokio::test]
#[serial_test::serial]
async fn partial_results_kept_when_slow_backend_hangs() {
    std::env::set_var("NETRAIL_FANOUT_DEADLINE_SECS", "2");
    let fast = MockServer::start().await;
    let slow = MockServer::start().await;
    mount_fast(&fast, "https://fast.example/1").await;
    mount_slow(&slow).await;

    let settings = searxng_settings(vec![fast.uri(), slow.uri()]);
    let client = build_http_client();
    let start = Instant::now();
    let response = search_with_fanout(&client, "rust", SearchMode::Web, 10, &settings).await;
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(15),
        "hung backend must not stretch wall time past deadline, took {elapsed:?}"
    );
    assert!(
        !response.results.is_empty(),
        "fast backend partial results must be kept: {:?}",
        response.errors
    );
    assert!(
        response
            .errors
            .iter()
            .any(|e| e.contains("timed out after 20 seconds")),
        "timeout must be recorded in errors[]: {:?}",
        response.errors
    );

    std::env::remove_var("NETRAIL_FANOUT_DEADLINE_SECS");
}

#[tokio::test]
#[serial_test::serial]
async fn all_hung_backends_stay_bounded() {
    std::env::set_var("NETRAIL_FANOUT_DEADLINE_SECS", "2");
    let slow_a = MockServer::start().await;
    let slow_b = MockServer::start().await;
    mount_slow(&slow_a).await;
    mount_slow(&slow_b).await;

    let settings = searxng_settings(vec![slow_a.uri(), slow_b.uri()]);
    let client = build_http_client();
    let start = Instant::now();
    let response = search_with_fanout(&client, "rust", SearchMode::Web, 10, &settings).await;
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(15),
        "all-hung fanout must stay bounded, took {elapsed:?}"
    );
    assert!(
        response
            .errors
            .iter()
            .any(|e| e.contains("timed out after 20 seconds") || e.contains("deadline exhausted")),
        "timeout/deadline must be recorded: {:?}",
        response.errors
    );

    std::env::remove_var("NETRAIL_FANOUT_DEADLINE_SECS");
}

#[test]
fn backend_kind_name_parity_smoke() {
    // Guard against accidental backend renames breaking errors[] strings.
    assert_eq!(BackendKind::Ddgs.name(), "ddgs");
}

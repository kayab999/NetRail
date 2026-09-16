//! Phase 3b: fanout merge follows configured backend order, not speed.
//!
//! Both mock backends are SearXNG (same `name`), so instance order is pinned
//! via `provenance_chain` (`SearXNG @ {url}`), which is per-instance, plus
//! result URLs. Empty backends must consume no interleave slot.

use netrail_lib::backends::types::SearchMode;
use netrail_lib::backends::merge::merge_fanout;
use netrail_lib::backends::search_with_fanout;
use netrail_lib::backends::types::SearchResult;
use netrail_lib::config::{BackendConfig, Settings};
use netrail_lib::http_client::build_http_client;
use std::time::Duration;
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

fn result(url: &str, backend: &str) -> SearchResult {
    SearchResult {
        title: format!("Result from {backend}"),
        url: url.into(),
        snippet: "Test snippet".into(),
        image: None,
        source: String::new(),
        backend: backend.into(),
        provenance: backend.into(),
    }
}

async fn mount_search(server: &MockServer, url: &str, delay: Duration) {
    Mock::given(method("GET"))
        .and(path("/healthz"))
        .respond_with(ResponseTemplate::new(200))
        .mount(server)
        .await;
    let template = ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "results": [{ "title": url, "url": url, "content": "x" }]
    }));
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(template.set_delay(delay))
        .mount(server)
        .await;
}

async fn mount_empty(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/healthz"))
        .respond_with(ResponseTemplate::new(200))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": []
        })))
        .mount(server)
        .await;
}

#[tokio::test]
#[serial_test::serial]
async fn fanout_follows_configured_order_not_speed() {
    // Slow instance configured FIRST, fast SECOND. Completion order is the
    // reverse; merge input (and provenance) must follow configuration.
    let slow = MockServer::start().await;
    let fast = MockServer::start().await;
    mount_search(&slow, "https://slow.example/1", Duration::from_millis(800)).await;
    mount_search(&fast, "https://fast.example/1", Duration::ZERO).await;

    let settings = searxng_settings(vec![slow.uri(), fast.uri()]);
    let client = build_http_client();
    let response = search_with_fanout(&client, "rust", SearchMode::Web, 10, &settings).await;

    assert_eq!(
        response.provenance_chain,
        vec![
            format!("SearXNG @ {} (your instance, your engines)", slow.uri()),
            format!("SearXNG @ {} (your instance, your engines)", fast.uri()),
        ],
        "provenance must follow configured order, not speed"
    );
    let urls: Vec<&str> = response.results.iter().map(|r| r.url.as_str()).collect();
    assert!(urls.contains(&"https://slow.example/1"));
    assert!(urls.contains(&"https://fast.example/1"));
}

#[tokio::test]
#[serial_test::serial]
async fn empty_backend_consumes_no_interleave_slot() {
    let empty = MockServer::start().await;
    let fast = MockServer::start().await;
    mount_empty(&empty).await;
    mount_search(&fast, "https://fast.example/1", Duration::ZERO).await;

    let settings = searxng_settings(vec![empty.uri(), fast.uri()]);
    let client = build_http_client();
    let response = search_with_fanout(&client, "rust", SearchMode::Web, 10, &settings).await;

    assert_eq!(response.backends_used, vec!["searxng"]);
    assert_eq!(
        response.results.iter().map(|r| r.url.as_str()).collect::<Vec<_>>(),
        vec!["https://fast.example/1"]
    );
    assert!(
        response.errors.iter().any(|e| e.contains("returned no results")),
        "empty backend must be recorded: {:?}",
        response.errors
    );
}

#[test]
fn merge_fanout_round_robins_in_input_order() {
    // Caller contract: output order derives from input batch order.
    let batches = vec![
        ("ddgs".to_string(), vec![result("https://ddgs.com/0", "ddgs")]),
        ("searxng".to_string(), vec![result("https://searxng.com/0", "searxng")]),
        ("brave".to_string(), vec![result("https://brave.com/0", "brave")]),
    ];
    let merged = merge_fanout(batches, 10);
    let backends: Vec<&str> = merged.iter().map(|r| r.backend.as_str()).collect();
    assert_eq!(backends, vec!["ddgs", "searxng", "brave"]);
}

//! Phase 3c: backend fetch matrix — wiremock-backed coverage for the
//! previously untested fetch/parse code in brave (9%), ddgs (5%) and
//! wikipedia (18%). No live network: every URL points at a local mock.

use netrail_lib::backends::brave::BraveBackend;
use netrail_lib::backends::ddgs::DdgsBackend;
use netrail_lib::backends::types::SearchMode;
use netrail_lib::backends::wikipedia::WikipediaBackend;
use netrail_lib::http_client::build_http_client;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const DDG_TEXT_HTML: &str = r#"<!DOCTYPE html><html><body>
<div class="result">
  <a class="result__a" href="https://rust-lang.org/">Rust Programming Language</a>
  <span class="result__snippet">A systems programming language.</span>
</div>
<div class="result">
  <a class="result__a" href="https://example.com/empty-snippet">Untitled Page</a>
</div>
<div class="result"><span class="no-link">no anchor here</span></div>
</body></html>"#;

// ── Brave ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn brave_web_success_parses_results() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/res/v1/web/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "web": { "results": [
                {"url": "https://a.test/1", "title": "A", "description": "desc a"},
                {"url": "", "title": "Skipped", "description": "no url"},
                {"url": "https://a.test/2"}
            ]}
        })))
        .mount(&server)
        .await;

    let backend =
        BraveBackend::with_base_url(build_http_client(), "test-key", &server.uri());
    let results = backend.search("rust", SearchMode::Web, 10).await.unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].url, "https://a.test/1");
    assert_eq!(results[0].title, "A");
    assert_eq!(results[0].snippet, "desc a");
    assert_eq!(results[0].backend, "brave");
    // Missing title falls back to the URL.
    assert_eq!(results[1].title, "https://a.test/2");
}

#[tokio::test]
async fn brave_images_success_parses_thumbnails() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/res/v1/images/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {"url": "https://img.test/1.jpg", "title": "Pic",
                 "source": "img.test", "thumbnail": {"src": "https://img.test/t1.jpg"}},
                {"url": "", "title": "Skipped"}
            ]
        })))
        .mount(&server)
        .await;

    let backend =
        BraveBackend::with_base_url(build_http_client(), "test-key", &server.uri());
    let results = backend.search("cats", SearchMode::Images, 10).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].image.as_deref(), Some("https://img.test/t1.jpg"));
    assert_eq!(results[0].backend, "brave");
}

#[tokio::test]
async fn brave_http_error_maps_to_typed_code() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/res/v1/web/search"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;

    let backend =
        BraveBackend::with_base_url(build_http_client(), "test-key", &server.uri());
    let err = backend.search("rust", SearchMode::Web, 10).await.unwrap_err();
    assert_eq!(err.error_code(), "BRAVE_HTTP_ERROR");
    assert_eq!(err.status_code(), http::StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn brave_malformed_json_is_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/res/v1/web/search"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&server)
        .await;

    let backend =
        BraveBackend::with_base_url(build_http_client(), "test-key", &server.uri());
    assert!(backend.search("rust", SearchMode::Web, 10).await.is_err());
}

#[tokio::test]
async fn brave_empty_results_is_ok_empty() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/res/v1/web/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "web": {"results": []}
        })))
        .mount(&server)
        .await;

    let backend =
        BraveBackend::with_base_url(build_http_client(), "test-key", &server.uri());
    let results = backend.search("rust", SearchMode::Web, 10).await.unwrap();
    assert!(results.is_empty());
}

#[test]
#[serial_test::serial]
fn brave_from_env_missing_key_is_none() {
    std::env::remove_var("BRAVE_SEARCH_API_KEY");
    std::env::remove_var("NETRAIL_BRAVE_API_KEY");
    assert!(BraveBackend::from_env(build_http_client()).is_none());
}

// ── Wikipedia ─────────────────────────────────────────────────────────

fn opensearch_mock(payload: serde_json::Value) -> Mock {
    Mock::given(method("GET"))
        .and(path("/w/api.php"))
        .and(query_param("action", "opensearch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(payload))
}

#[tokio::test]
async fn wikipedia_opensearch_with_descriptions_needs_no_extracts() {
    let server = MockServer::start().await;
    opensearch_mock(serde_json::json!([
        "music",
        ["Music"],
        ["Art form"],
        ["https://en.wikipedia.org/wiki/Music"]
    ]))
    .mount(&server)
    .await;

    let backend = WikipediaBackend::with_base_url(build_http_client(), &server.uri());
    let results = backend.search("music", SearchMode::Web, 10).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Music");
    assert_eq!(results[0].snippet, "Art form");
    assert_eq!(results[0].backend, "wikipedia");
}

#[tokio::test]
async fn wikipedia_empty_description_falls_back_to_extracts() {
    let server = MockServer::start().await;
    opensearch_mock(serde_json::json!([
        "music",
        ["Music"],
        [""],
        ["https://en.wikipedia.org/wiki/Music"]
    ]))
    .mount(&server)
    .await;
    Mock::given(method("GET"))
        .and(path("/w/api.php"))
        .and(query_param("action", "query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "query": {"pages": {"1": {
                "title": "Music",
                "extract": "Music is\n\nan art form."
            }}}
        })))
        .mount(&server)
        .await;

    let backend = WikipediaBackend::with_base_url(build_http_client(), &server.uri());
    let results = backend.search("music", SearchMode::Web, 10).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].snippet, "Music is an art form.");
}

#[tokio::test]
async fn wikipedia_http_error_maps_to_typed_code() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/w/api.php"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let backend = WikipediaBackend::with_base_url(build_http_client(), &server.uri());
    let err = backend.search("music", SearchMode::Web, 10).await.unwrap_err();
    assert_eq!(err.error_code(), "WIKIPEDIA_HTTP_ERROR");
}

#[tokio::test]
async fn wikipedia_malformed_payload_is_ok_empty() {
    let server = MockServer::start().await;
    opensearch_mock(serde_json::json!({"unexpected": "shape"}))
        .mount(&server)
        .await;

    let backend = WikipediaBackend::with_base_url(build_http_client(), &server.uri());
    let results = backend.search("music", SearchMode::Web, 10).await.unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn wikipedia_non_web_mode_needs_no_network() {
    // No mocks mounted: any request would 404-fail, but Images short-circuits.
    let backend = WikipediaBackend::new(build_http_client());
    let results = backend.search("cats", SearchMode::Images, 10).await.unwrap();
    assert!(results.is_empty());
}

// ── DDGS ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn ddgs_text_parses_fixture_html() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/html/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(DDG_TEXT_HTML))
        .mount(&server)
        .await;

    let backend =
        DdgsBackend::with_base_urls(build_http_client(), &server.uri(), &server.uri());
    let results = backend.search("rust", SearchMode::Web, 10).await.unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].url, "https://rust-lang.org/");
    assert_eq!(results[0].title, "Rust Programming Language");
    assert_eq!(results[0].snippet, "A systems programming language.");
    assert_eq!(results[1].url, "https://example.com/empty-snippet");
}

#[tokio::test]
async fn ddgs_bot_challenge_maps_to_typed_code() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/html/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "<html><div class=\"anomaly-modal\">bots use DuckDuckGo</div></html>",
        ))
        .mount(&server)
        .await;

    let backend =
        DdgsBackend::with_base_urls(build_http_client(), &server.uri(), &server.uri());
    let err = backend.search("rust", SearchMode::Web, 10).await.unwrap_err();
    assert_eq!(err.error_code(), "DDGS_BOT_CHALLENGE");
}

#[tokio::test]
async fn ddgs_images_fetches_vqd_then_results() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>vqd=abc123&</html>"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/i.js"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {"url": "https://img.test/1.jpg", "title": "Pic",
                 "source": "img.test", "thumbnail": "https://img.test/t1.jpg"},
                {"title": "Skipped, no url"}
            ]
        })))
        .mount(&server)
        .await;

    let backend =
        DdgsBackend::with_base_urls(build_http_client(), &server.uri(), &server.uri());
    let results = backend.search("cats", SearchMode::Images, 10).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].image.as_deref(), Some("https://img.test/t1.jpg"));
}

#[tokio::test]
async fn ddgs_images_missing_vqd_maps_to_typed_code() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>no token here</html>"))
        .mount(&server)
        .await;

    let backend =
        DdgsBackend::with_base_urls(build_http_client(), &server.uri(), &server.uri());
    let err = backend.search("cats", SearchMode::Images, 10).await.unwrap_err();
    assert_eq!(err.error_code(), "DDGS_VQD_TOKEN_MISSING");
}

use netrail_lib::backends::searxng;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn fetch_config_complete() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/config"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "categories": ["general", "science"],
            "engines": [
                {"name": "duckduckgo", "enabled": true, "categories": ["general"]},
                {"name": "arxiv", "enabled": true, "categories": ["science"]}
            ]
        })))
        .mount(&server)
        .await;
    let res = searxng::fetch_config(server.uri().as_str(), false).await;
    match res {
        searxng::ConfigFetchResult::Ok(cfg) => {
            assert_eq!(cfg.categories, vec!["general", "science"]);
            assert_eq!(cfg.engines.len(), 2);
        }
        other => panic!("unexpected: {:?}", other),
    }
}

#[tokio::test]
async fn fetch_config_403_unavailable() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/config"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;
    let res = searxng::fetch_config(server.uri().as_str(), false).await;
    assert!(matches!(res, searxng::ConfigFetchResult::Unavailable(_)));
}

#[tokio::test]
async fn fetch_config_invalid_json() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/config"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{bad"))
        .mount(&server)
        .await;
    let res = searxng::fetch_config(server.uri().as_str(), false).await;
    assert!(matches!(res, searxng::ConfigFetchResult::Invalid(_)));
}

#[tokio::test]
async fn fetch_config_cache() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/config"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "categories": ["general"],
            "engines": []
        })))
        .mount(&server)
        .await;
    searxng::clear_config_cache();
    let _ = searxng::fetch_config(server.uri().as_str(), false).await;
    let _ = searxng::fetch_config(server.uri().as_str(), false).await;
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
}
pub mod brave;
pub mod ddgs;
pub mod merge;
pub mod searxng;
pub mod types;

use crate::config::Settings;
use futures::future::join_all;
use reqwest::Client;
use std::time::Duration;
use types::{SearchMode, SearchResponse, SearchResult};

#[derive(Debug, Clone)]
pub enum BackendKind {
    Ddgs,
    Searxng(String),
    Brave(Option<String>),
}

impl BackendKind {
    pub fn name(&self) -> &str {
        match self {
            Self::Ddgs => "ddgs",
            Self::Searxng(_) => "searxng",
            Self::Brave(_) => "brave",
        }
    }

    pub fn provenance(&self) -> String {
        match self {
            Self::Ddgs => ddgs::PROVENANCE.into(),
            Self::Searxng(url) => format!("SearXNG @ {url} (your instance, your engines)"),
            Self::Brave(_) => brave::PROVENANCE.into(),
        }
    }

    pub fn is_available(&self, client: &Client) -> bool {
        match self {
            Self::Ddgs => DdgsBackend::new(client.clone()).is_available(),
            Self::Searxng(url) => SearxngBackend::new(client.clone(), url).is_available(),
            Self::Brave(env) => {
                BraveBackend::from_env_var(client.clone(), env.as_deref()).is_some()
            }
        }
    }

    pub async fn search(
        &self,
        client: &Client,
        query: &str,
        mode: SearchMode,
        max_results: usize,
    ) -> Result<Vec<SearchResult>, String> {
        match self {
            Self::Ddgs => DdgsBackend::new(client.clone()).search(query, mode, max_results).await,
            Self::Searxng(url) => {
                SearxngBackend::new(client.clone(), url)
                    .search(query, mode, max_results)
                    .await
            }
            Self::Brave(env) => {
                let backend = BraveBackend::from_env_var(client.clone(), env.as_deref())
                    .ok_or_else(|| "brave: API key not set".to_string())?;
                backend.search(query, mode, max_results).await
            }
        }
    }

    pub fn supports_operators(&self) -> &'static [&'static str] {
        types::OPERATORS
    }
}

use ddgs::DdgsBackend;
use searxng::SearxngBackend;
use brave::BraveBackend;

pub fn get_enabled_backends(settings: &Settings, client: &Client) -> Vec<BackendKind> {
    let mut backends = Vec::new();

    if !settings.backends.is_empty() {
        for entry in &settings.backends {
            if !entry.enabled {
                continue;
            }
            match entry.id.as_str() {
                "ddgs" => backends.push(BackendKind::Ddgs),
                "searxng" => {
                    if let Some(url) = entry.url.clone().or_else(|| settings.searxng_url.clone())
                    {
                        backends.push(BackendKind::Searxng(url));
                    }
                }
                "brave"
                    if BraveBackend::from_env_var(
                        client.clone(),
                        entry.api_key_env.as_deref(),
                    )
                    .is_some() =>
                {
                    backends.push(BackendKind::Brave(entry.api_key_env.clone()));
                }
                _ => {}
            }
        }
        if !backends.is_empty() {
            return backends;
        }
    }

    for backend_id in &settings.backend_order {
        match backend_id.as_str() {
            "ddgs" if settings.ddgs_enabled => backends.push(BackendKind::Ddgs),
            "searxng" if settings.searxng_url.is_some() => {
                backends.push(BackendKind::Searxng(settings.searxng_url.clone().unwrap()));
            }
            "brave" if settings.brave_enabled && BraveBackend::from_env(client.clone()).is_some()
            =>
            {
                backends.push(BackendKind::Brave(None));
            }
            _ => {}
        }
    }
    if backends.is_empty() {
        backends.push(BackendKind::Ddgs);
    }
    backends
}

async fn backend_available(backend: &BackendKind, client: &Client) -> bool {
    match backend {
        BackendKind::Ddgs => DdgsBackend::new(client.clone()).is_available(),
        BackendKind::Searxng(url) => {
            if let Some(cached) = searxng::cached_health(url) {
                return cached;
            }
            searxng::check_health(client, url).await
        }
        BackendKind::Brave(env) => {
            BraveBackend::from_env_var(client.clone(), env.as_deref()).is_some()
        }
    }
}

fn sovereignty_step(backends_used: &[String]) -> u8 {
    let used: Vec<&String> = backends_used
        .iter()
        .filter(|b| b.as_str() != "none")
        .collect();
    if used.iter().any(|b| b.as_str() == "brave") {
        return 3;
    }
    if used.iter().any(|b| b.as_str() == "searxng") {
        return 3;
    }
    if used.len() > 1 {
        2
    } else {
        1
    }
}

struct BackendBatch {
    name: String,
    provenance: String,
    results: Vec<SearchResult>,
}

async fn query_backend(
    client: &Client,
    backend: BackendKind,
    query: &str,
    mode: SearchMode,
    max_results: usize,
) -> Result<BackendBatch, String> {
    let name = backend.name().to_string();
    let provenance = backend.provenance();
    let results = backend
        .search(client, query, mode, max_results)
        .await?;
    Ok(BackendBatch {
        name,
        provenance,
        results,
    })
}

const FANOUT_DEADLINE: Duration = Duration::from_secs(20);

pub async fn search_with_fanout(
    client: &Client,
    query: &str,
    mode: SearchMode,
    max_results: u32,
    settings: &Settings,
) -> SearchResponse {
    let query = query.trim();
    let max_results = max_results.clamp(1, 50) as usize;

    if query.is_empty() {
        return empty_response(query, mode);
    }

    match tokio::time::timeout(
        FANOUT_DEADLINE,
        search_with_fanout_inner(client, query, mode, max_results, settings),
    )
    .await
    {
        Ok(response) => response,
        Err(_) => {
            let mut errors = vec!["fanout: timed out after 20 seconds".into()];
            let unavailable: Vec<String> = get_enabled_backends(settings, client)
                .into_iter()
                .filter(|b| !b.is_available(client))
                .map(|b| format!("{}: unavailable", b.name()))
                .collect();
            errors.extend(unavailable);
            SearchResponse {
                query: query.into(),
                mode,
                results: vec![],
                backends_used: vec!["none".into()],
                provenance_chain: vec!["Search timed out".into()],
                sovereignty_step: 1,
                errors,
                search_strategy: settings.search_strategy.clone(),
            }
        }
    }
}

async fn search_with_fanout_inner(
    client: &Client,
    query: &str,
    mode: SearchMode,
    max_results: usize,
    settings: &Settings,
) -> SearchResponse {
    let enabled = get_enabled_backends(settings, client);
    let availability = join_all(enabled.iter().map(|b| backend_available(b, client))).await;

    let mut backends = Vec::new();
    let mut unavailable = Vec::new();
    for (backend, available) in enabled.into_iter().zip(availability) {
        if available {
            backends.push(backend);
        } else {
            unavailable.push(format!("{}: unavailable", backend.name()));
        }
    }

    if backends.is_empty() {
        return SearchResponse {
            query: query.into(),
            mode,
            results: vec![],
            backends_used: vec!["none".into()],
            provenance_chain: vec!["No backend available".into()],
            sovereignty_step: 1,
            errors: unavailable,
            search_strategy: settings.search_strategy.clone(),
        };
    }

    let tasks = backends.iter().map(|backend| {
        query_backend(client, backend.clone(), query, mode, max_results)
    });
    let outcomes = join_all(tasks).await;

    let mut errors = unavailable;
    let mut batches: Vec<(String, Vec<SearchResult>)> = Vec::new();
    let mut backends_used = Vec::new();
    let mut provenance_chain = Vec::new();

    for outcome in outcomes {
        match outcome {
            Ok(batch) if !batch.results.is_empty() => {
                backends_used.push(batch.name.clone());
                provenance_chain.push(batch.provenance.clone());
                batches.push((batch.name, batch.results));
            }
            Ok(_) => {}
            Err(err) => errors.push(err),
        }
    }

    let results = if settings.search_strategy == "fallback" {
        let flat: Vec<SearchResult> = batches.iter().flat_map(|(_, r)| r.clone()).collect();
        merge::dedupe_results(flat)
            .into_iter()
            .take(max_results)
            .collect()
    } else {
        merge::merge_fanout(batches, max_results)
    };

    let step = sovereignty_step(&backends_used);
    let backends_used = if backends_used.is_empty() {
        vec!["none".into()]
    } else {
        backends_used
    };

    SearchResponse {
        query: query.into(),
        mode,
        results,
        backends_used,
        provenance_chain: if provenance_chain.is_empty() {
            vec!["No backend returned results".into()]
        } else {
            provenance_chain
        },
        sovereignty_step: step,
        errors,
        search_strategy: settings.search_strategy.clone(),
    }
}

fn empty_response(query: &str, mode: SearchMode) -> SearchResponse {
    SearchResponse {
        query: query.into(),
        mode,
        results: vec![],
        backends_used: vec![],
        provenance_chain: vec![],
        sovereignty_step: 1,
        errors: vec![],
        search_strategy: "fanout".into(),
    }
}

/// Backward-compatible alias used by search module.
pub async fn search_with_fallback(
    client: &Client,
    query: &str,
    mode: SearchMode,
    max_results: u32,
    settings: &Settings,
) -> SearchResponse {
    search_with_fanout(client, query, mode, max_results, settings).await
}

#[cfg(test)]
mod backend_selection_tests {
    use super::*;
    use crate::config::{BackendConfig, Settings};

    #[test]
    fn structured_empty_falls_back_to_legacy_ddgs() {
        let settings = Settings {
            backends: vec![BackendConfig {
                id: "searxng".into(),
                enabled: true,
                url: None,
                api_key_env: None,
            }],
            ddgs_enabled: true,
            ..Settings::default()
        };
        let client = crate::http_client::build_http_client();
        let enabled = get_enabled_backends(&settings, &client);
        let names: Vec<_> = enabled.iter().map(|b| b.name()).collect();
        assert_eq!(names, vec!["ddgs"]);
    }

    #[test]
    fn brave_not_added_without_api_key() {
        let settings = Settings {
            backends: vec![BackendConfig {
                id: "brave".into(),
                enabled: true,
                url: None,
                api_key_env: Some("BRAVE_SEARCH_API_KEY".into()),
            }],
            ..Settings::default()
        };
        std::env::remove_var("BRAVE_SEARCH_API_KEY");
        std::env::remove_var("NETRAIL_BRAVE_API_KEY");
        let client = crate::http_client::build_http_client();
        let enabled = get_enabled_backends(&settings, &client);
        let names: Vec<_> = enabled.iter().map(|b| b.name()).collect();
        assert!(names.iter().all(|n| *n != "brave"));
    }
}

#[cfg(test)]
mod fanout_wiremock_tests {
    use super::*;
    use crate::config::{BackendConfig, Settings};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn fanout_partial_failure_merges_results_and_errors() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [{
                    "title": "Rust",
                    "url": "https://www.rust-lang.org",
                    "content": "systems programming"
                }]
            })))
            .mount(&mock_server)
            .await;

        let settings = Settings {
            backends: vec![
                BackendConfig {
                    id: "searxng".into(),
                    enabled: true,
                    url: Some(mock_server.uri()),
                    api_key_env: None,
                },
                BackendConfig {
                    id: "searxng".into(),
                    enabled: true,
                    url: Some("http://127.0.0.1:9".into()),
                    api_key_env: None,
                },
            ],
            search_strategy: "fanout".into(),
            ddgs_enabled: false,
            ..Settings::default()
        };

        let client = crate::http_client::build_http_client();
        let response =
            search_with_fanout(&client, "rust", SearchMode::Web, 10, &settings).await;

        assert!(
            !response.results.is_empty(),
            "expected SearXNG results from wiremock"
        );
        assert!(
            !response.errors.is_empty(),
            "expected unavailable backend error in partial fanout"
        );
        assert!(
            response.errors.iter().any(|e| e.contains("searxng")),
            "errors should mention searxng: {:?}",
            response.errors
        );
    }
}
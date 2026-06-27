pub mod brave;
pub mod ddgs;
pub mod merge;
pub mod searxng;
pub mod types;

use crate::config::Settings;
use futures::future::join_all;
use types::{SearchMode, SearchResponse, SearchResult};

#[derive(Debug, Clone)]
pub enum BackendKind {
    Ddgs,
    Searxng(String),
    Brave,
}

impl BackendKind {
    pub fn name(&self) -> &str {
        match self {
            Self::Ddgs => "ddgs",
            Self::Searxng(_) => "searxng",
            Self::Brave => "brave",
        }
    }

    pub fn provenance(&self) -> String {
        match self {
            Self::Ddgs => ddgs::PROVENANCE.into(),
            Self::Searxng(url) => format!("SearXNG @ {url} (your instance, your engines)"),
            Self::Brave => brave::PROVENANCE.into(),
        }
    }

    pub fn is_available(&self) -> bool {
        match self {
            Self::Ddgs => DdgsBackend::new().is_available(),
            Self::Searxng(url) => SearxngBackend::new(url).is_available(),
            Self::Brave => BraveBackend::from_env().is_some(),
        }
    }

    pub async fn search(
        &self,
        query: &str,
        mode: SearchMode,
        max_results: usize,
    ) -> Result<Vec<SearchResult>, String> {
        match self {
            Self::Ddgs => DdgsBackend::new().search(query, mode, max_results).await,
            Self::Searxng(url) => {
                SearxngBackend::new(url)
                    .search(query, mode, max_results)
                    .await
            }
            Self::Brave => {
                let backend = BraveBackend::from_env().ok_or_else(|| "brave: API key not set".to_string())?;
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

pub fn get_enabled_backends(settings: &Settings) -> Vec<BackendKind> {
    if !settings.backends.is_empty() {
        return settings
            .backends
            .iter()
            .filter(|b| b.enabled)
            .filter_map(|b| match b.id.as_str() {
                "ddgs" => Some(BackendKind::Ddgs),
                "searxng" => b
                    .url
                    .clone()
                    .or_else(|| settings.searxng_url.clone())
                    .map(BackendKind::Searxng),
                "brave" => Some(BackendKind::Brave),
                _ => None,
            })
            .collect();
    }

    let mut backends = Vec::new();
    for backend_id in &settings.backend_order {
        match backend_id.as_str() {
            "ddgs" if settings.ddgs_enabled => backends.push(BackendKind::Ddgs),
            "searxng" if settings.searxng_url.is_some() => {
                backends.push(BackendKind::Searxng(settings.searxng_url.clone().unwrap()));
            }
            "brave" if settings.brave_enabled => backends.push(BackendKind::Brave),
            _ => {}
        }
    }
    if backends.is_empty() {
        backends.push(BackendKind::Ddgs);
    }
    backends
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
    backend: BackendKind,
    query: &str,
    mode: SearchMode,
    max_results: usize,
) -> Result<BackendBatch, String> {
    let name = backend.name().to_string();
    let provenance = backend.provenance();
    let results = backend.search(query, mode, max_results).await?;
    Ok(BackendBatch {
        name,
        provenance,
        results,
    })
}

pub async fn search_with_fanout(
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

    let backends: Vec<BackendKind> = get_enabled_backends(settings)
        .into_iter()
        .filter(|b| b.is_available())
        .collect();

    let unavailable: Vec<String> = get_enabled_backends(settings)
        .into_iter()
        .filter(|b| !b.is_available())
        .map(|b| format!("{}: unavailable", b.name()))
        .collect();

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

    let tasks = backends
        .iter()
        .map(|backend| query_backend(backend.clone(), query, mode, max_results));
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
    query: &str,
    mode: SearchMode,
    max_results: u32,
    settings: &Settings,
) -> SearchResponse {
    search_with_fanout(query, mode, max_results, settings).await
}
pub mod ddgs;
pub mod searxng;
pub mod types;

use crate::config::Settings;
use types::{SearchMode, SearchResponse, SearchResult};

pub fn get_enabled_backends(settings: &Settings) -> Vec<BackendKind> {
    let mut backends = Vec::new();
    for backend_id in &settings.backend_order {
        match backend_id.as_str() {
            "ddgs" if settings.ddgs_enabled => backends.push(BackendKind::Ddgs),
            "searxng" if settings.searxng_url.is_some() => {
                backends.push(BackendKind::Searxng(settings.searxng_url.clone().unwrap()));
            }
            _ => {}
        }
    }
    if backends.is_empty() {
        backends.push(BackendKind::Ddgs);
    }
    backends
}

#[derive(Debug, Clone)]
pub enum BackendKind {
    Ddgs,
    Searxng(String),
}

impl BackendKind {
    pub fn name(&self) -> &str {
        match self {
            Self::Ddgs => "ddgs",
            Self::Searxng(_) => "searxng",
        }
    }

    pub fn provenance(&self) -> String {
        match self {
            Self::Ddgs => ddgs::PROVENANCE.into(),
            Self::Searxng(url) => format!("SearXNG @ {url} (your instance, your engines)"),
        }
    }

    pub fn is_available(&self) -> bool {
        match self {
            Self::Ddgs => DdgsBackend::new().is_available(),
            Self::Searxng(url) => SearxngBackend::new(url).is_available(),
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
        }
    }

    pub fn supports_operators(&self) -> &'static [&'static str] {
        types::OPERATORS
    }
}

use ddgs::DdgsBackend;
use searxng::SearxngBackend;

fn sovereignty_step(backends_used: &[String]) -> u8 {
    let used: Vec<&String> = backends_used
        .iter()
        .filter(|b| b.as_str() != "none")
        .collect();
    if used.iter().any(|b| b.as_str() == "searxng") {
        3
    } else if used.len() > 1 {
        2
    } else {
        1
    }
}

fn dedupe_results(results: Vec<SearchResult>) -> Vec<SearchResult> {
    let mut seen = std::collections::HashSet::new();
    let mut unique = Vec::new();
    for item in results {
        let key = item.url.trim_end_matches('/').to_lowercase();
        if seen.insert(key) {
            unique.push(item);
        }
    }
    unique
}

pub async fn search_with_fallback(
    query: &str,
    mode: SearchMode,
    max_results: u32,
    settings: &Settings,
) -> SearchResponse {
    let query = query.trim();
    let max_results = max_results.clamp(1, 50) as usize;

    if query.is_empty() {
        return SearchResponse {
            query: query.into(),
            mode,
            results: vec![],
            backends_used: vec![],
            provenance_chain: vec![],
            sovereignty_step: 1,
            errors: vec![],
        };
    }

    let backends = get_enabled_backends(settings);
    let mut errors = Vec::new();
    let mut all_results = Vec::new();
    let mut backends_used = Vec::new();
    let mut provenance_chain = Vec::new();

    for backend in backends {
        if !backend.is_available() {
            errors.push(format!("{}: unavailable", backend.name()));
            continue;
        }
        match backend.search(query, mode, max_results).await {
            Ok(batch) if !batch.is_empty() => {
                backends_used.push(backend.name().into());
                provenance_chain.push(backend.provenance());
                all_results.extend(batch);
            }
            Ok(_) => {}
            Err(err) => errors.push(format!("{}: {err}", backend.name())),
        }
    }

    let merged = dedupe_results(all_results);
    let results = merged.into_iter().take(max_results).collect();
    let step = sovereignty_step(&backends_used);

    SearchResponse {
        query: query.into(),
        mode,
        results,
        backends_used: if backends_used.is_empty() {
            vec!["none".into()]
        } else {
            backends_used
        },
        provenance_chain: if provenance_chain.is_empty() {
            vec!["No backend returned results".into()]
        } else {
            provenance_chain
        },
        sovereignty_step: step,
        errors,
    }
}
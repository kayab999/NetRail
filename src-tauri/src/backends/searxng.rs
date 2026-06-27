use super::types::{SearchMode, SearchResult};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use reqwest::Client;
use std::collections::HashMap;
use std::time::{Duration, Instant};

const HEALTH_TTL: Duration = Duration::from_secs(60);
const HEALTH_TIMEOUT: Duration = Duration::from_secs(3);

struct HealthEntry {
    ok: bool,
    checked_at: Instant,
}

static HEALTH_CACHE: Lazy<Mutex<HashMap<String, HealthEntry>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn cache_key(base_url: &str) -> String {
    base_url.trim_end_matches('/').to_lowercase()
}

fn url_format_ok(base_url: &str) -> bool {
    base_url.starts_with("http://") || base_url.starts_with("https://")
}

pub fn cached_health(base_url: &str) -> Option<bool> {
    let cache = HEALTH_CACHE.lock();
    let entry = cache.get(&cache_key(base_url))?;
    if entry.checked_at.elapsed() < HEALTH_TTL {
        Some(entry.ok)
    } else {
        None
    }
}

pub fn record_health(base_url: &str, ok: bool) {
    HEALTH_CACHE.lock().insert(
        cache_key(base_url),
        HealthEntry {
            ok,
            checked_at: Instant::now(),
        },
    );
}

pub async fn check_health(client: &Client, base_url: &str) -> bool {
    if !url_format_ok(base_url) {
        return false;
    }
    let url = format!("{}/healthz", base_url.trim_end_matches('/'));
    let ok = tokio::time::timeout(HEALTH_TIMEOUT, client.get(&url).send())
        .await
        .ok()
        .and_then(|r| r.ok())
        .map(|r| r.status().as_u16() < 500)
        .unwrap_or(false);
    record_health(base_url, ok);
    ok
}

pub struct SearxngBackend {
    base_url: String,
    client: Client,
}

impl SearxngBackend {
    pub fn new(client: Client, base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        }
    }

    pub fn name(&self) -> &'static str {
        "searxng"
    }

    pub fn provenance(&self) -> String {
        format!(
            "SearXNG @ {} (your instance, your engines)",
            self.base_url
        )
    }

    /// Sync availability uses TTL cache only — never blocks on network I/O.
    pub fn is_available(&self) -> bool {
        if !url_format_ok(&self.base_url) {
            return false;
        }
        cached_health(&self.base_url).unwrap_or(true)
    }

    pub async fn search(
        &self,
        query: &str,
        mode: SearchMode,
        max_results: usize,
    ) -> Result<Vec<SearchResult>, String> {
        let category = match mode {
            SearchMode::Images => "images",
            SearchMode::Web => "general",
        };
        let endpoint = format!("{}/search", self.base_url);
        let response = self
            .client
            .get(&endpoint)
            .query(&[
                ("q", query),
                ("format", "json"),
                ("categories", category),
            ])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            return Err(format!("searxng: HTTP {}", response.status()));
        }

        let payload: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        let provenance = self.provenance();
        let mut results = Vec::new();

        if let Some(items) = payload.get("results").and_then(|v| v.as_array()) {
            for item in items.iter().take(max_results) {
                let url = item.get("url").and_then(|v| v.as_str()).unwrap_or("");
                if url.is_empty() {
                    continue;
                }
                results.push(SearchResult {
                    title: item
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or(url)
                        .to_string(),
                    url: url.to_string(),
                    snippet: item
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    image: item
                        .get("thumbnail")
                        .or_else(|| item.get("img_src"))
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    source: item
                        .get("engine")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    backend: self.name().into(),
                    provenance: provenance.clone(),
                });
            }
        }
        Ok(results)
    }
}
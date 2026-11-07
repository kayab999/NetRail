use super::types::{SearchMode, SearchResult, SearchOptions};
use crate::error::{NetRailError, NetRailResult};
use crate::security::{check_backend_fetch_url, resolve_host_ips};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use reqwest::Client;
use reqwest::redirect::Policy;
use std::collections::HashMap;
use std::time::{Duration, Instant};

const HEALTH_TTL: Duration = Duration::from_secs(60);
const HEALTH_TIMEOUT: Duration = Duration::from_secs(3);
const CONFIG_TTL: Duration = Duration::from_secs(300);
const CONFIG_TIMEOUT: Duration = Duration::from_secs(5);

struct HealthEntry {
    ok: bool,
    checked_at: Instant,
}

static HEALTH_CACHE: Lazy<Mutex<HashMap<String, HealthEntry>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone)]
pub struct SearxngEngine {
    pub name: String,
    pub enabled: bool,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SearxngConfig {
    pub categories: Vec<String>,
    pub engines: Vec<SearxngEngine>,
}

#[derive(Debug, Clone)]
pub struct ConfigCacheEntry {
    pub result: ConfigFetchResult,
    pub checked_at: Instant,
}

#[derive(Debug, Clone)]
pub enum ConfigFetchResult {
    Ok(SearxngConfig),
    Unavailable(String),
    Invalid(String),
}

static CONFIG_CACHE: Lazy<Mutex<HashMap<String, ConfigCacheEntry>>> =
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

pub fn cached_config(base_url: &str) -> Option<ConfigFetchResult> {
    let cache = CONFIG_CACHE.lock();
    let entry = cache.get(&cache_key(base_url))?;
    if entry.checked_at.elapsed() < CONFIG_TTL {
        Some(entry.result.clone())
    } else {
        None
    }
}

pub fn record_config(base_url: &str, result: ConfigFetchResult) {
    CONFIG_CACHE.lock().insert(
        cache_key(base_url),
        ConfigCacheEntry {
            result,
            checked_at: Instant::now(),
        },
    );
}

pub fn clear_config_cache() {
    CONFIG_CACHE.lock().clear();
}

pub async fn fetch_config(base_url: &str, strict: bool) -> ConfigFetchResult {
    if !url_format_ok(base_url) {
        return ConfigFetchResult::Invalid("invalid base url".into());
    }
    if let Some(cached) = cached_config(base_url) {
        return cached;
    }
    if check_backend_fetch_url(base_url, strict, resolve_host_ips).is_err() {
        let r = ConfigFetchResult::Unavailable("backend url rejected by SSRF policy".into());
        record_config(base_url, r.clone());
        return r;
    }
    let url = format!("{}/config", base_url.trim_end_matches('/'));
    // Dedicated client: no redirects, 5s timeout.
    let cfg_client = match Client::builder()
        .timeout(CONFIG_TIMEOUT)
        .redirect(Policy::none())
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            let r = ConfigFetchResult::Unavailable(format!("client build failed: {e}"));
            record_config(base_url, r.clone());
            return r;
        }
    };
    let resp = tokio::time::timeout(CONFIG_TIMEOUT, cfg_client.get(&url).send()).await;
    let response = match resp {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            let r = ConfigFetchResult::Unavailable(format!("request failed: {e}"));
            record_config(base_url, r.clone());
            return r;
        }
        Err(_) => {
            let r = ConfigFetchResult::Unavailable("timed out after 5 seconds".into());
            record_config(base_url, r.clone());
            return r;
        }
    };
    if !response.status().is_success() {
        let r = ConfigFetchResult::Unavailable(format!("HTTP {}", response.status().as_u16()));
        record_config(base_url, r.clone());
        return r;
    }
    let payload: serde_json::Value = match response.json().await {
        Ok(v) => v,
        Err(e) => {
            let r = ConfigFetchResult::Invalid(format!("invalid JSON: {e}"));
            record_config(base_url, r.clone());
            return r;
        }
    };
    // Parse categories.
    let mut categories: Vec<String> = Vec::new();
    if let Some(cats) = payload.get("categories") {
        if let Some(arr) = cats.as_array() {
            for c in arr {
                if let Some(s) = c.as_str() {
                    categories.push(s.to_string());
                }
            }
        } else if let Some(obj) = cats.as_object() {
            for k in obj.keys() {
                categories.push(k.clone());
            }
        }
    }
    // Parse engines.
    let mut engines: Vec<SearxngEngine> = Vec::new();
    if let Some(engs) = payload.get("engines").and_then(|v| v.as_array()) {
        for e in engs {
            let name = e
                .get("name")
                .or_else(|| e.get("engine"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                continue;
            }
            let enabled = e.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
            let mut cats: Vec<String> = Vec::new();
            if let Some(ca) = e.get("categories").and_then(|v| v.as_array()) {
                for c in ca {
                    if let Some(s) = c.as_str() {
                        cats.push(s.to_string());
                    }
                }
            }
            engines.push(SearxngEngine {
                name,
                enabled,
                categories: cats,
            });
        }
    }
    // If categories empty but engines present, derive.
    if categories.is_empty() && !engines.is_empty() {
        let mut set = std::collections::BTreeSet::new();
        for e in &engines {
            for c in &e.categories {
                set.insert(c.clone());
            }
        }
        categories = set.into_iter().collect();
    }
    let cfg = SearxngConfig {
        categories,
        engines,
    };
    let r = ConfigFetchResult::Ok(cfg);
    record_config(base_url, r.clone());
    r
}

pub async fn check_health(client: &Client, base_url: &str, strict: bool) -> bool {
    if !url_format_ok(base_url) {
        return false;
    }
    // Fetch-time SSRF guard (A-05), same rules as Python's check_backend_fetch_url.
    if check_backend_fetch_url(base_url, strict, resolve_host_ips).is_err() {
        record_health(base_url, false);
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
    strict: bool,
    client: Client,
}

impl SearxngBackend {
    pub fn new(client: Client, base_url: &str, strict: bool) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            strict,
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
        options: &SearchOptions,
    ) -> NetRailResult<Vec<SearchResult>> {
        // Fetch-time SSRF guard (A-05): hostnames are resolved and classified
        // with the same rules as save-time validation before any request.
        check_backend_fetch_url(&self.base_url, self.strict, resolve_host_ips)?;
        let category = match mode {
            SearchMode::Images => "images",
            SearchMode::Web => "general",
        };
        let endpoint = format!("{}/search", self.base_url);
        let mut params: Vec<(&str, String)> = vec![
            ("q", query.to_string()),
            ("format", "json".to_string()),
            ("categories", category.to_string()),
        ];
        if let Some(lang) = options.language.as_ref() {
            params.push(("language", lang.clone()));
        }
        if let Some(tr) = options.time_range.as_ref() {
            params.push(("time_range", tr.clone()));
        }
        if let Some(pn) = options.pageno {
            params.push(("pageno", pn.to_string()));
        }
        if let Some(ss) = options.safesearch {
            params.push(("safesearch", ss.to_string()));
        }
        let response = self
            .client
            .get(&endpoint)
            .query(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(NetRailError::BackendHttp {
                code: "SEARXNG_HTTP_ERROR",
                backend: "searxng".into(),
                status: response.status().as_u16(),
            });
        }

        let payload: serde_json::Value = response.json().await?;
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
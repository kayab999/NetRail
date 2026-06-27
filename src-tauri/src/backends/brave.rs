use super::types::{SearchMode, SearchResult};
use reqwest::Client;
use std::env;

pub const PROVENANCE: &str = "Brave Search API (your key, your quota)";

pub struct BraveBackend {
    api_key: String,
    client: Client,
}

impl BraveBackend {
    pub fn from_env() -> Option<Self> {
        Self::from_env_var(None)
    }

    pub fn from_env_var(env_name: Option<&str>) -> Option<Self> {
        let primary = env_name.unwrap_or("BRAVE_SEARCH_API_KEY");
        let key = env::var(primary)
            .or_else(|_| {
                if primary != "NETRAIL_BRAVE_API_KEY" {
                    env::var("NETRAIL_BRAVE_API_KEY")
                } else {
                    Err(env::VarError::NotPresent)
                }
            })
            .or_else(|_| {
                if primary != "BRAVE_SEARCH_API_KEY" {
                    env::var("BRAVE_SEARCH_API_KEY")
                } else {
                    Err(env::VarError::NotPresent)
                }
            })
            .ok()?;
        if key.trim().is_empty() {
            return None;
        }
        Some(Self {
            api_key: key,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(12))
                .build()
                .unwrap_or_default(),
        })
    }

    pub fn name(&self) -> &'static str {
        "brave"
    }

    pub fn provenance(&self) -> &'static str {
        PROVENANCE
    }

    pub fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }

    pub async fn search(
        &self,
        query: &str,
        mode: SearchMode,
        max_results: usize,
    ) -> Result<Vec<SearchResult>, String> {
        match mode {
            SearchMode::Images => self.search_images(query, max_results).await,
            SearchMode::Web => self.search_web(query, max_results).await,
        }
    }

    async fn search_web(&self, query: &str, max_results: usize) -> Result<Vec<SearchResult>, String> {
        let response = self
            .client
            .get("https://api.search.brave.com/res/v1/web/search")
            .header("Accept", "application/json")
            .header("X-Subscription-Token", &self.api_key)
            .query(&[("q", query), ("count", &max_results.min(20).to_string())])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            return Err(format!("brave: HTTP {}", response.status()));
        }

        let payload: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        let mut results = Vec::new();

        if let Some(items) = payload
            .get("web")
            .and_then(|w| w.get("results"))
            .and_then(|v| v.as_array())
        {
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
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    image: None,
                    source: "brave".into(),
                    backend: self.name().into(),
                    provenance: self.provenance().into(),
                });
            }
        }
        Ok(results)
    }

    async fn search_images(&self, query: &str, max_results: usize) -> Result<Vec<SearchResult>, String> {
        let response = self
            .client
            .get("https://api.search.brave.com/res/v1/images/search")
            .header("Accept", "application/json")
            .header("X-Subscription-Token", &self.api_key)
            .query(&[("q", query), ("count", &max_results.min(20).to_string())])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            return Err(format!("brave: HTTP {}", response.status()));
        }

        let payload: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
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
                        .unwrap_or("Image result")
                        .to_string(),
                    url: url.to_string(),
                    snippet: item
                        .get("source")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    image: item
                        .get("thumbnail")
                        .and_then(|v| v.get("src"))
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    source: item
                        .get("source")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    backend: self.name().into(),
                    provenance: self.provenance().into(),
                });
            }
        }
        Ok(results)
    }
}
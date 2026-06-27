use super::types::{SearchMode, SearchResult};
use reqwest::Client;

pub struct SearxngBackend {
    base_url: String,
    client: Client,
}

impl SearxngBackend {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(12))
                .build()
                .unwrap_or_default(),
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

    pub fn is_available(&self) -> bool {
        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return false;
        }
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .ok()
            .and_then(|client| {
                client
                    .get(format!("{}/healthz", self.base_url))
                    .send()
                    .ok()
            })
            .map(|r| r.status().as_u16() < 500)
            .unwrap_or(false)
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
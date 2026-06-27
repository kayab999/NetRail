use super::types::{SearchMode, SearchResult};
use reqwest::Client;
use scraper::{Html, Selector};

pub const PROVENANCE: &str = "ddgs → DuckDuckGo metasearch → primarily Bing index";

const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub struct DdgsBackend {
    client: Client,
}

impl DdgsBackend {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_default();
        Self { client }
    }

    pub fn name(&self) -> &'static str {
        "ddgs"
    }

    pub fn provenance(&self) -> &'static str {
        PROVENANCE
    }

    pub fn is_available(&self) -> bool {
        true
    }

    pub async fn search(
        &self,
        query: &str,
        mode: SearchMode,
        max_results: usize,
    ) -> Result<Vec<SearchResult>, String> {
        match mode {
            SearchMode::Images => self.search_images(query, max_results).await,
            SearchMode::Web => self.search_text(query, max_results).await,
        }
    }

    async fn search_text(&self, query: &str, max_results: usize) -> Result<Vec<SearchResult>, String> {
        let url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(query)
        );
        let body = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;

        let document = Html::parse_document(&body);
        let result_sel = Selector::parse(".result").map_err(|e| format!("{e:?}"))?;
        let link_sel = Selector::parse(".result__a").map_err(|e| format!("{e:?}"))?;
        let snippet_sel = Selector::parse(".result__snippet").map_err(|e| format!("{e:?}"))?;

        let mut results = Vec::new();
        for block in document.select(&result_sel) {
            let Some(link) = block.select(&link_sel).next() else {
                continue;
            };
            let href = link.value().attr("href").unwrap_or("").to_string();
            if href.is_empty() {
                continue;
            }
            let title = link.text().collect::<String>().trim().to_string();
            let snippet = block
                .select(&snippet_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            results.push(SearchResult {
                title: if title.is_empty() {
                    href.clone()
                } else {
                    title
                },
                url: href,
                snippet,
                image: None,
                source: String::new(),
                backend: self.name().into(),
                provenance: self.provenance().into(),
            });
            if results.len() >= max_results {
                break;
            }
        }
        Ok(results)
    }

    async fn search_images(&self, query: &str, max_results: usize) -> Result<Vec<SearchResult>, String> {
        let vqd = self.fetch_vqd(query).await?;
        let url = format!(
            "https://duckduckgo.com/i.js?o=json&q={}&vqd={}",
            urlencoding::encode(query),
            urlencoding::encode(&vqd)
        );
        let body = self
            .client
            .get(&url)
            .header("Referer", "https://duckduckgo.com/")
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;

        let payload: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("ddgs images parse: {e}"))?;

        let mut results = Vec::new();
        if let Some(items) = payload.get("results").and_then(|v| v.as_array()) {
            for item in items {
                let url = item
                    .get("url")
                    .or_else(|| item.get("image"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
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
                        .or_else(|| item.get("image"))
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
                if results.len() >= max_results {
                    break;
                }
            }
        }
        Ok(results)
    }

    async fn fetch_vqd(&self, query: &str) -> Result<String, String> {
        let url = format!("https://duckduckgo.com/?q={}", urlencoding::encode(query));
        let body = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;

        for token in ["vqd=", "vqd='", "vqd=\""] {
            if let Some(start) = body.find(token) {
                let rest = &body[start + token.len()..];
                let end = rest
                    .find(['&', '\'', '"', ';', ' '])
                    .unwrap_or(rest.len());
                let vqd = rest[..end].to_string();
                if !vqd.is_empty() {
                    return Ok(vqd);
                }
            }
        }
        Err("ddgs: could not obtain image search token".into())
    }
}

impl Default for DdgsBackend {
    fn default() -> Self {
        Self::new()
    }
}
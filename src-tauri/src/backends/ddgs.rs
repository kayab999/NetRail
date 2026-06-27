use super::types::{SearchMode, SearchResult};
use super::url_resolve::{clean_result_title, resolve_result_url};
use crate::error::{NetRailError, NetRailResult};
use reqwest::Client;
use scraper::{Html, Selector};

pub const PROVENANCE: &str = "ddgs → DuckDuckGo metasearch → primarily Bing index";

pub struct DdgsBackend {
    client: Client,
}

impl DdgsBackend {
    pub fn new(client: Client) -> Self {
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
    ) -> NetRailResult<Vec<SearchResult>> {
        match mode {
            SearchMode::Images => self.search_images(query, max_results).await,
            SearchMode::Web => self.search_text(query, max_results).await,
        }
    }

    async fn search_text(
        &self,
        query: &str,
        max_results: usize,
    ) -> NetRailResult<Vec<SearchResult>> {
        let url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(query)
        );
        let body = self.client.get(&url).send().await?.text().await?;

        let document = Html::parse_document(&body);
        let result_sel = Selector::parse(".result").map_err(|e| NetRailError::Internal {
            code: "DDGS_SELECTOR",
            message: format!("{e:?}"),
        })?;
        let link_sel = Selector::parse(".result__a").map_err(|e| NetRailError::Internal {
            code: "DDGS_SELECTOR",
            message: format!("{e:?}"),
        })?;
        let snippet_sel = Selector::parse(".result__snippet").map_err(|e| NetRailError::Internal {
            code: "DDGS_SELECTOR",
            message: format!("{e:?}"),
        })?;
        let display_url_sel = Selector::parse(".result__url").ok();

        let mut results = Vec::new();
        for block in document.select(&result_sel) {
            let Some(link) = block.select(&link_sel).next() else {
                continue;
            };
            let href = link.value().attr("href").unwrap_or("").to_string();
            if href.is_empty() {
                continue;
            }
            let resolved_url = resolve_result_url(&href, 0);
            let display_hint = display_url_sel
                .as_ref()
                .and_then(|sel| block.select(sel).next())
                .map(|el| el.text().collect::<String>());
            let raw_title = link.text().collect::<String>().trim().to_string();
            let title = clean_result_title(&raw_title, &href, display_hint.as_deref());
            let snippet = block
                .select(&snippet_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            results.push(SearchResult {
                title: if title.is_empty() {
                    resolved_url.clone()
                } else {
                    title
                },
                url: resolved_url,
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

    async fn search_images(
        &self,
        query: &str,
        max_results: usize,
    ) -> NetRailResult<Vec<SearchResult>> {
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
            .await?
            .text()
            .await?;

        let payload: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            NetRailError::Parse {
                code: "DDGS_IMAGES_PARSE",
                message: format!("ddgs images parse: {e}"),
            }
        })?;

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

    async fn fetch_vqd(&self, query: &str) -> NetRailResult<String> {
        let url = format!("https://duckduckgo.com/?q={}", urlencoding::encode(query));
        let body = self.client.get(&url).send().await?.text().await?;

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
        Err(NetRailError::BackendFailure {
            code: "DDGS_VQD_TOKEN_MISSING",
            backend: "ddgs".into(),
            message: "could not obtain image search token".into(),
        })
    }
}
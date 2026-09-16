use super::types::{SearchMode, SearchResult};
use super::url_resolve::{clean_result_title, resolve_result_url};
use crate::error::{NetRailError, NetRailResult};
use reqwest::Client;
use scraper::{Html, Selector};

pub const PROVENANCE: &str = "ddgs → DuckDuckGo metasearch → primarily Bing index";

const DEFAULT_BASE_URL: &str = "https://duckduckgo.com";
const DEFAULT_HTML_BASE_URL: &str = "https://html.duckduckgo.com";

pub struct DdgsBackend {
    client: Client,
    base_url: String,
    html_base_url: String,
}

impl DdgsBackend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            base_url: DEFAULT_BASE_URL.into(),
            html_base_url: DEFAULT_HTML_BASE_URL.into(),
        }
    }

    /// Test hook: point the endpoints at a wiremock server. Production
    /// callers use `new`, which keeps the default DuckDuckGo endpoints.
    pub fn with_base_urls(client: Client, base_url: &str, html_base_url: &str) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            html_base_url: html_base_url.trim_end_matches('/').to_string(),
        }
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
        let url = format!("{}/html/", self.html_base_url);
        let body = self
            .client
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "text/html,application/xhtml+xml")
            .header("Accept-Language", "en-US,en;q=0.9")
            .body(format!("q={}&b=&l=us-en", urlencoding::encode(query)))
            .send()
            .await?
            .text()
            .await?;

        if Self::is_ddg_bot_challenge(&body) {
            return Err(NetRailError::BackendFailure {
                code: "DDGS_BOT_CHALLENGE",
                backend: self.name().into(),
                message:
                    "DuckDuckGo blocked automated search (bot challenge). Enable Brave or SearXNG."
                        .into(),
            });
        }

        self.parse_text_html(&body, max_results)
    }

    /// Pure HTML → results parser (no I/O), so selector behavior is unit
    /// testable with fixture pages instead of live DuckDuckGo.
    fn parse_text_html(
        &self,
        body: &str,
        max_results: usize,
    ) -> NetRailResult<Vec<SearchResult>> {
        let document = Html::parse_document(body);
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
            "{}/i.js?o=json&q={}&vqd={}",
            self.base_url,
            urlencoding::encode(query),
            urlencoding::encode(&vqd)
        );
        let body = self
            .client
            .get(&url)
            .header("Referer", format!("{}/", self.base_url))
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

    fn is_ddg_bot_challenge(body: &str) -> bool {
        body.contains("anomaly-modal") || body.contains("bots use DuckDuckGo")
    }

    async fn fetch_vqd(&self, query: &str) -> NetRailResult<String> {
        let url = format!("{}/?q={}", self.base_url, urlencoding::encode(query));
        let body = self.client.get(&url).send().await?.text().await?;

        Self::extract_vqd_token(&body).ok_or_else(|| NetRailError::BackendFailure {
            code: "DDGS_VQD_TOKEN_MISSING",
            backend: "ddgs".into(),
            message: "could not obtain image search token".into(),
        })
    }

    /// Pure vqd token extractor (no I/O), unit testable with fixture pages.
    fn extract_vqd_token(body: &str) -> Option<String> {
        for token in ["vqd=", "vqd='", "vqd=\""] {
            if let Some(start) = body.find(token) {
                let rest = &body[start + token.len()..];
                let end = rest
                    .find(['&', '\'', '"', ';', ' '])
                    .unwrap_or(rest.len());
                let vqd = rest[..end].to_string();
                if !vqd.is_empty() {
                    return Some(vqd);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::DdgsBackend;
    use crate::http_client::build_http_client;

    fn backend() -> DdgsBackend {
        DdgsBackend::new(build_http_client())
    }

    #[test]
    fn detects_ddg_bot_challenge_page() {
        let html = r#"<div class="anomaly-modal__title">Unfortunately, bots use DuckDuckGo too.</div>"#;
        assert!(DdgsBackend::is_ddg_bot_challenge(html));
    }

    #[test]
    fn normal_result_html_is_not_challenge() {
        let html = r#"<div class="result"><a class="result__a" href="https://example.com">Example</a></div>"#;
        assert!(!DdgsBackend::is_ddg_bot_challenge(html));
    }

    #[test]
    fn parse_skips_blocks_without_links_and_empty_hrefs() {
        let html = r#"<div class="result"><span>no anchor</span></div>
            <div class="result"><a class="result__a" href="">empty</a></div>
            <div class="result"><a class="result__a" href="https://kept.test/">Kept</a></div>"#;
        let results = backend().parse_text_html(html, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, "https://kept.test/");
    }

    #[test]
    fn parse_respects_max_results() {
        let html = (0..5)
            .map(|i| {
                format!(
                    r#"<div class="result"><a class="result__a" href="https://a.test/{i}">T{i}</a></div>"#
                )
            })
            .collect::<String>();
        let results = backend().parse_text_html(&html, 3).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn parse_empty_title_falls_back_to_url() {
        let html = r#"<div class="result"><a class="result__a" href="https://bare.test/doc"></a></div>"#;
        let results = backend().parse_text_html(html, 10).unwrap();
        assert_eq!(results.len(), 1);
        // `clean_result_title` derives "bare.test/doc" from the URL, so the
        // display title is never empty even when the anchor text is.
        assert_eq!(results[0].title, "bare.test/doc");
        assert_eq!(results[0].url, "https://bare.test/doc");
    }

    #[test]
    fn vqd_extraction_supports_quoted_variants() {
        assert_eq!(
            DdgsBackend::extract_vqd_token("<html>vqd=abc123&more</html>").as_deref(),
            Some("abc123")
        );
        assert_eq!(
            DdgsBackend::extract_vqd_token("x vqd='q1' y").as_deref(),
            Some("q1")
        );
        assert_eq!(
            DdgsBackend::extract_vqd_token("x vqd=\"q2\" y").as_deref(),
            Some("q2")
        );
    }

    #[test]
    fn vqd_extraction_rejects_empty_token() {
        assert_eq!(DdgsBackend::extract_vqd_token("<html>no token</html>"), None);
        assert_eq!(DdgsBackend::extract_vqd_token("vqd=&x=1"), None);
    }
}
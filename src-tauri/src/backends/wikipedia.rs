use super::types::{SearchMode, SearchResult};
use crate::error::{NetRailError, NetRailResult};
use reqwest::Client;
use std::collections::HashMap;

pub const PROVENANCE: &str =
    "Wikipedia OpenSearch + intro extracts (direct, no API key)";

pub struct WikipediaBackend {
    client: Client,
}

impl WikipediaBackend {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn name(&self) -> &'static str {
        "wikipedia"
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
        if mode != SearchMode::Web {
            return Ok(vec![]);
        }

        let url = format!(
            "https://en.wikipedia.org/w/api.php?action=opensearch&profile=fuzzy&search={}&limit={}&namespace=0&format=json",
            urlencoding::encode(query),
            max_results
        );
        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(NetRailError::BackendHttp {
                code: "WIKIPEDIA_HTTP_ERROR",
                backend: self.name().into(),
                status: response.status().as_u16(),
            });
        }

        let payload: serde_json::Value = response.json().await?;
        let Some(array) = payload.as_array() else {
            return Ok(vec![]);
        };
        if array.len() < 4 {
            return Ok(vec![]);
        }

        let titles = array[1].as_array().cloned().unwrap_or_default();
        let urls = array[3].as_array().cloned().unwrap_or_default();
        let descriptions = array[2].as_array().cloned().unwrap_or_default();

        let mut pending_titles = Vec::new();
        let mut rows: Vec<(String, String, String)> = Vec::new();

        for (index, title_val) in titles.iter().enumerate() {
            let Some(title) = title_val.as_str() else {
                continue;
            };
            let url = urls.get(index).and_then(|v| v.as_str()).unwrap_or("");
            if url.is_empty() {
                continue;
            }
            let snippet = descriptions
                .get(index)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if snippet.is_empty() {
                pending_titles.push(title.to_string());
            }
            rows.push((title.to_string(), url.to_string(), snippet));
            if rows.len() >= max_results {
                break;
            }
        }

        let extracts = if pending_titles.is_empty() {
            HashMap::new()
        } else {
            self.fetch_extracts(&pending_titles).await?
        };

        let mut results = Vec::new();
        for (title, url, mut snippet) in rows {
            if snippet.is_empty() {
                if let Some(extract) = extracts.get(&title.to_lowercase()) {
                    snippet = extract.clone();
                }
            }

            results.push(SearchResult {
                title,
                url,
                snippet,
                image: None,
                source: "wikipedia".into(),
                backend: self.name().into(),
                provenance: self.provenance().into(),
            });
        }
        Ok(results)
    }

    async fn fetch_extracts(&self, titles: &[String]) -> NetRailResult<HashMap<String, String>> {
        if titles.is_empty() {
            return Ok(HashMap::new());
        }

        let titles_param: String = titles
            .iter()
            .map(|t| urlencoding::encode(t).into_owned())
            .collect::<Vec<_>>()
            .join("|");

        let url = format!(
            "https://en.wikipedia.org/w/api.php?action=query&format=json&prop=extracts&exintro=1&explaintext=1&exchars=400&redirects=1&titles={titles_param}"
        );
        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(NetRailError::BackendHttp {
                code: "WIKIPEDIA_EXTRACT_HTTP_ERROR",
                backend: self.name().into(),
                status: response.status().as_u16(),
            });
        }

        let payload: serde_json::Value = response.json().await?;
        let mut extracts = HashMap::new();
        let Some(pages) = payload
            .get("query")
            .and_then(|q| q.get("pages"))
            .and_then(|p| p.as_object())
        else {
            return Ok(extracts);
        };

        for page in pages.values() {
            let Some(title) = page.get("title").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(extract) = page.get("extract").and_then(|v| v.as_str()) else {
                continue;
            };
            let normalized = normalize_extract(extract);
            if !normalized.is_empty() {
                extracts.insert(title.to_lowercase(), normalized);
            }
        }

        Ok(extracts)
    }
}

fn normalize_extract(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_opensearch_payload() {
        let payload: serde_json::Value = serde_json::json!([
            "music",
            ["Music", "MusicBrainz"],
            ["Art form", ""],
            [
                "https://en.wikipedia.org/wiki/Music",
                "https://en.wikipedia.org/wiki/MusicBrainz"
            ]
        ]);
        let titles = payload[1].as_array().unwrap();
        let urls = payload[3].as_array().unwrap();
        assert_eq!(titles.len(), 2);
        assert_eq!(urls[0], "https://en.wikipedia.org/wiki/Music");
    }

    #[test]
    fn normalize_extract_collapses_whitespace() {
        let raw = "Music is an art form.\n\nIt includes rhythm and melody.";
        assert_eq!(
            normalize_extract(raw),
            "Music is an art form. It includes rhythm and melody."
        );
    }

    #[test]
    fn parses_extract_query_payload() {
        let payload: serde_json::Value = serde_json::json!({
            "query": {
                "pages": {
                    "44057": {
                        "pageid": 44057,
                        "title": "Music",
                        "extract": "Music is an art form and cultural activity."
                    }
                }
            }
        });
        let pages = payload["query"]["pages"].as_object().unwrap();
        let page = pages["44057"].as_object().unwrap();
        assert_eq!(page["title"], "Music");
        assert!(page["extract"].as_str().unwrap().starts_with("Music is"));
    }
}
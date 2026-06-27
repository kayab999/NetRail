use super::types::{SearchMode, SearchResult};
use crate::error::{NetRailError, NetRailResult};
use reqwest::Client;

pub const PROVENANCE: &str = "Wikipedia OpenSearch API (direct, no API key)";

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

        let mut results = Vec::new();
        for (index, title_val) in titles.iter().enumerate() {
            let Some(title) = title_val.as_str() else {
                continue;
            };
            let url = urls
                .get(index)
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if url.is_empty() {
                continue;
            }
            let snippet = descriptions
                .get(index)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            results.push(SearchResult {
                title: title.to_string(),
                url: url.to_string(),
                snippet,
                image: None,
                source: "wikipedia".into(),
                backend: self.name().into(),
                provenance: self.provenance().into(),
            });
            if results.len() >= max_results {
                break;
            }
        }
        Ok(results)
    }
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
}
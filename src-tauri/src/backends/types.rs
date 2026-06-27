use serde::Serialize;

pub const OPERATORS: &[&str] = &["site:", "filetype:", "intitle:", "inurl:", "\"", "-"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    Web,
    Images,
}

impl SearchMode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "images" => Self::Images,
            _ => Self::Web,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::Images => "images",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub snippet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub source: String,
    pub backend: String,
    pub provenance: String,
}

impl SearchResult {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "title": self.title,
            "url": self.url,
            "snippet": self.snippet,
            "image": self.image,
            "source": self.source,
            "backend": self.backend,
            "provenance": self.provenance,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SearchResponse {
    pub query: String,
    pub mode: SearchMode,
    pub results: Vec<SearchResult>,
    pub backends_used: Vec<String>,
    pub provenance_chain: Vec<String>,
    pub sovereignty_step: u8,
    pub errors: Vec<String>,
    pub search_strategy: String,
}

impl SearchResponse {
    pub fn sovereignty_label(step: u8) -> &'static str {
        match step {
            2 => "Pluggable backends enabled",
            3 => "Self-hosted discovery (SearXNG)",
            4 => "Local history and corpus",
            5 => "Owned index — full sovereignty",
            _ => "Local console — borrowed indexes",
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "query": self.query,
            "mode": self.mode.as_str(),
            "count": self.results.len(),
            "results": self.results.iter().map(SearchResult::to_json).collect::<Vec<_>>(),
            "backends_used": self.backends_used,
            "provenance_chain": self.provenance_chain,
            "sovereignty": {
                "step": self.sovereignty_step,
                "total": 5,
                "label": Self::sovereignty_label(self.sovereignty_step),
            },
            "errors": self.errors,
            "search_strategy": self.search_strategy,
        })
    }
}
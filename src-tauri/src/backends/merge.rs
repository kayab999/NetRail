use super::types::SearchResult;
use super::url_resolve::resolve_result_url;
use std::collections::HashMap;
use url::Url;

const TRACKING_PARAMS: &[&str] = &[
    "utm_source",
    "utm_medium",
    "utm_campaign",
    "utm_term",
    "utm_content",
    "fbclid",
    "gclid",
    "mc_cid",
    "mc_eid",
    "ref",
    "ref_src",
    "igshid",
    "si",
];

/// Normalize a URL for deduplication: lowercase host without www, strip tracking params.
pub fn normalize_url_key(raw: &str) -> String {
    let trimmed = resolve_result_url(raw, 0);
    if let Ok(mut parsed) = Url::parse(&trimmed) {
        if let Some(host) = parsed.host_str() {
            let host = host.strip_prefix("www.").unwrap_or(host).to_lowercase();
            let _ = parsed.set_host(Some(&host));
        }
        if let Some(segments) = parsed.path_segments() {
            let path: Vec<_> = segments.collect();
            if path.last().is_some_and(|s| s.is_empty()) && path.len() > 1 {
                let joined = path[..path.len() - 1].join("/");
                parsed.set_path(&format!("/{joined}"));
            }
        }
        let mut pairs: Vec<(String, String)> = parsed
            .query_pairs()
            .filter(|(k, _)| !TRACKING_PARAMS.contains(&k.to_lowercase().as_str()))
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        parsed.set_query(None);
        if !pairs.is_empty() {
            let query: String = pairs
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&");
            parsed.set_query(Some(&query));
        }
        let mut out = parsed.to_string();
        while out.ends_with('/') && out.len() > 8 {
            out.pop();
        }
        return out.to_lowercase();
    }
    trimmed.trim_end_matches('/').to_lowercase()
}

fn richer(a: &SearchResult, b: &SearchResult) -> SearchResult {
    let a_score = a.snippet.len() + a.title.len();
    let b_score = b.snippet.len() + b.title.len();
    if b_score > a_score {
        b.clone()
    } else {
        a.clone()
    }
}

/// Dedupe by normalized URL, keeping the richer snippet.
pub fn dedupe_results(results: Vec<SearchResult>) -> Vec<SearchResult> {
    let mut map: HashMap<String, SearchResult> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for item in results {
        let key = normalize_url_key(&item.url);
        if let Some(existing) = map.get(&key) {
            map.insert(key, richer(existing, &item));
        } else {
            order.push(key.clone());
            map.insert(key, item);
        }
    }
    order
        .into_iter()
        .filter_map(|key| map.remove(&key))
        .collect()
}

/// Round-robin interleave across backend batches for index diversity.
pub fn interleave_batches(batches: Vec<Vec<SearchResult>>, max_results: usize) -> Vec<SearchResult> {
    if batches.is_empty() {
        return vec![];
    }
    if batches.len() == 1 {
        return batches
            .into_iter()
            .next()
            .unwrap_or_default()
            .into_iter()
            .take(max_results)
            .collect();
    }

    let mut indices = vec![0usize; batches.len()];
    let mut output = Vec::new();
    let mut seen = std::collections::HashSet::new();

    while output.len() < max_results {
        let mut advanced = false;
        for (i, batch) in batches.iter().enumerate() {
            while indices[i] < batch.len() {
                let item = &batch[indices[i]];
                indices[i] += 1;
                let key = normalize_url_key(&item.url);
                if seen.insert(key) {
                    output.push(item.clone());
                    advanced = true;
                    break;
                }
            }
            if output.len() >= max_results {
                break;
            }
        }
        if !advanced {
            break;
        }
    }
    output
}

/// Fanout merge: dedupe globally, then interleave per-backend ordering.
pub fn merge_fanout(
    batches: Vec<(String, Vec<SearchResult>)>,
    max_results: usize,
) -> Vec<SearchResult> {
    let batch_order: Vec<String> = batches.iter().map(|(name, _)| name.clone()).collect();
    let per_backend: Vec<Vec<SearchResult>> = batches
        .into_iter()
        .map(|(_, results)| results)
        .collect();
    let flat: Vec<SearchResult> = per_backend.iter().flatten().cloned().collect();
    let deduped = dedupe_results(flat);

    let mut by_backend: HashMap<String, Vec<SearchResult>> = HashMap::new();
    for item in deduped {
        by_backend
            .entry(item.backend.clone())
            .or_default()
            .push(item);
    }

    let ordered_batches: Vec<Vec<SearchResult>> = batch_order
        .into_iter()
        .filter_map(|name| by_backend.remove(&name))
        .collect();
    interleave_batches(ordered_batches, max_results)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(url: &str, snippet: &str, backend: &str) -> SearchResult {
        SearchResult {
            title: "Title".into(),
            url: url.into(),
            snippet: snippet.into(),
            image: None,
            source: String::new(),
            backend: backend.into(),
            provenance: String::new(),
        }
    }

    #[test]
    fn strips_tracking_params() {
        let a = normalize_url_key("https://www.Example.com/page?utm_source=x&id=1");
        let b = normalize_url_key("https://example.com/page?id=1");
        assert_eq!(a, b);
    }

    #[test]
    fn keeps_richer_snippet_on_dedupe() {
        let items = vec![
            result("https://a.test/x", "short", "ddgs"),
            result("https://a.test/x/", "much longer snippet here", "searxng"),
        ];
        let merged = dedupe_results(items);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].snippet, "much longer snippet here");
        assert_eq!(merged[0].backend, "searxng");
    }

    #[test]
    fn single_batch_respects_max_results() {
        let batch = vec![
            result("https://a/1", "", "ddgs"),
            result("https://a/2", "", "ddgs"),
            result("https://a/3", "", "ddgs"),
        ];
        let out = interleave_batches(vec![batch], 2);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn interleaves_backends() {
        let batches = vec![
            vec![result("https://a/1", "", "ddgs"), result("https://a/2", "", "ddgs")],
            vec![result("https://b/1", "", "searxng")],
        ];
        let out = interleave_batches(batches, 10);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].backend, "ddgs");
        assert_eq!(out[1].backend, "searxng");
        assert_eq!(out[2].backend, "ddgs");
    }
}
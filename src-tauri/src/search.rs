use crate::backends::types::{SearchMode, SearchResponse};
use crate::backends::search_with_fallback;
use crate::config::Settings;
use crate::error::{NetRailError, NetRailResult};
use crate::history::{get_store, HistoryStore};
use reqwest::Client;

pub async fn search(
    client: &Client,
    query: &str,
    mode: &str,
    max_results: u32,
    settings: &Settings,
) -> NetRailResult<serde_json::Value> {
    let mode = match mode.trim().to_lowercase().as_str() {
        "web" => SearchMode::Web,
        "images" => SearchMode::Images,
        other => {
            tracing::warn!(mode = %other, "invalid search mode; defaulting to web");
            SearchMode::Web
        }
    };
    let response = search_with_fallback(client, query, mode, max_results, settings).await;

    if response.results.is_empty() && !response.errors.is_empty() {
        return Err(NetRailError::FanoutFailure {
            code: "FANOUT_TOTAL_FAILURE",
            message: response.errors.join("; "),
        });
    }

    let mut payload = response.to_json();
    let step = sovereignty_with_history(response.sovereignty_step, settings);
    payload["sovereignty"]["step"] = step.into();
    payload["sovereignty"]["label"] = SearchResponse::sovereignty_label(step).into();

    if let Some(store) = get_store(settings) {
        let backends_used: Vec<String> = payload["backends_used"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        enrich_with_history(&mut payload, &store, &response, mode, backends_used)?;
    }

    Ok(payload)
}

fn sovereignty_with_history(step: u8, settings: &Settings) -> u8 {
    if !settings.history_enabled {
        return step;
    }
    if let Some(store) = get_store(settings) {
        let queries = store.stats()["queries"].as_i64().unwrap_or(0);
        if queries > 0 {
            return step.max(4);
        }
    }
    step
}

fn enrich_with_history(
    payload: &mut serde_json::Value,
    store: &HistoryStore,
    response: &crate::backends::types::SearchResponse,
    mode: SearchMode,
    backends_used: Vec<String>,
) -> NetRailResult<()> {
    let results_array = payload
        .get_mut("results")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| NetRailError::Internal {
            code: "SEARCH_PAYLOAD",
            message: "missing results".into(),
        })?;

    if !response.results.is_empty() {
        let (query_id, url_to_result_id) = store.record_search(
            &response.query,
            mode.as_str(),
            &backends_used,
            &response.results,
        )?;

        let urls: Vec<String> = response.results.iter().map(|r| r.url.clone()).collect();
        let visit_meta = store.get_visit_metadata(&urls)?;

        for item in results_array.iter_mut() {
            let url = item["url"].as_str().unwrap_or("").to_string();
            item["result_id"] = url_to_result_id
                .get(&url)
                .map(|id| serde_json::json!(id))
                .unwrap_or(serde_json::Value::Null);
            item["visit_metadata"] = visit_meta
                .get(&url)
                .cloned()
                .unwrap_or(serde_json::Value::Null);
        }
        payload["query_id"] = query_id.into();
    } else {
        let urls: Vec<String> = results_array
            .iter()
            .filter_map(|item| item["url"].as_str().map(str::to_string))
            .collect();
        let visit_meta = store.get_visit_metadata(&urls)?;
        for item in results_array.iter_mut() {
            let url = item["url"].as_str().unwrap_or("").to_string();
            item["visit_metadata"] = visit_meta
                .get(&url)
                .cloned()
                .unwrap_or(serde_json::Value::Null);
        }
    }
    Ok(())
}
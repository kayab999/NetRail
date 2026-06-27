use crate::backends::get_enabled_backends;
use crate::browsers::{discover_browsers, open_url};
use crate::config::{is_flatpak, load_settings, save_settings, static_dir, Settings, HOST, PORT, VERSION};
use crate::crypto::{encryption_active, ensure_encryption_key};
use crate::docs;
use crate::history::{get_store, init_history_on_startup, HistoryStore};
use crate::error::NetRailError;
use crate::http_client::build_http_client;
use crate::search;
use crate::security::{validate_open_url, CSP};
use reqwest::Client;
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use regex::Regex;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::services::ServeDir;

#[derive(Clone)]
pub struct AppState {
    pub http_client: Client,
    pub settings_fn: Arc<dyn Fn() -> Settings + Send + Sync>,
}

pub fn build_router(state: AppState) -> Router {
    let static_path = static_dir();
    Router::new()
        .route("/", get(index))
        .route("/api/health", get(health))
        .route("/api/backends", get(list_backends))
        .route("/api/browsers", get(list_browsers))
        .route("/api/settings", get(get_settings).put(put_settings))
        .route("/api/search", post(run_search))
        .route("/api/open", post(open_link))
        .route("/api/history", get(get_history).delete(purge_history))
        .route("/api/history/{query_id}", delete(delete_history_entry))
        .route("/api/collections", get(list_collections).post(create_collection))
        .route(
            "/api/collections/{collection_id}/items",
            post(add_collection_item),
        )
        .route(
            "/api/collections/{collection_id}/export",
            get(export_collection),
        )
        .route("/api/docs/{slug}", get(get_doc))
        .route("/api/docs/assets/{filename}", get(get_doc_asset))
        .nest_service("/static", ServeDir::new(static_path))
        .with_state(state)
        .layer(axum::middleware::from_fn(security_headers))
}

pub async fn start() -> Result<(), String> {
    init_history_on_startup(&load_settings());

    let state = AppState {
        http_client: build_http_client(),
        settings_fn: Arc::new(load_settings),
    };

    let app = build_router(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], PORT));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("bind {HOST}:{PORT}: {e}"))?;

    tracing::info!(
        static_dir = %static_dir().display(),
        "NetRail API listening on http://{HOST}:{PORT}"
    );
    axum::serve(listener, app)
        .await
        .map_err(|e| e.to_string())
}

async fn security_headers(
    request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CSP),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    response
}

async fn index() -> impl IntoResponse {
    let path = static_dir().join("index.html");
    match tokio::fs::read(&path).await {
        Ok(bytes) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            bytes,
        )
            .into_response(),
        Err(err) => {
            tracing::error!(
                path = %path.display(),
                error = %err,
                "index.html not found — UI assets missing from install"
            );
            (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                format!(
                    r#"<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"/><title>NetRail</title>
<style>body{{font-family:system-ui,sans-serif;max-width:42rem;margin:3rem auto;padding:0 1rem;color:#e8e8e8;background:#12141a}}
h1{{font-size:1.25rem}}code{{background:#1e2230;padding:.15rem .35rem;border-radius:.25rem}}</style></head>
<body><h1>NetRail UI assets missing</h1>
<p>The API is running but <code>index.html</code> was not found at <code>{}</code>.</p>
<p>Reinstall from a current release, or set <code>NETRAIL_STATIC_DIR</code> to the folder containing the web UI.</p>
<p>Developer checkout: <code>export NETRAIL_STATIC_DIR=/path/to/NetRail/netrail/static</code></p></body></html>"#,
                    path.display()
                ),
            )
                .into_response()
        }
    }
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let settings = (state.settings_fn)();
    let backends = get_enabled_backends(&settings, &state.http_client);
    let encrypt_requested = settings.history_encrypt;
    if encrypt_requested {
        ensure_encryption_key();
    }
    let encryption_ok = encryption_active();
    let store = get_store(&settings);
    let mut history = store
        .as_ref()
        .map(|s| s.stats())
        .unwrap_or_else(|| serde_json::json!({ "enabled": false }));
    if let serde_json::Value::Object(ref mut map) = history {
        map.insert("encrypt_requested".into(), encrypt_requested.into());
        map.insert("encryption_active".into(), encryption_ok.into());
        if encrypt_requested && !encryption_ok {
            map.insert(
                "encryption_warning".into(),
                "History encryption is enabled but no key is available.".into(),
            );
        }
        if crate::history::encryption_degraded() {
            map.insert("encryption_degraded".into(), true.into());
            map.insert(
                "encryption_degraded_message".into(),
                crate::history::encryption_degraded_message().into(),
            );
        }
    }

    Json(serde_json::json!({
        "status": "ok",
        "version": VERSION,
        "telemetry": "none",
        "backends_configured": backends.iter().map(|b| b.name()).collect::<Vec<_>>(),
        "default_provenance": "ddgs → DuckDuckGo metasearch → primarily Bing index",
        "history": history,
        "sandbox": if is_flatpak() { "flatpak" } else { "native" },
    }))
}

async fn list_backends(State(state): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let settings = (state.settings_fn)();
    let backends = get_enabled_backends(&settings, &state.http_client)
        .into_iter()
        .map(|b| {
            serde_json::json!({
                "name": b.name(),
                "provenance": b.provenance(),
                "available": b.is_available(&state.http_client),
                "supports_operators": b.supports_operators(),
            })
        })
        .collect();
    Json(backends)
}

async fn list_browsers() -> Json<Vec<serde_json::Value>> {
    let browsers = discover_browsers()
        .into_iter()
        .map(|b| {
            serde_json::json!({
                "id": b.id,
                "name": b.name,
                "executable": b.executable,
                "supports_private": b.supports_private,
            })
        })
        .collect();
    Json(browsers)
}

async fn get_settings(State(state): State<AppState>) -> Json<Settings> {
    Json((state.settings_fn)())
}

async fn put_settings(
    State(state): State<AppState>,
    Json(body): Json<Settings>,
) -> Result<Json<Settings>, ApiError> {
    let saved = save_settings(&body)?;
    init_history_on_startup(&saved);
    let _ = state;
    Ok(Json(saved))
}

#[derive(Deserialize)]
struct SearchRequest {
    query: String,
    #[serde(default = "default_mode")]
    mode: String,
    #[serde(default = "default_max_results")]
    max_results: u32,
}

fn default_mode() -> String {
    "web".into()
}

fn default_max_results() -> u32 {
    25
}

async fn run_search(
    State(state): State<AppState>,
    Json(body): Json<SearchRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let query = body.query.trim();
    if query.is_empty() || query.len() > 500 {
        return Err(NetRailError::InvalidQuery {
            code: "QUERY_INVALID",
            message: "Query must be 1-500 characters.".into(),
        }
        .into());
    }
    let settings = (state.settings_fn)();
    let payload = search::search(
        &state.http_client,
        query,
        &body.mode,
        body.max_results.clamp(1, 50),
        &settings,
    )
    .await?;
    Ok(Json(payload))
}

#[derive(Deserialize)]
struct OpenRequest {
    url: String,
    browser_id: Option<String>,
    #[serde(default)]
    private_mode: bool,
    result_id: Option<i64>,
}

async fn open_link(
    State(state): State<AppState>,
    Json(body): Json<OpenRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let safe_url = validate_open_url(&body.url)?;
    let mut settings = (state.settings_fn)();
    if let Some(id) = body.browser_id {
        settings.browser_id = Some(id);
    }
    if body.private_mode {
        settings.private_mode = true;
    }

    let result = open_url(&safe_url, &settings, body.result_id)?;
    Ok(Json(serde_json::to_value(result).unwrap_or_default()))
}

#[derive(Deserialize)]
struct HistoryQuery {
    q: Option<String>,
    #[serde(default = "default_history_limit")]
    limit: u32,
    #[serde(default)]
    offset: u32,
}

fn default_history_limit() -> u32 {
    50
}

fn fts_query(q: &str) -> String {
    let re = Regex::new(r"[^\w\s-]").unwrap();
    let cleaned = re.replace_all(q, " ").trim().to_string();
    if cleaned.is_empty() {
        return "\"\"".into();
    }
    cleaned
        .split_whitespace()
        .map(|part| format!("\"{part}\""))
        .collect::<Vec<_>>()
        .join(" ")
}

async fn get_history(
    State(state): State<AppState>,
    Query(params): Query<HistoryQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let settings = (state.settings_fn)();
    let store = require_store(&settings)?;
    let fts_q = params.q.as_deref().map(fts_query);
    let payload = store.list_history(
        fts_q.as_deref(),
        params.limit.clamp(1, 200),
        params.offset,
    )?;
    Ok(Json(payload))
}

async fn delete_history_entry(
    State(state): State<AppState>,
    Path(query_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let settings = (state.settings_fn)();
    let store = require_store(&settings)?;
    if !store.delete_history_entry(query_id)? {
        return Err(NetRailError::NotFound {
            code: "HISTORY_ENTRY_NOT_FOUND",
            entity: format!("history entry {query_id}"),
        }
        .into());
    }
    Ok(Json(serde_json::json!({
        "status": "ok",
        "deleted_id": query_id,
    })))
}

async fn purge_history(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let settings = (state.settings_fn)();
    let store = require_store(&settings)?;
    let count = store.purge_all_history()?;
    Ok(Json(serde_json::json!({
        "status": "ok",
        "purged": count,
    })))
}

#[derive(Deserialize)]
struct CollectionCreate {
    name: String,
}

async fn list_collections(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let settings = (state.settings_fn)();
    let store = require_store(&settings)?;
    let items = store.list_collections()?;
    Ok(Json(items))
}

async fn create_collection(
    State(state): State<AppState>,
    Json(body): Json<CollectionCreate>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let settings = (state.settings_fn)();
    let store = require_store(&settings)?;
    let name = body.name.trim();
    if name.is_empty() || name.len() > 120 {
        return Err(NetRailError::InvalidConfig {
            code: "COLLECTION_NAME_INVALID",
            message: "Collection name must be 1-120 characters.".into(),
        }
        .into());
    }
    Ok(Json(store.create_collection(name)?))
}

#[derive(Deserialize)]
struct CollectionItemCreate {
    url: String,
    title: String,
    notes: Option<String>,
}

async fn add_collection_item(
    State(state): State<AppState>,
    Path(collection_id): Path<i64>,
    Json(body): Json<CollectionItemCreate>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let settings = (state.settings_fn)();
    let store = require_store(&settings)?;
    let safe_url = validate_open_url(&body.url)?;
    let title = body.title.trim();
    if title.is_empty() || title.len() > 500 {
        return Err(NetRailError::InvalidConfig {
            code: "COLLECTION_ITEM_TITLE_INVALID",
            message: "Title must be 1-500 characters.".into(),
        }
        .into());
    }
    Ok(Json(
        store.add_collection_item(collection_id, &safe_url, title, body.notes.as_deref())?,
    ))
}

#[derive(Deserialize)]
struct ExportQuery {
    #[serde(default = "default_export_fmt")]
    fmt: String,
}

fn default_export_fmt() -> String {
    "json".into()
}

async fn get_doc(Path(slug): Path<String>) -> Result<Json<serde_json::Value>, ApiError> {
    docs::load_doc(&slug).map(Json).map_err(Into::into)
}

async fn get_doc_asset(Path(filename): Path<String>) -> Result<Response, ApiError> {
    let path = docs::asset_path(&filename).ok_or_else(|| {
        ApiError::from(NetRailError::NotFound {
            code: "DOC_ASSET_NOT_FOUND",
            entity: filename.clone(),
        })
    })?;
    let bytes = tokio::fs::read(&path).await.map_err(|e| {
        ApiError::from(NetRailError::NotFound {
            code: "DOC_ASSET_NOT_FOUND",
            entity: format!("{filename}: {e}"),
        })
    })?;
    let media = match filename.rsplit('.').next() {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    };
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, media)],
        bytes,
    )
        .into_response())
}

async fn export_collection(
    State(state): State<AppState>,
    Path(collection_id): Path<i64>,
    Query(params): Query<ExportQuery>,
) -> Result<Response, ApiError> {
    let settings = (state.settings_fn)();
    let store = require_store(&settings)?;
    let fmt = if params.fmt == "csv" { "csv" } else { "json" };
    let content = store.export_collection(collection_id, fmt)?;
    let media = if fmt == "csv" {
        "text/csv"
    } else {
        "application/json"
    };
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, media)],
        content,
    )
        .into_response())
}

fn require_store(settings: &Settings) -> Result<HistoryStore, ApiError> {
    get_store(settings).ok_or_else(|| {
        NetRailError::InvalidConfig {
            code: "HISTORY_DISABLED",
            message: "History is disabled in settings.".into(),
        }
        .into()
    })
}

struct ApiError {
    status: StatusCode,
    code: &'static str,
    detail: String,
}

impl From<NetRailError> for ApiError {
    fn from(err: NetRailError) -> Self {
        Self {
            status: err.status_code(),
            code: err.error_code(),
            detail: err.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "code": self.code,
            "detail": self.detail,
            "status": self.status.as_u16(),
        });
        (self.status, Json(body)).into_response()
    }
}
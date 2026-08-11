use crate::audit;
use crate::auth::{self, check_request_token, inject_ui_token, path_requires_token};
use crate::backends::get_enabled_backends;
use crate::browsers::{discover_browsers, open_url};
use crate::config::{is_flatpak, load_settings, save_settings, static_dir, readonly_mode, Settings, API_CONTRACT, HOST, PORT, VERSION};
use crate::crypto::{encryption_active, ensure_encryption_key};
use crate::docs;
use crate::history::SharedStore;
use crate::error::NetRailError;
use crate::http_client::build_http_client;
use crate::rate_limit::RateLimiter;
use crate::search;
use crate::security::{pin_open_host, resolve_host_ips, validate_open_url, CSP};
use reqwest::Client;
use axum::{
    extract::{rejection::{JsonRejection, QueryRejection}, Path, Query, State},
    http::{header, HeaderMap, HeaderValue, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use sha2::Digest;
use base64::Engine;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::services::ServeDir;

static FTS_STRIP: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^\w\s-]").expect("FTS strip regex"));

#[derive(Clone)]
pub struct AppState {
    pub http_client: Client,
    pub settings_fn: Arc<dyn Fn() -> Settings + Send + Sync>,
    pub rate_limiter: RateLimiter,
    pub store: Arc<SharedStore>,
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
        .layer(axum::middleware::from_fn(api_auth_middleware))
        .layer(axum::middleware::from_fn(security_headers))
}

pub async fn start() -> Result<(), String> {
    let settings = load_settings();
    let state = AppState {
        http_client: build_http_client(),
        settings_fn: Arc::new(load_settings),
        rate_limiter: RateLimiter::from_env(),
        store: Arc::new(SharedStore::new(&settings)),
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
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| e.to_string())
}

/// Wait for SIGINT (Ctrl-C) or SIGTERM, then let in-flight requests drain
/// before axum::serve returns (A4). Docker sends SIGTERM; shells send SIGINT.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
    tracing::info!("shutdown signal received — draining in-flight requests");
}

async fn security_headers(request: Request<axum::body::Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    // Handlers may set a more specific CSP (e.g. token injection adds a script hash).
    if !headers.contains_key(header::CONTENT_SECURITY_POLICY) {
        headers.insert(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(CSP),
        );
    }
    if !headers.contains_key(header::X_CONTENT_TYPE_OPTIONS) {
        headers.insert(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        );
    }
    if !headers.contains_key(header::REFERRER_POLICY) {
        headers.insert(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        );
    }
    response
}

async fn api_auth_middleware(request: Request<axum::body::Body>, next: Next) -> Response {
    let path = request.uri().path().to_string();
    if path_requires_token(&path) {
        let auth = request
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok());
        let x_token = request
            .headers()
            .get("x-netrail-token")
            .and_then(|v| v.to_str().ok());
        if let Err(err) = check_request_token(auth, x_token) {
            return ApiError::from(err).into_response();
        }
    }
    next.run(request).await
}

/// Inline token script + its CSP sha256 hash, so `script-src 'self'` can
/// whitelist exactly this script (blocking every other inline script).
fn token_script_parts(token: &str) -> (String, String) {
    // Escape for JS string in double quotes.
    let escaped = token.replace('\\', "\\\\").replace('"', "\\\"");
    let content = format!("window.NETRAIL_API_TOKEN=\"{escaped}\";");
    let snippet = format!("<script>{content}</script>");
    let digest = sha2::Sha256::digest(content.as_bytes());
    let b64 = base64::engine::general_purpose::STANDARD.encode(digest);
    (snippet, format!("'sha256-{b64}'"))
}

/// Inject the API token into the UI page. Returns the HTML plus the CSP
/// script hash when injection happened (None otherwise).
fn inject_token_script(html: &str) -> (String, Option<String>) {
    let Some(token) = auth::api_token_from_env() else {
        return (html.to_string(), None);
    };
    if !inject_ui_token() {
        return (html.to_string(), None);
    }
    let (snippet, csp_hash) = token_script_parts(&token);
    if let Some(idx) = html.find("</head>") {
        let mut out = String::with_capacity(html.len() + snippet.len());
        out.push_str(&html[..idx]);
        out.push_str(&snippet);
        out.push_str(&html[idx..]);
        (out, Some(csp_hash))
    } else {
        (format!("{snippet}{html}"), Some(csp_hash))
    }
}

fn csp_with_script_hash(csp_hash: &str) -> HeaderValue {
    let csp = CSP.replace("script-src 'self'", &format!("script-src 'self' {csp_hash}"));
    HeaderValue::from_str(&csp).unwrap_or_else(|_| HeaderValue::from_static(CSP))
}

async fn index() -> impl IntoResponse {
    let path = static_dir().join("index.html");
    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            let html = String::from_utf8_lossy(&bytes);
            let (body, script_hash) = inject_token_script(&html);
            let mut response = (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                body,
            )
                .into_response();
            if let Some(hash) = script_hash {
                response
                    .headers_mut()
                    .insert(header::CONTENT_SECURITY_POLICY, csp_with_script_hash(&hash));
            }
            response
        }
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
    let mut history = state
        .store
        .with_store(&settings, |store| store.stats())
        .unwrap_or_else(|| serde_json::json!({ "enabled": false }));
    if let serde_json::Value::Object(ref mut map) = history {
        map.insert("encrypt_requested".into(), encrypt_requested.into());
        map.insert("encryption_active".into(), encryption_ok.into());
        map.insert(
            "encryption_state".into(),
            crate::history::cipher_state(encrypt_requested, encryption_ok).into(),
        );
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

    let backend_names: Vec<&str> = backends.iter().map(|b| b.name()).collect();
    let brave_key_present = std::env::var("BRAVE_SEARCH_API_KEY")
        .or_else(|_| std::env::var("NETRAIL_BRAVE_API_KEY"))
        .is_ok();
    let searxng_configured = settings.searxng_url.is_some()
        || settings.backends.iter().any(|b| b.id == "searxng" && b.url.is_some());
    let mut recovery_hints: Vec<&str> = Vec::new();
    if !searxng_configured {
        recovery_hints.push("Set NETRAIL_SEARXNG_URL to a SearXNG instance for self-hosted metasearch.");
    }
    if !settings.brave_enabled || !brave_key_present {
        recovery_hints.push("Export BRAVE_SEARCH_API_KEY and enable Brave in settings for richer web results.");
    }

    Json(serde_json::json!({
        "status": "ok",
        "version": VERSION,
        "telemetry": "none",
        "api_contract": API_CONTRACT,
        "backends_configured": backend_names,
        "search_recovery": {
            "searxng_configured": searxng_configured,
            "brave_key_present": brave_key_present,
            "brave_enabled": settings.brave_enabled,
            "hints": recovery_hints,
        },
        "default_provenance": "ddgs → DuckDuckGo metasearch → primarily Bing index",
        "history": history,
        "sandbox": if is_flatpak() { "flatpak" } else { "native" },
        "auth": {
            "token_required": auth::token_required(),
        },
        "strict_backend_urls": settings.strict_backend_urls
            || crate::config::strict_backend_urls_from_env(),
        "audit_log": audit::enabled(),
        "rate_limit": state.rate_limiter.status_json(),
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

/// Strong ETag over the canonical settings JSON, so concurrent writers can
/// detect last-writer-wins clobbering via `If-Match` (A6).
fn settings_etag(settings: &Settings) -> String {
    let bytes = serde_json::to_vec(settings).unwrap_or_default();
    let digest = sha2::Sha256::digest(&bytes);
    let encoded = base64::engine::general_purpose::STANDARD.encode(digest);
    format!("\"{encoded}\"")
}

async fn get_settings(State(state): State<AppState>) -> Response {
    let settings = (state.settings_fn)();
    ([(header::ETAG, settings_etag(&settings))], Json(settings)).into_response()
}

fn ensure_mutable() -> Result<(), ApiError> {
    if readonly_mode() {
        return Err(NetRailError::Readonly {
            code: "READONLY_MODE",
            message: "Read-only mode: mutations are disabled (NETRAIL_READONLY=1).".into(),
        }
        .into());
    }
    Ok(())
}

async fn put_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Json<Settings>, JsonRejection>,
) -> Result<Response, ApiError> {
    ensure_mutable()?;
    let Json(body) = body?;
    let identity = request_identity(&headers);
    if let Some(if_match) = headers.get(header::IF_MATCH).and_then(|v| v.to_str().ok()) {
        let current = (state.settings_fn)();
        if if_match.trim() != settings_etag(&current) {
            return Err(NetRailError::Conflict {
                code: "SETTINGS_CONFLICT",
                message: "Settings changed since read (ETag mismatch). Re-fetch and retry.".into(),
            }
            .into());
        }
    }
    state.rate_limiter.check_mutate(&identity)?;
    let saved = save_settings(&body)?;
    audit::log_event("settings.put", serde_json::json!({ "ok": true }));
    Ok(([(header::ETAG, settings_etag(&saved))], Json(saved)).into_response())
}

/// Rate-limit bucket key for the current request (A9): token hash when auth
/// is on, "anonymous" otherwise.
fn request_identity(headers: &HeaderMap) -> String {
    crate::auth::client_identity(
        headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()),
        headers.get("x-netrail-token").and_then(|v| v.to_str().ok()),
    )
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
    headers: HeaderMap,
    body: Result<Json<SearchRequest>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Json(body) = body?;
    state.rate_limiter.check_search(&request_identity(&headers))?;
    let query = body.query.trim();
    if query.is_empty() || query.len() > 500 {
        return Err(NetRailError::InvalidQuery {
            code: "QUERY_INVALID",
            message: "Query must be 1-500 characters.".into(),
        }
        .into());
    }
    if !(1..=50).contains(&body.max_results) {
        return Err(NetRailError::InvalidConfig {
            code: "CONFIG_MAX_RESULTS",
            message: "max_results must be between 1 and 50.".into(),
        }
        .into());
    }
    let settings = (state.settings_fn)();
    let payload = search::search(
        &state.http_client,
        query,
        &body.mode,
        body.max_results,
        &settings,
        &state.store,
    )
    .await?;
    audit::log_event(
        "search",
        serde_json::json!({
            "mode": body.mode,
            "query_len": query.len(),
            "max_results": body.max_results,
        }),
    );
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
    headers: HeaderMap,
    body: Result<Json<OpenRequest>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Json(body) = body?;
    state.rate_limiter.check_open(&request_identity(&headers))?;

    // Audit attempted-but-blocked opens (validation, DNS pin, browser spawn)
    // alongside the successful `open` event (NR blocked-opens finding).
    let raw_url = body.url.clone();
    let outcome = (|| -> Result<Json<serde_json::Value>, ApiError> {
        let safe_url = validate_open_url(&body.url)?;
        pin_open_host(&safe_url, resolve_host_ips)?;
        let mut settings = (state.settings_fn)();
        if let Some(id) = body.browser_id {
            settings.browser_id = Some(id);
        }
        if body.private_mode {
            settings.private_mode = true;
        }

        let result = open_url(&safe_url, &settings)?;
        state.store.with_store(&settings, |store| {
            let _ = store.record_visit(
                &safe_url,
                body.result_id,
                settings.browser_id.as_deref(),
                settings.private_mode,
            );
        });
        audit::log_event(
            "open",
            serde_json::json!({
                "url_host": url::Url::parse(&safe_url).ok().and_then(|u| u.host_str().map(str::to_string)),
                "private_mode": body.private_mode,
            }),
        );
        Ok(Json(serde_json::to_value(result).unwrap_or_default()))
    })();

    if let Err(err) = &outcome {
        audit_open_blocked(&raw_url, err.code, &err.detail);
    }
    outcome
}

fn audit_open_blocked(raw_url: &str, code: &str, detail: &str) {
    let host = url::Url::parse(raw_url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string));
    audit::log_event(
        "open.blocked",
        serde_json::json!({
            "url_host": host,
            "code": code,
            "detail": detail,
        }),
    );
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
    let cleaned = FTS_STRIP.replace_all(q, " ").trim().to_string();
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
    params: Result<Query<HistoryQuery>, QueryRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Query(params) = params?;
    let settings = (state.settings_fn)();
    let fts_q = params.q.as_deref().map(fts_query);
    let payload = state
        .store
        .with_store(&settings, |store| {
            store.list_history(fts_q.as_deref(), params.limit.clamp(1, 200), params.offset)
        })
        .ok_or_else(history_disabled_error)?
        ?;
    Ok(Json(payload))
}

async fn delete_history_entry(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(query_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    ensure_mutable()?;
    state.rate_limiter.check_mutate(&request_identity(&headers))?;
    let settings = (state.settings_fn)();
    let deleted = state
        .store
        .with_store(&settings, |store| store.delete_history_entry(query_id))
        .ok_or_else(history_disabled_error)?
        ?;
    if !deleted {
        return Err(NetRailError::NotFound {
            code: "HISTORY_ENTRY_NOT_FOUND",
            entity: format!("history entry {query_id}"),
        }
        .into());
    }
    audit::log_event(
        "history.delete",
        serde_json::json!({ "query_id": query_id }),
    );
    Ok(Json(serde_json::json!({
        "status": "ok",
        "deleted_id": query_id,
    })))
}

async fn purge_history(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    ensure_mutable()?;
    state.rate_limiter.check_mutate(&request_identity(&headers))?;
    let settings = (state.settings_fn)();
    let count = state
        .store
        .with_store(&settings, |store| store.purge_all_history())
        .ok_or_else(history_disabled_error)?
        ?;
    audit::log_event("history.purge", serde_json::json!({ "purged": count }));
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
    let items = state
        .store
        .with_store(&settings, |store| store.list_collections())
        .ok_or_else(history_disabled_error)?
        ?;
    Ok(Json(items))
}

async fn create_collection(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Json<CollectionCreate>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    ensure_mutable()?;
    let Json(body) = body?;
    state.rate_limiter.check_mutate(&request_identity(&headers))?;
    let settings = (state.settings_fn)();
    let name = body.name.trim();
    if name.is_empty() || name.len() > 120 {
        return Err(NetRailError::InvalidConfig {
            code: "COLLECTION_NAME_INVALID",
            message: "Collection name must be 1-120 characters.".into(),
        }
        .into());
    }
    let created = state
        .store
        .with_store(&settings, |store| store.create_collection(name))
        .ok_or_else(history_disabled_error)?
        ?;
    audit::log_event("collection.create", serde_json::json!({ "name_len": name.len() }));
    Ok(Json(created))
}

#[derive(Deserialize)]
struct CollectionItemCreate {
    url: String,
    title: String,
    notes: Option<String>,
}

async fn add_collection_item(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(collection_id): Path<i64>,
    body: Result<Json<CollectionItemCreate>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    ensure_mutable()?;
    let Json(body) = body?;
    state.rate_limiter.check_mutate(&request_identity(&headers))?;
    let settings = (state.settings_fn)();
    let safe_url = validate_open_url(&body.url)?;
    let title = body.title.trim();
    if title.is_empty() || title.len() > 500 {
        return Err(NetRailError::InvalidConfig {
            code: "COLLECTION_ITEM_TITLE_INVALID",
            message: "Title must be 1-500 characters.".into(),
        }
        .into());
    }
    if let Some(ref notes) = body.notes {
        if notes.len() > 2000 {
            return Err(NetRailError::InvalidConfig {
                code: "COLLECTION_ITEM_NOTES_INVALID",
                message: "Notes must be at most 2000 characters.".into(),
            }
            .into());
        }
    }
    let item = state
        .store
        .with_store(&settings, |store| {
            store.add_collection_item(collection_id, &safe_url, title, body.notes.as_deref())
        })
        .ok_or_else(history_disabled_error)?
        ?;
    audit::log_event(
        "collection.item.add",
        serde_json::json!({
            "collection_id": collection_id,
            "url_host": url::Url::parse(&safe_url).ok().and_then(|u| u.host_str().map(str::to_string)),
        }),
    );
    Ok(Json(item))
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
    params: Result<Query<ExportQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let Query(params) = params?;
    let settings = (state.settings_fn)();
    let fmt = if params.fmt == "csv" { "csv" } else { "json" };
    let content = state
        .store
        .with_store(&settings, |store| store.export_collection(collection_id, fmt))
        .ok_or_else(history_disabled_error)?
        ?;
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

/// Map axum JSON extractor rejections onto the typed error contract, mirroring
/// Python's `request_validation_handler` (netrail/main.py). Every 4xx from the
/// API must be `{code, detail, status}` — never plain-text extractor output.
impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        let text = rejection.body_text();
        let lower = text.to_lowercase();
        let err: NetRailError = if lower.contains("missing field `query`")
            || (lower.contains("query") && !lower.contains("max_results"))
        {
            NetRailError::InvalidQuery {
                code: "QUERY_INVALID",
                message: "Query must be 1-500 characters.".into(),
            }
        } else if lower.contains("missing field `url`") {
            NetRailError::InvalidOpenUrl {
                code: "OPEN_URL_INVALID",
                message: "URL is required.".into(),
            }
        } else if lower.contains("max_results") {
            NetRailError::InvalidConfig {
                code: "CONFIG_MAX_RESULTS",
                message: "max_results must be between 1 and 50.".into(),
            }
        } else if lower.contains("`mode`") {
            NetRailError::InvalidQuery {
                code: "QUERY_INVALID",
                message: "mode must be 'web' or 'images'.".into(),
            }
        } else if lower.contains("missing field `name`") {
            NetRailError::InvalidConfig {
                code: "COLLECTION_NAME_INVALID",
                message: "Collection name must be 1-120 characters.".into(),
            }
        } else if lower.contains("title") {
            NetRailError::InvalidConfig {
                code: "COLLECTION_ITEM_TITLE_INVALID",
                message: "Title must be 1-500 characters.".into(),
            }
        } else if lower.contains("notes") {
            NetRailError::InvalidConfig {
                code: "COLLECTION_ITEM_NOTES_INVALID",
                message: "Notes must be at most 2000 characters.".into(),
            }
        } else {
            NetRailError::InvalidConfig {
                code: "REQUEST_INVALID",
                message: text,
            }
        };
        err.into()
    }
}

impl From<QueryRejection> for ApiError {
    fn from(rejection: QueryRejection) -> Self {
        NetRailError::InvalidConfig {
            code: "REQUEST_INVALID",
            message: rejection.body_text(),
        }
        .into()
    }
}

fn history_disabled_error() -> ApiError {
    NetRailError::InvalidConfig {
        code: "HISTORY_DISABLED",
        message: "History is disabled in settings.".into(),
    }
    .into()
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
            detail: err.detail_message(),
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

#[cfg(test)]
mod open_blocked_audit_tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn audit_open_blocked_writes_typed_event() {
        let dir = TempDir::new().unwrap();
        std::env::set_var("NETRAIL_AUDIT_LOG_PATH", dir.path().join("audit.log"));
        audit::reset_for_tests();

        audit_open_blocked("http://127.0.0.1/", "OPEN_URL_LOCALHOST", "blocked");

        let content = std::fs::read_to_string(dir.path().join("audit.log")).unwrap();
        let ev: serde_json::Value = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(ev["action"], "open.blocked");
        assert_eq!(ev["detail"]["code"], "OPEN_URL_LOCALHOST");
        assert_eq!(ev["detail"]["url_host"], "127.0.0.1");
        assert_eq!(ev["detail"]["detail"], "blocked");

        std::env::remove_var("NETRAIL_AUDIT_LOG_PATH");
        audit::reset_for_tests();
    }

    #[test]
    #[serial]
    fn audit_open_blocked_unparseable_url_has_null_host() {
        let dir = TempDir::new().unwrap();
        std::env::set_var("NETRAIL_AUDIT_LOG_PATH", dir.path().join("audit.log"));
        audit::reset_for_tests();

        audit_open_blocked("http://%B4.3511866278/x", "OPEN_URL_INVALID", "invalid");

        let content = std::fs::read_to_string(dir.path().join("audit.log")).unwrap();
        let ev: serde_json::Value = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(ev["action"], "open.blocked");
        assert_eq!(ev["detail"]["code"], "OPEN_URL_INVALID");
        assert!(ev["detail"]["url_host"].is_null());

        std::env::remove_var("NETRAIL_AUDIT_LOG_PATH");
        audit::reset_for_tests();
    }
}

#[cfg(test)]
mod token_csp_tests {
    use super::*;

    #[test]
    fn csp_hash_matches_injected_script_content() {
        let (snippet, hash) = token_script_parts("tok-42\"quote\\back");
        let content = snippet
            .strip_prefix("<script>")
            .and_then(|s| s.strip_suffix("</script>"))
            .expect("script wrapper");
        let digest = sha2::Sha256::digest(content.as_bytes());
        let b64 = base64::engine::general_purpose::STANDARD.encode(digest);
        assert_eq!(hash, format!("'sha256-{b64}'"));
    }

    #[test]
    fn csp_includes_failsafe_script_hash() {
        // The inline splash failsafe in index.html is allowed by a sha256 hash
        // in the static CSP; keep the two in lock-step.
        let html = std::fs::read_to_string(static_dir().join("index.html")).unwrap();
        let start = html.find("<script>").expect("inline script open tag");
        let rest = &html[start + "<script>".len()..];
        let content = &rest[..rest.find("</script>").expect("inline script close tag")];
        let digest = sha2::Sha256::digest(content.as_bytes());
        let b64 = base64::engine::general_purpose::STANDARD.encode(digest);
        assert!(
            CSP.contains(&format!("'sha256-{b64}'")),
            "CSP must whitelist the inline failsafe script hash; update security::CSP"
        );
    }

    #[test]
    fn csp_with_script_hash_keeps_other_directives() {
        let csp = csp_with_script_hash("'sha256-abc'").to_str().unwrap().to_string();
        assert!(csp.contains("script-src 'self' 'sha256-abc'"));
        assert!(csp.contains("img-src 'self' https: data:"));
        assert!(csp.contains("connect-src 'self'"));
        // script-src must not regain unsafe-inline; style-src keeps it for the UI.
        assert!(!csp.contains("script-src 'self' 'unsafe-inline'"));
    }

    #[test]
    fn no_token_means_no_injection() {
        let prev = auth::api_token_from_env();
        std::env::remove_var("NETRAIL_API_TOKEN");
        let (html, hash) = inject_token_script("<!DOCTYPE html><head></head><body></body>");
        assert_eq!(html, "<!DOCTYPE html><head></head><body></body>");
        assert!(hash.is_none());
        if let Some(token) = prev {
            std::env::set_var("NETRAIL_API_TOKEN", token);
        }
    }
}
from __future__ import annotations

import base64
import hashlib
import os
import re
import threading
import webbrowser
from contextlib import asynccontextmanager
from typing import Any, Literal

import uvicorn
from fastapi import FastAPI, Query, Request, Response
from fastapi.exceptions import RequestValidationError
from fastapi.responses import FileResponse, HTMLResponse, JSONResponse, PlainTextResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, Field

from netrail import __version__
from netrail import audit
from netrail import rate_limit
from netrail.auth import (
    api_token_from_env,
    check_request_token,
    inject_ui_token,
    path_requires_token,
    token_required,
)
from netrail.errors import NetRailError
from netrail.backends.registry import get_enabled_backends
from netrail.browsers import discover_browsers, open_url
from netrail.config import load_settings, save_settings, strict_backend_urls_from_env
from netrail.docs_content import asset_path, load_doc
from netrail.history.store import get_store, init_history_on_startup
from netrail.runtime import is_flatpak, static_dir
from netrail.search import search
from netrail.security import validate_open_url

STATIC_DIR = static_dir()

CSP = (
    "default-src 'self'; "
    "script-src 'self'; "
    "style-src 'self' 'unsafe-inline'; "
    "img-src 'self' https: data:; "
    "connect-src 'self'; "
    "frame-ancestors 'none'; "
    "base-uri 'self'; "
    "form-action 'self'"
)


@asynccontextmanager
async def lifespan(_app: FastAPI):
    init_history_on_startup()
    yield


app = FastAPI(
    title="NetRail",
    description="Local research console. No telemetry. No accounts.",
    version=__version__,
    lifespan=lifespan,
)

app.mount("/static", StaticFiles(directory=STATIC_DIR), name="static")


@app.exception_handler(NetRailError)
async def netrail_error_handler(_request: Request, exc: NetRailError) -> JSONResponse:
    return JSONResponse(status_code=exc.status, content=exc.to_dict())


@app.exception_handler(RequestValidationError)
async def request_validation_handler(
    _request: Request, exc: RequestValidationError
) -> JSONResponse:
    """Map FastAPI/Pydantic 422 into stable NetRail `{code, detail, status}` (ADV-04)."""
    errors = exc.errors()
    locs = [tuple(err.get("loc", ())) for err in errors]
    messages = [str(err.get("msg", "Invalid request")) for err in errors]
    detail = messages[0] if messages else "Invalid request."

    def _has_field(*names: str) -> bool:
        return any(any(part in names for part in loc) for loc in locs)

    if _has_field("query"):
        err = NetRailError(
            "QUERY_INVALID",
            "Query must be 1-500 characters.",
            status=400,
        )
    elif _has_field("max_results"):
        err = NetRailError(
            "CONFIG_MAX_RESULTS",
            "max_results must be between 1 and 50.",
            status=400,
        )
    elif _has_field("mode"):
        err = NetRailError(
            "QUERY_INVALID",
            "mode must be 'web' or 'images'.",
            status=400,
        )
    elif _has_field("url"):
        err = NetRailError(
            "OPEN_URL_INVALID",
            "URL is required.",
            status=400,
        )
    elif _has_field("name"):
        err = NetRailError(
            "COLLECTION_NAME_INVALID",
            "Collection name must be 1-120 characters.",
            status=400,
        )
    elif _has_field("title"):
        err = NetRailError(
            "COLLECTION_ITEM_TITLE_INVALID",
            "Title must be 1-500 characters.",
            status=400,
        )
    elif _has_field("notes"):
        err = NetRailError(
            "COLLECTION_ITEM_NOTES_INVALID",
            "Notes must be at most 2000 characters.",
            status=400,
        )
    else:
        err = NetRailError("REQUEST_INVALID", detail, status=400)
    return JSONResponse(status_code=err.status, content=err.to_dict())


@app.middleware("http")
async def api_auth_middleware(request: Request, call_next) -> Response:
    path = request.url.path
    if path_requires_token(path):
        try:
            check_request_token(
                request.headers.get("authorization"),
                request.headers.get("x-netrail-token"),
            )
        except NetRailError as exc:
            return JSONResponse(status_code=exc.status, content=exc.to_dict())
    return await call_next(request)


@app.middleware("http")
async def security_headers(request: Request, call_next) -> Response:
    response = await call_next(request)
    if "Content-Security-Policy" not in response.headers:
        response.headers["Content-Security-Policy"] = CSP
    if "X-Content-Type-Options" not in response.headers:
        response.headers["X-Content-Type-Options"] = "nosniff"
    if "Referrer-Policy" not in response.headers:
        response.headers["Referrer-Policy"] = "no-referrer"
    return response


class SearchRequest(BaseModel):
    query: str = Field(min_length=1, max_length=500)
    mode: Literal["web", "images"] = "web"
    max_results: int = Field(default=25, ge=1, le=50)


class OpenRequest(BaseModel):
    url: str = Field(min_length=1)
    browser_id: str | None = None
    private_mode: bool = False
    result_id: int | None = None


class BackendConfigModel(BaseModel):
    id: str
    enabled: bool = True
    url: str | None = None
    api_key_env: str | None = None


class SettingsModel(BaseModel):
    browser_id: str | None = None
    private_mode: bool = False
    max_results: int = Field(default=25, ge=1, le=50)
    backend_order: list[str] = Field(default_factory=lambda: ["searxng", "ddgs", "brave"])
    ddgs_enabled: bool = True
    searxng_url: str | None = None
    brave_enabled: bool = False
    search_strategy: Literal["fanout", "fallback"] = "fanout"
    backends: list[BackendConfigModel] = Field(
        default_factory=lambda: [
            BackendConfigModel(id="searxng", enabled=True),
            BackendConfigModel(id="ddgs", enabled=True),
            BackendConfigModel(
                id="brave",
                enabled=False,
                api_key_env="BRAVE_SEARCH_API_KEY",
            ),
        ]
    )
    history_enabled: bool = True
    history_encrypt: bool = True
    history_ttl_days: int = Field(default=90, ge=0, le=3650)
    strict_backend_urls: bool = False


class CollectionCreate(BaseModel):
    name: str = Field(min_length=1, max_length=120)


class CollectionItemCreate(BaseModel):
    url: str = Field(min_length=1)
    title: str = Field(min_length=1, max_length=500)
    notes: str | None = Field(default=None, max_length=2000)


def _require_store():
    store = get_store()
    if store is None:
        raise NetRailError(
            "HISTORY_DISABLED",
            "History is disabled in settings.",
        )
    return store


def _fts_query(q: str) -> str:
    cleaned = re.sub(r"[^\w\s-]", " ", q, flags=re.UNICODE).strip()
    if not cleaned:
        return '""'
    return " ".join(f'"{part}"' for part in cleaned.split())


def _token_script_csp_hash(token: str) -> str:
    """CSP sha256 hash of the exact inline token script, so `script-src 'self'`
    can whitelist it while still blocking every other inline script."""
    escaped = token.replace("\\", "\\\\").replace('"', '\\"')
    content = f'window.NETRAIL_API_TOKEN="{escaped}";'
    digest = hashlib.sha256(content.encode("utf-8")).digest()
    return f"'sha256-{base64.b64encode(digest).decode()}'"


@app.get("/")
async def index() -> Response:
    path = STATIC_DIR / "index.html"
    if not path.is_file():
        return HTMLResponse(
            "<h1>NetRail UI assets missing</h1>",
            status_code=404,
        )
    html = path.read_text(encoding="utf-8")
    token = api_token_from_env()
    headers: dict[str, str] = {}
    if token and inject_ui_token():
        escaped = token.replace("\\", "\\\\").replace('"', '\\"')
        snippet = f'<script>window.NETRAIL_API_TOKEN="{escaped}";</script>'
        if "</head>" in html:
            html = html.replace("</head>", snippet + "</head>", 1)
        else:
            html = snippet + html
        headers["Content-Security-Policy"] = CSP.replace(
            "script-src 'self'",
            f"script-src 'self' {_token_script_csp_hash(token)}",
        )
    return HTMLResponse(html, headers=headers)


@app.get("/api/health")
async def health() -> dict[str, Any]:
    from netrail.history.crypto import encryption_active, ensure_encryption_key

    settings = load_settings()
    backends = get_enabled_backends(settings)
    encrypt_requested = bool(settings.get("history_encrypt", True))
    if encrypt_requested:
        ensure_encryption_key()
    encryption_ok = encryption_active()
    from netrail.history.store import encryption_degraded, encryption_degraded_message

    store = get_store()
    history = store.stats() if store else {"enabled": False}
    history["encrypt_requested"] = encrypt_requested
    history["encryption_active"] = encryption_ok
    if encrypt_requested and not encryption_ok:
        history["encryption_warning"] = (
            "History encryption is enabled but no key is available."
        )
    if encryption_degraded():
        history["encryption_degraded"] = True
        history["encryption_degraded_message"] = encryption_degraded_message()

    brave_key_present = bool(
        os.environ.get("BRAVE_SEARCH_API_KEY") or os.environ.get("NETRAIL_BRAVE_API_KEY")
    )
    searxng_configured = bool(settings.get("searxng_url")) or any(
        b.get("id") == "searxng" and b.get("url")
        for b in (settings.get("backends") or [])
        if isinstance(b, dict)
    )
    recovery_hints: list[str] = []
    if not searxng_configured:
        recovery_hints.append(
            "Set NETRAIL_SEARXNG_URL to a SearXNG instance for self-hosted metasearch."
        )
    if not settings.get("brave_enabled") or not brave_key_present:
        recovery_hints.append(
            "Export BRAVE_SEARCH_API_KEY and enable Brave in settings for richer web results."
        )

    return {
        "status": "ok",
        "version": __version__,
        "telemetry": "none",
        "backends_configured": [b.name for b in backends],
        "search_recovery": {
            "searxng_configured": searxng_configured,
            "brave_key_present": brave_key_present,
            "brave_enabled": bool(settings.get("brave_enabled")),
            "hints": recovery_hints,
        },
        "default_provenance": "ddgs → DuckDuckGo metasearch → primarily Bing index",
        "history": history,
        "sandbox": "flatpak" if is_flatpak() else "native",
        "api_contract": "1.4",
        "auth": {"token_required": token_required()},
        "strict_backend_urls": bool(settings.get("strict_backend_urls"))
        or strict_backend_urls_from_env(),
        "audit_log": audit.enabled(),
        "rate_limit": rate_limit.status_dict(),
    }


@app.get("/api/backends")
async def list_backends() -> list[dict[str, Any]]:
    settings = load_settings()
    return [
        {
            "name": backend.name,
            "provenance": backend.provenance,
            "available": backend.is_available(),
            "supports_operators": sorted(backend.supports_operators),
        }
        for backend in get_enabled_backends(settings)
    ]


@app.get("/api/browsers")
async def list_browsers() -> list[dict[str, Any]]:
    return [
        {
            "id": browser.id,
            "name": browser.name,
            "executable": browser.executable,
            "supports_private": browser.private_flag is not None,
        }
        for browser in discover_browsers()
    ]


@app.get("/api/settings")
async def get_settings() -> dict[str, Any]:
    return load_settings()


@app.put("/api/settings")
async def put_settings(settings: SettingsModel) -> dict[str, Any]:
    rate_limit.check_mutate()
    saved = save_settings(settings.model_dump())
    audit.log_event("settings.put", {"ok": True})
    return saved


@app.get("/api/docs/{slug}")
async def get_doc(slug: str) -> dict[str, str]:
    return load_doc(slug)


@app.get("/api/docs/assets/{filename}")
async def get_doc_asset(filename: str) -> FileResponse:
    path = asset_path(filename)
    if path is None:
        raise NetRailError("DOC_ASSET_NOT_FOUND", "Document asset not found.", status=404)
    return FileResponse(path)


@app.post("/api/search")
async def run_search(request: SearchRequest) -> dict[str, Any]:
    rate_limit.check_search()
    payload = search(
        query=request.query,
        mode=request.mode,
        max_results=request.max_results,
    )
    audit.log_event(
        "search",
        {
            "mode": request.mode,
            "query_len": len(request.query),
            "max_results": request.max_results,
        },
    )
    return payload


@app.post("/api/open")
async def open_link(request: OpenRequest) -> dict[str, str]:
    rate_limit.check_open()
    safe_url = validate_open_url(request.url)

    settings = load_settings()
    browser_id = request.browser_id or settings.get("browser_id")
    private_mode = request.private_mode or bool(settings.get("private_mode"))

    try:
        result = open_url(safe_url, browser_id=browser_id, private_mode=private_mode)
    except RuntimeError as exc:
        try:
            webbrowser.open(safe_url)
            result = {"browser": "system default", "mode": "normal", "url": safe_url}
        except Exception as fallback_exc:  # noqa: BLE001
            raise NetRailError(
                "BROWSER_NOT_FOUND",
                str(exc) or "No web browser found on this system.",
                status=500,
            ) from fallback_exc

    store = get_store()
    if store:
        store.record_visit(
            safe_url,
            result_id=request.result_id,
            browser_id=browser_id,
            private_mode=private_mode,
        )

    from urllib.parse import urlparse

    audit.log_event(
        "open",
        {
            "url_host": urlparse(safe_url).hostname,
            "private_mode": private_mode,
        },
    )
    return result


@app.get("/api/history")
async def get_history(
    q: str | None = None,
    limit: int = Query(default=50, ge=1, le=200),
    offset: int = Query(default=0, ge=0),
) -> dict[str, Any]:
    store = _require_store()
    fts_q = _fts_query(q) if q else None
    return store.list_history(q=fts_q, limit=limit, offset=offset)


@app.delete("/api/history/{query_id}")
async def delete_history_entry(query_id: int) -> dict[str, Any]:
    rate_limit.check_mutate()
    store = _require_store()
    if not store.delete_history_entry(query_id):
        raise NetRailError(
            "HISTORY_ENTRY_NOT_FOUND",
            f"history entry {query_id}",
            status=404,
        )
    audit.log_event("history.delete", {"query_id": query_id})
    return {"status": "ok", "deleted_id": query_id}


@app.delete("/api/history")
async def purge_history() -> dict[str, Any]:
    rate_limit.check_mutate()
    store = _require_store()
    count = store.purge_all_history()
    audit.log_event("history.purge", {"purged": count})
    return {"status": "ok", "purged": count}


@app.get("/api/collections")
async def list_collections() -> list[dict[str, Any]]:
    store = _require_store()
    return store.list_collections()


@app.post("/api/collections")
async def create_collection(body: CollectionCreate) -> dict[str, Any]:
    rate_limit.check_mutate()
    store = _require_store()
    created = store.create_collection(body.name)
    audit.log_event("collection.create", {"name_len": len(body.name)})
    return created


@app.post("/api/collections/{collection_id}/items")
async def add_collection_item(collection_id: int, body: CollectionItemCreate) -> dict[str, Any]:
    store = _require_store()
    safe_url = validate_open_url(body.url)
    return store.add_collection_item(
        collection_id,
        url=safe_url,
        title=body.title,
        notes=body.notes,
    )


@app.get("/api/collections/{collection_id}/export")
async def export_collection(
    collection_id: int,
    fmt: Literal["json", "csv"] = Query(default="json"),
) -> Response:
    store = _require_store()
    content = store.export_collection(collection_id, fmt=fmt)

    media = "application/json" if fmt == "json" else "text/csv"
    return PlainTextResponse(content=content, media_type=media)


def _schedule_ui_open() -> None:
    if os.getenv("NETRAIL_AUTO_OPEN", "true").lower() not in {"1", "true", "yes", "on"}:
        return

    def _open() -> None:
        try:
            webbrowser.open("http://127.0.0.1:7421")
        except Exception:  # noqa: BLE001
            pass

    threading.Timer(1.5, _open).start()


def main() -> None:
    _schedule_ui_open()
    uvicorn.run(
        "netrail.main:app",
        host="127.0.0.1",
        port=7421,
        log_level="warning",
    )


if __name__ == "__main__":
    main()
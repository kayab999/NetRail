from __future__ import annotations

import os
import re
import threading
import webbrowser
from contextlib import asynccontextmanager
from typing import Any, Literal

import uvicorn
from fastapi import FastAPI, HTTPException, Query, Request, Response
from fastapi.responses import FileResponse, PlainTextResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, Field

from netrail import __version__
from netrail.backends.registry import get_enabled_backends
from netrail.browsers import discover_browsers, open_url
from netrail.config import load_settings, save_settings
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


@app.middleware("http")
async def security_headers(request: Request, call_next) -> Response:
    response = await call_next(request)
    response.headers["Content-Security-Policy"] = CSP
    response.headers["X-Content-Type-Options"] = "nosniff"
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


class CollectionCreate(BaseModel):
    name: str = Field(min_length=1, max_length=120)


class CollectionItemCreate(BaseModel):
    url: str = Field(min_length=1)
    title: str = Field(min_length=1, max_length=500)
    notes: str | None = Field(default=None, max_length=2000)


def _require_store():
    store = get_store()
    if store is None:
        raise HTTPException(status_code=400, detail="History is disabled in settings.")
    return store


def _fts_query(q: str) -> str:
    cleaned = re.sub(r"[^\w\s-]", " ", q, flags=re.UNICODE).strip()
    if not cleaned:
        return '""'
    return " ".join(f'"{part}"' for part in cleaned.split())


@app.get("/")
async def index() -> FileResponse:
    return FileResponse(STATIC_DIR / "index.html")


@app.get("/api/health")
async def health() -> dict[str, Any]:
    settings = load_settings()
    backends = get_enabled_backends(settings)
    store = get_store()
    return {
        "status": "ok",
        "version": __version__,
        "telemetry": "none",
        "backends_configured": [b.name for b in backends],
        "default_provenance": "ddgs → DuckDuckGo metasearch → primarily Bing index",
        "history": store.stats() if store else {"enabled": False},
        "sandbox": "flatpak" if is_flatpak() else "native",
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
    return save_settings(settings.model_dump())


@app.post("/api/search")
async def run_search(request: SearchRequest) -> dict[str, Any]:
    try:
        return search(
            query=request.query,
            mode=request.mode,
            max_results=request.max_results,
        )
    except Exception as exc:  # noqa: BLE001
        raise HTTPException(status_code=502, detail=str(exc)) from exc


@app.post("/api/open")
async def open_link(request: OpenRequest) -> dict[str, str]:
    try:
        safe_url = validate_open_url(request.url)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

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
            raise HTTPException(status_code=500, detail=str(exc)) from fallback_exc

    store = get_store()
    if store:
        store.record_visit(
            safe_url,
            result_id=request.result_id,
            browser_id=browser_id,
            private_mode=private_mode,
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
    store = _require_store()
    if not store.delete_history_entry(query_id):
        raise HTTPException(status_code=404, detail="History entry not found.")
    return {"status": "ok", "deleted_id": query_id}


@app.delete("/api/history")
async def purge_history() -> dict[str, Any]:
    store = _require_store()
    count = store.purge_all_history()
    return {"status": "ok", "purged": count}


@app.get("/api/collections")
async def list_collections() -> list[dict[str, Any]]:
    store = _require_store()
    return store.list_collections()


@app.post("/api/collections")
async def create_collection(body: CollectionCreate) -> dict[str, Any]:
    store = _require_store()
    try:
        return store.create_collection(body.name)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@app.post("/api/collections/{collection_id}/items")
async def add_collection_item(collection_id: int, body: CollectionItemCreate) -> dict[str, Any]:
    store = _require_store()
    try:
        safe_url = validate_open_url(body.url)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    try:
        return store.add_collection_item(
            collection_id,
            url=safe_url,
            title=body.title,
            notes=body.notes,
        )
    except ValueError as exc:
        raise HTTPException(status_code=404, detail=str(exc)) from exc


@app.get("/api/collections/{collection_id}/export")
async def export_collection(
    collection_id: int,
    fmt: Literal["json", "csv"] = Query(default="json"),
) -> Response:
    store = _require_store()
    try:
        content = store.export_collection(collection_id, fmt=fmt)
    except ValueError as exc:
        raise HTTPException(status_code=404, detail=str(exc)) from exc

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
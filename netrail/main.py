from __future__ import annotations

import webbrowser
from pathlib import Path
from typing import Any, Literal

import uvicorn
from fastapi import FastAPI, HTTPException, Request, Response
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, Field

from netrail import __version__
from netrail.backends.registry import get_enabled_backends
from netrail.browsers import discover_browsers, open_url
from netrail.config import load_settings, save_settings
from netrail.search import search
from netrail.security import validate_open_url

STATIC_DIR = Path(__file__).parent / "static"

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

app = FastAPI(
    title="NetRail",
    description="Local research console. No telemetry. No accounts.",
    version=__version__,
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


class SettingsModel(BaseModel):
    browser_id: str | None = None
    private_mode: bool = False
    max_results: int = Field(default=25, ge=1, le=50)
    backend_order: list[str] = Field(default_factory=lambda: ["searxng", "ddgs"])
    ddgs_enabled: bool = True
    searxng_url: str | None = None


@app.get("/")
async def index() -> FileResponse:
    return FileResponse(STATIC_DIR / "index.html")


@app.get("/api/health")
async def health() -> dict[str, Any]:
    settings = load_settings()
    backends = get_enabled_backends(settings)
    return {
        "status": "ok",
        "version": __version__,
        "telemetry": "none",
        "backends_configured": [b.name for b in backends],
        "default_provenance": "ddgs → DuckDuckGo metasearch → primarily Bing index",
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
    except Exception as exc:  # noqa: BLE001 — surface provider errors to UI
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
        return open_url(safe_url, browser_id=browser_id, private_mode=private_mode)
    except RuntimeError as exc:
        try:
            webbrowser.open(safe_url)
            return {"browser": "system default", "mode": "normal", "url": safe_url}
        except Exception as fallback_exc:  # noqa: BLE001
            raise HTTPException(status_code=500, detail=str(exc)) from fallback_exc


def main() -> None:
    uvicorn.run(
        "netrail.main:app",
        host="127.0.0.1",
        port=7421,
        log_level="warning",
    )


if __name__ == "__main__":
    main()
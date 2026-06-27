from __future__ import annotations

import webbrowser
from pathlib import Path
from typing import Any, Literal

import uvicorn
from fastapi import FastAPI, HTTPException
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, Field

from netrail import __version__
from netrail.browsers import discover_browsers, open_url
from netrail.config import load_settings, save_settings
from netrail.search import search

STATIC_DIR = Path(__file__).parent / "static"

app = FastAPI(
    title="NetRail",
    description="Local research console. No telemetry. No accounts.",
    version=__version__,
)

app.mount("/static", StaticFiles(directory=STATIC_DIR), name="static")


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


@app.get("/")
async def index() -> FileResponse:
    return FileResponse(STATIC_DIR / "index.html")


@app.get("/api/health")
async def health() -> dict[str, str]:
    return {
        "status": "ok",
        "version": __version__,
        "telemetry": "none",
    }


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
        results = search(
            query=request.query,
            mode=request.mode,
            max_results=request.max_results,
        )
    except Exception as exc:  # noqa: BLE001 — surface provider errors to UI
        raise HTTPException(status_code=502, detail=str(exc)) from exc

    return {
        "query": request.query,
        "mode": request.mode,
        "count": len(results),
        "results": results,
    }


@app.post("/api/open")
async def open_link(request: OpenRequest) -> dict[str, str]:
    if not request.url.startswith(("http://", "https://")):
        raise HTTPException(status_code=400, detail="Only http(s) URLs are supported.")

    settings = load_settings()
    browser_id = request.browser_id or settings.get("browser_id")
    private_mode = request.private_mode or bool(settings.get("private_mode"))

    try:
        return open_url(request.url, browser_id=browser_id, private_mode=private_mode)
    except RuntimeError as exc:
        try:
            webbrowser.open(request.url)
            return {"browser": "system default", "mode": "normal", "url": request.url}
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
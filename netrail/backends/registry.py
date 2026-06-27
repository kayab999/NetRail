from __future__ import annotations

import logging
from typing import Any

from netrail.backends.ddgs import DDGSBackend
from netrail.backends.searxng import SearXNGBackend
from netrail.backends.types import SearchMode, SearchResponse, SearchResult
from netrail.config import load_settings

logger = logging.getLogger(__name__)


def get_enabled_backends(settings: dict[str, Any] | None = None) -> list[Any]:
    settings = settings or load_settings()
    backends: list[Any] = []
    order = settings.get("backend_order", ["searxng", "ddgs"])

    for backend_id in order:
        if backend_id == "ddgs" and settings.get("ddgs_enabled", True):
            backends.append(DDGSBackend())
        elif backend_id == "searxng":
            url = settings.get("searxng_url")
            if url:
                backends.append(SearXNGBackend(url))

    if not backends:
        backends.append(DDGSBackend())
    return backends


def _sovereignty_step(backends_used: list[str]) -> int:
    if "searxng" in backends_used:
        return 3
    if len(backends_used) > 1:
        return 2
    return 1


def _dedupe_results(results: list[SearchResult]) -> list[SearchResult]:
    seen: set[str] = set()
    unique: list[SearchResult] = []
    for item in results:
        key = item.url.rstrip("/").lower()
        if key in seen:
            continue
        seen.add(key)
        unique.append(item)
    return unique


def search_with_fallback(
    query: str,
    mode: SearchMode = "web",
    max_results: int = 25,
    settings: dict[str, Any] | None = None,
) -> SearchResponse:
    query = query.strip()
    if not query:
        return SearchResponse(query=query, mode=mode)

    max_results = max(1, min(max_results, 50))
    backends = get_enabled_backends(settings)
    errors: list[str] = []
    all_results: list[SearchResult] = []
    backends_used: list[str] = []
    provenance_chain: list[str] = []

    for backend in backends:
        if not backend.is_available():
            errors.append(f"{backend.name}: unavailable")
            continue
        try:
            batch = backend.search(query, mode, max_results)
            if batch:
                backends_used.append(backend.name)
                provenance_chain.append(backend.provenance)
                all_results.extend(batch)
        except Exception as exc:  # noqa: BLE001 — collect per-backend failures
            message = f"{backend.name}: {exc}"
            logger.warning(message)
            errors.append(message)

    merged = _dedupe_results(all_results)[:max_results]

    return SearchResponse(
        query=query,
        mode=mode,
        results=merged,
        backends_used=backends_used or ["none"],
        provenance_chain=provenance_chain or ["No backend returned results"],
        sovereignty_step=_sovereignty_step(backends_used),
        errors=errors,
    )
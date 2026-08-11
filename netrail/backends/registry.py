from __future__ import annotations

import logging
from concurrent.futures import ThreadPoolExecutor, TimeoutError as FuturesTimeout, as_completed
from typing import Any

from netrail.backends.brave import BraveBackend
from netrail.backends.ddgs import DDGSBackend
from netrail.backends.merge import dedupe_results, merge_fanout
from netrail.backends.searxng import SearXNGBackend
from netrail.backends.types import SearchMode, SearchResponse, SearchResult
from netrail.backends.wikipedia import WikipediaBackend
from netrail.config import load_settings, strict_backend_urls_from_env

logger = logging.getLogger(__name__)

# Match Rust FANOUT_DEADLINE (src-tauri/src/backends/mod.rs).
FANOUT_DEADLINE_SECONDS = 20.0


def get_enabled_backends(settings: dict[str, Any] | None = None) -> list[Any]:
    settings = settings or load_settings()
    # Backend fetches are made by this process: strict_backend_urls applies at
    # fetch time too (A-05), same value as the save-time default.
    strict_backend = bool(settings.get("strict_backend_urls")) or strict_backend_urls_from_env()
    backends: list[Any] = []

    structured = settings.get("backends") or []
    if structured:
        for entry in structured:
            if not entry.get("enabled", True):
                continue
            backend_id = entry.get("id")
            if backend_id == "ddgs":
                backends.append(DDGSBackend())
            elif backend_id == "searxng":
                url = entry.get("url") or settings.get("searxng_url")
                if url:
                    backends.append(SearXNGBackend(url, strict=strict_backend))
            elif backend_id == "brave":
                brave = BraveBackend.from_env_var(entry.get("api_key_env"))
                if brave:
                    backends.append(brave)
        if backends:
            return backends

    order = settings.get("backend_order", ["searxng", "ddgs", "brave"])
    for backend_id in order:
        if backend_id == "ddgs" and settings.get("ddgs_enabled", True):
            backends.append(DDGSBackend())
        elif backend_id == "searxng":
            url = settings.get("searxng_url")
            if url:
                backends.append(SearXNGBackend(url, strict=strict_backend))
        elif backend_id == "brave" and settings.get("brave_enabled"):
            brave = BraveBackend.from_env()
            if brave:
                backends.append(brave)

    if not backends:
        backends.append(DDGSBackend())
    return backends


def _sovereignty_step(backends_used: list[str]) -> int:
    if "brave" in backends_used or "searxng" in backends_used:
        return 3
    if len(backends_used) > 1:
        return 2
    return 1


def _query_backend(backend: Any, query: str, mode: SearchMode, max_results: int) -> tuple[str, str, list[SearchResult], str | None]:
    name = backend.name
    provenance = backend.provenance
    if not backend.is_available():
        return name, provenance, [], f"{name}: unavailable"
    try:
        batch = backend.search(query, mode, max_results)
        return name, provenance, batch, None
    except Exception as exc:  # noqa: BLE001
        return name, provenance, [], f"{name}: {exc}"


def search_with_fallback(
    query: str,
    mode: SearchMode = "web",
    max_results: int = 25,
    settings: dict[str, Any] | None = None,
) -> SearchResponse:
    settings = settings or load_settings()
    query = query.strip()
    if not query:
        return SearchResponse(query=query, mode=mode)

    max_results = max(1, min(max_results, 50))
    strategy = settings.get("search_strategy", "fanout")
    backends = get_enabled_backends(settings)

    errors: list[str] = []
    batches: list[tuple[str, list[SearchResult]]] = []
    backends_used: list[str] = []
    provenance_chain: list[str] = []

    pool = ThreadPoolExecutor(max_workers=max(1, len(backends)))
    futures = {}
    for backend in backends:
        future = pool.submit(_query_backend, backend, query, mode, max_results)
        futures[future] = backend
    try:
        for future in as_completed(futures, timeout=FANOUT_DEADLINE_SECONDS):
            name, provenance, batch, err = future.result()
            if err:
                logger.warning("backend search failed: %s", err)
                errors.append(err)
                continue
            if batch:
                backends_used.append(name)
                provenance_chain.append(provenance)
                batches.append((name, batch))
            else:
                logger.warning("%s returned zero parseable results", name)
                errors.append(f"{name}: returned no results")
    except FuturesTimeout:
        logger.warning("fanout timed out after %.0fs", FANOUT_DEADLINE_SECONDS)
        errors.append("fanout: timed out after 20 seconds")
        # Cancel queued work and stop waiting on the pool (QA-10): the old
        # `with ThreadPoolExecutor` blocked in __exit__ on hung/queued futures,
        # stretching the request wall time past the 20s deadline. Pending
        # futures are cancelled outright; already-running backend threads are
        # left to finish under their own HTTP timeouts (brave/searxng 12s,
        # wikipedia 15s, ddgs ~5s per engine), so nothing runs unbounded.
        for future in futures:
            future.cancel()
        pool.shutdown(wait=False, cancel_futures=True)
    else:
        pool.shutdown(wait=True)

    if strategy == "fallback":
        flat = [item for _, batch in batches for item in batch]
        merged = dedupe_results(flat)[:max_results]
    else:
        merged = merge_fanout(batches, max_results)

    if not merged and mode == "web":
        logger.info("fanout empty — activating wikipedia fallback for %r", query)
        wiki = WikipediaBackend()
        try:
            wiki_results = wiki.search(query, mode, max_results)
            if wiki_results:
                logger.info("wikipedia fallback succeeded (%d results)", len(wiki_results))
                backends_used.append(wiki.name)
                provenance_chain.append(wiki.provenance)
                merged = wiki_results
            else:
                logger.warning("wikipedia fallback returned no results")
                errors.append("wikipedia: returned no results")
        except Exception as exc:  # noqa: BLE001
            logger.warning("wikipedia fallback failed: %s", exc)
            errors.append(f"wikipedia: {exc}")

    return SearchResponse(
        query=query,
        mode=mode,
        results=merged,
        backends_used=backends_used or ["none"],
        provenance_chain=provenance_chain or ["No backend returned results"],
        sovereignty_step=_sovereignty_step(backends_used),
        errors=errors,
        search_strategy=strategy,
    )
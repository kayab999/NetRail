from __future__ import annotations

import logging
import threading
import time
from typing import Callable
from urllib.parse import urljoin

import httpx

from netrail.backends.types import OPERATORS, SearchMode, SearchResult
from netrail.security import check_backend_fetch_url

logger = logging.getLogger(__name__)

# Parity with Rust USER_AGENT (http_client.rs) and the 60 s SearXNG health
# TTL (backends/searxng.rs): availability probes are cached so every search
# does not pay an extra /healthz round-trip.
USER_AGENT = (
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
)
_HEALTH_TTL_SECONDS = 60.0
_HEALTH_CACHE: dict[str, tuple[bool, float]] = {}
_HEALTH_LOCK = threading.Lock()
_SHARED_HEALTH_CLIENT = httpx.Client(
    timeout=3.0,
    follow_redirects=False,
    headers={"User-Agent": USER_AGENT},
)


def _cache_key(base_url: str) -> str:
    return base_url.rstrip("/").lower()


def _cached_health(base_url: str) -> bool | None:
    with _HEALTH_LOCK:
        entry = _HEALTH_CACHE.get(_cache_key(base_url))
    if entry is None:
        return None
    ok, checked_at = entry
    if time.monotonic() - checked_at < _HEALTH_TTL_SECONDS:
        return ok
    return None


def _record_health(base_url: str, ok: bool) -> None:
    with _HEALTH_LOCK:
        _HEALTH_CACHE[_cache_key(base_url)] = (ok, time.monotonic())


def _clear_health_cache() -> None:
    """Test hook: drop cached availability (mirrors Rust test isolation)."""
    with _HEALTH_LOCK:
        _HEALTH_CACHE.clear()


class SearXNGBackend:
    """Self-hosted SearXNG JSON API. First truly user-controlled backend."""

    supports_operators = OPERATORS

    def __init__(
        self,
        base_url: str,
        timeout: float = 12.0,
        *,
        strict: bool = False,
        resolver: Callable[[str], list] | None = None,
    ) -> None:
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout
        self.strict = strict
        self._resolver = resolver

    @property
    def name(self) -> str:
        return "searxng"

    @property
    def provenance(self) -> str:
        return f"SearXNG @ {self.base_url} (your instance, your engines)"

    def _check_fetch_url(self) -> None:
        # Fetch-time SSRF guard (A-05): hostnames are resolved before every
        # request; cloud metadata / link-local / unspecified are always
        # rejected, other non-public ranges only when strict_backend_urls.
        check_backend_fetch_url(
            self.base_url,
            strict=self.strict,
            resolver=self._resolver,
        )

    def is_available(self) -> bool:
        if not self.base_url.startswith(("http://", "https://")):
            return False
        cached = _cached_health(self.base_url)
        if cached is not None:
            return cached
        try:
            self._check_fetch_url()
            response = _SHARED_HEALTH_CLIENT.get(f"{self.base_url}/healthz")
            ok = response.status_code < 500
        except Exception:
            ok = False
        _record_health(self.base_url, ok)
        return ok

    def search(self, query: str, mode: SearchMode, max_results: int) -> list[SearchResult]:
        self._check_fetch_url()
        category = "images" if mode == "images" else "general"
        endpoint = urljoin(self.base_url + "/", "search")
        params = {
            "q": query,
            "format": "json",
            "categories": category,
        }

        with httpx.Client(timeout=self.timeout, follow_redirects=False) as client:
            response = client.get(endpoint, params=params)
            response.raise_for_status()
            payload = response.json()

        results: list[SearchResult] = []
        for item in payload.get("results", [])[:max_results]:
            url = item.get("url", "")
            if not url:
                continue
            results.append(
                SearchResult(
                    title=item.get("title") or url,
                    url=url,
                    snippet=item.get("content", ""),
                    image=item.get("thumbnail") or item.get("img_src"),
                    source=item.get("engine", ""),
                    backend=self.name,
                    provenance=self.provenance,
                )
            )
        return results
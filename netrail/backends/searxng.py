from __future__ import annotations

import logging
import threading
import time
from typing import Any, Callable
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

_CONFIG_TTL_SECONDS = 300.0
_CONFIG_CACHE: dict[str, tuple[dict[str, Any], float]] = {}
_CONFIG_LOCK = threading.Lock()
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


def _clear_config_cache() -> None:
    """Test hook: drop cached config."""
    with _CONFIG_LOCK:
        _CONFIG_CACHE.clear()


def fetch_config(
    base_url: str,
    strict: bool = False,
    resolver: Callable[[str], list] | None = None,
    timeout: float = 5.0,
) -> dict[str, Any]:
    key = _cache_key(base_url)
    with _CONFIG_LOCK:
        entry = _CONFIG_CACHE.get(key)
    if entry is not None:
        result, checked_at = entry
        if time.monotonic() - checked_at < _CONFIG_TTL_SECONDS:
            return dict(result)

    if not base_url.startswith(("http://", "https://")):
        result = {"status": "invalid", "categories": [], "engines": [], "error": "invalid base url"}
        _record_config(base_url, result)
        return dict(result)

    # SSRF guard
    try:
        check_backend_fetch_url(base_url, strict=strict, resolver=resolver)
    except Exception as exc:  # noqa: BLE001
        result = {"status": "unavailable", "categories": [], "engines": [], "error": str(exc)}
        _record_config(base_url, result)
        return dict(result)

    endpoint = base_url.rstrip("/") + "/config"
    try:
        with httpx.Client(timeout=timeout, follow_redirects=False) as client:
            response = client.get(endpoint)
    except Exception as exc:  # noqa: BLE001
        result = {"status": "unavailable", "categories": [], "engines": [], "error": str(exc)}
        _record_config(base_url, result)
        return dict(result)

    if response.status_code >= 400:
        result = {
            "status": "unavailable",
            "categories": [],
            "engines": [],
            "error": f"HTTP {response.status_code}",
        }
        _record_config(base_url, result)
        return dict(result)

    try:
        payload = response.json()
    except Exception as exc:  # noqa: BLE001
        result = {"status": "invalid", "categories": [], "engines": [], "error": f"invalid JSON: {exc}"}
        _record_config(base_url, result)
        return dict(result)

    if not isinstance(payload, dict):
        result = {"status": "invalid", "categories": [], "engines": [], "error": "config is not an object"}
        _record_config(base_url, result)
        return dict(result)

    categories: list[str] = []
    cats = payload.get("categories")
    if isinstance(cats, list):
        for c in cats:
            if isinstance(c, str):
                categories.append(c)
    elif isinstance(cats, dict):
        for k in cats.keys():
            categories.append(str(k))

    engines: list[dict[str, Any]] = []
    engs = payload.get("engines")
    if isinstance(engs, list):
        for e in engs:
            if not isinstance(e, dict):
                continue
            name = e.get("name") or e.get("engine")
            if not isinstance(name, str) or not name:
                continue
            enabled = e.get("enabled", True)
            if not isinstance(enabled, bool):
                enabled = True
            ecats: list[str] = []
            raw_ecats = e.get("categories")
            if isinstance(raw_ecats, list):
                for c in raw_ecats:
                    if isinstance(c, str):
                        ecats.append(c)
            engines.append({"name": name, "enabled": bool(enabled), "categories": ecats})

    if not categories and engines:
        seen: set[str] = set()
        for e in engines:
            for c in e["categories"]:
                seen.add(c)
        categories = sorted(seen)

    result = {
        "status": "ok",
        "categories": categories,
        "engines": engines,
        "error": None,
    }
    _record_config(base_url, result)
    return dict(result)


def _record_config(base_url: str, result: dict[str, Any]) -> None:
    with _CONFIG_LOCK:
        _CONFIG_CACHE[_cache_key(base_url)] = (dict(result), time.monotonic())


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

    def search(
        self,
        query: str,
        mode: SearchMode,
        max_results: int,
        options: dict[str, Any] | None = None,
    ) -> list[SearchResult]:
        self._check_fetch_url()
        category = "images" if mode == "images" else "general"
        endpoint = urljoin(self.base_url + "/", "search")
        params: dict[str, Any] = {
            "q": query,
            "format": "json",
            "categories": category,
        }
        opts = options or {}
        if opts.get("language"):
            params["language"] = opts["language"]
        if opts.get("time_range"):
            params["time_range"] = opts["time_range"]
        if opts.get("pageno"):
            params["pageno"] = opts["pageno"]
        if opts.get("safesearch") is not None:
            params["safesearch"] = opts["safesearch"]

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
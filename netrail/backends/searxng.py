from __future__ import annotations

import logging
from typing import Callable
from urllib.parse import urljoin

import httpx

from netrail.backends.types import OPERATORS, SearchMode, SearchResult
from netrail.security import check_backend_fetch_url

logger = logging.getLogger(__name__)


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
        try:
            self._check_fetch_url()
            with httpx.Client(timeout=3.0, follow_redirects=False) as client:
                response = client.get(f"{self.base_url}/healthz")
                return response.status_code < 500
        except Exception:
            return False

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
from __future__ import annotations

import logging
from urllib.parse import urljoin

import httpx

from netrail.backends.types import OPERATORS, SearchMode, SearchResult

logger = logging.getLogger(__name__)


class SearXNGBackend:
    """Self-hosted SearXNG JSON API. First truly user-controlled backend."""

    supports_operators = OPERATORS

    def __init__(self, base_url: str, timeout: float = 12.0) -> None:
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout

    @property
    def name(self) -> str:
        return "searxng"

    @property
    def provenance(self) -> str:
        return f"SearXNG @ {self.base_url} (your instance, your engines)"

    def is_available(self) -> bool:
        if not self.base_url.startswith(("http://", "https://")):
            return False
        try:
            with httpx.Client(timeout=3.0) as client:
                response = client.get(f"{self.base_url}/healthz")
                return response.status_code < 500
        except httpx.HTTPError:
            return False

    def search(self, query: str, mode: SearchMode, max_results: int) -> list[SearchResult]:
        category = "images" if mode == "images" else "general"
        endpoint = urljoin(self.base_url + "/", "search")
        params = {
            "q": query,
            "format": "json",
            "categories": category,
        }

        with httpx.Client(timeout=self.timeout) as client:
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
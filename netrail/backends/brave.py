from __future__ import annotations

import logging
import os

import httpx

from netrail.backends.types import OPERATORS, SearchMode, SearchResult

logger = logging.getLogger(__name__)

PROVENANCE = "Brave Search API (your key, your quota)"


class BraveBackend:
    name = "brave"
    provenance = PROVENANCE
    supports_operators = OPERATORS

    def __init__(self, api_key: str, timeout: float = 12.0) -> None:
        self.api_key = api_key
        self.timeout = timeout

    @classmethod
    def from_env(cls) -> "BraveBackend | None":
        return cls.from_env_var(None)

    @classmethod
    def from_env_var(cls, env_name: str | None) -> "BraveBackend | None":
        primary = env_name or "BRAVE_SEARCH_API_KEY"
        candidates = [primary]
        if primary != "NETRAIL_BRAVE_API_KEY":
            candidates.append("NETRAIL_BRAVE_API_KEY")
        if primary != "BRAVE_SEARCH_API_KEY":
            candidates.append("BRAVE_SEARCH_API_KEY")
        key = next((os.environ.get(name) for name in candidates if os.environ.get(name)), None)
        if not key or not key.strip():
            return None
        return cls(key.strip())

    def is_available(self) -> bool:
        return bool(self.api_key)

    def search(self, query: str, mode: SearchMode, max_results: int) -> list[SearchResult]:
        if mode == "images":
            return self._search_images(query, max_results)
        return self._search_web(query, max_results)

    def _search_web(self, query: str, max_results: int) -> list[SearchResult]:
        headers = {
            "Accept": "application/json",
            "X-Subscription-Token": self.api_key,
        }
        params = {"q": query, "count": min(max_results, 20)}
        with httpx.Client(timeout=self.timeout) as client:
            response = client.get(
                "https://api.search.brave.com/res/v1/web/search",
                headers=headers,
                params=params,
            )
            response.raise_for_status()
            payload = response.json()

        results: list[SearchResult] = []
        for item in payload.get("web", {}).get("results", [])[:max_results]:
            url = item.get("url", "")
            if not url:
                continue
            results.append(
                SearchResult(
                    title=item.get("title") or url,
                    url=url,
                    snippet=item.get("description", ""),
                    backend=self.name,
                    provenance=self.provenance,
                )
            )
        return results

    def _search_images(self, query: str, max_results: int) -> list[SearchResult]:
        headers = {
            "Accept": "application/json",
            "X-Subscription-Token": self.api_key,
        }
        params = {"q": query, "count": min(max_results, 20)}
        with httpx.Client(timeout=self.timeout) as client:
            response = client.get(
                "https://api.search.brave.com/res/v1/images/search",
                headers=headers,
                params=params,
            )
            response.raise_for_status()
            payload = response.json()

        results: list[SearchResult] = []
        for item in payload.get("results", [])[:max_results]:
            url = item.get("url", "")
            if not url:
                continue
            thumb = item.get("thumbnail") or {}
            results.append(
                SearchResult(
                    title=item.get("title") or "Image result",
                    url=url,
                    snippet=item.get("source", ""),
                    image=thumb.get("src"),
                    source=item.get("source", ""),
                    backend=self.name,
                    provenance=self.provenance,
                )
            )
        return results
from __future__ import annotations

import logging

from ddgs import DDGS

from netrail.backends.types import OPERATORS, SearchMode, SearchResult

logger = logging.getLogger(__name__)

PROVENANCE = "ddgs → DuckDuckGo metasearch → primarily Bing index"


class DDGSBackend:
    name = "ddgs"
    provenance = PROVENANCE
    supports_operators = OPERATORS

    def is_available(self) -> bool:
        return True

    def search(self, query: str, mode: SearchMode, max_results: int) -> list[SearchResult]:
        with DDGS() as ddgs:
            if mode == "images":
                raw = ddgs.images(query, max_results=max_results)
                return [
                    SearchResult(
                        title=item.get("title") or "Image result",
                        url=item.get("url") or item.get("image", ""),
                        snippet=item.get("source", ""),
                        image=item.get("thumbnail") or item.get("image"),
                        source=item.get("source", ""),
                        backend=self.name,
                        provenance=self.provenance,
                    )
                    for item in raw
                    if item.get("url") or item.get("image")
                ]

            raw = ddgs.text(query, max_results=max_results)
            return [
                SearchResult(
                    title=item.get("title") or item.get("href", "Untitled"),
                    url=item.get("href", ""),
                    snippet=item.get("body", ""),
                    backend=self.name,
                    provenance=self.provenance,
                )
                for item in raw
                if item.get("href")
            ]
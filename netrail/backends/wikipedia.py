from __future__ import annotations

import logging
from typing import Any
from urllib.parse import quote

import httpx

from netrail.backends.types import OPERATORS, SearchMode, SearchResult

logger = logging.getLogger(__name__)

PROVENANCE = "Wikipedia OpenSearch + intro extracts (direct, no API key)"
_OPENSEARCH = (
    "https://en.wikipedia.org/w/api.php"
    "?action=opensearch&profile=fuzzy&search={query}&limit={limit}"
    "&namespace=0&format=json"
)
_EXTRACTS = (
    "https://en.wikipedia.org/w/api.php"
    "?action=query&format=json&prop=extracts&exintro=1&explaintext=1"
    "&exchars=400&redirects=1&titles={titles}"
)


def _normalize_extract(text: str) -> str:
    return " ".join(text.split())


class WikipediaBackend:
    name = "wikipedia"
    provenance = PROVENANCE
    supports_operators = OPERATORS

    def __init__(self, client: httpx.Client | None = None) -> None:
        self._client = client

    def is_available(self) -> bool:
        return True

    def search(self, query: str, mode: SearchMode, max_results: int) -> list[SearchResult]:
        if mode != "web":
            return []

        client = self._client or httpx.Client(timeout=15.0, follow_redirects=True)
        owns_client = self._client is None
        try:
            response = client.get(
                _OPENSEARCH.format(query=quote(query), limit=max_results)
            )
            response.raise_for_status()
            payload: Any = response.json()
            if not isinstance(payload, list) or len(payload) < 4:
                return []

            titles = payload[1] if isinstance(payload[1], list) else []
            descriptions = payload[2] if isinstance(payload[2], list) else []
            urls = payload[3] if isinstance(payload[3], list) else []

            rows: list[tuple[str, str, str]] = []
            pending: list[str] = []
            for index, title_val in enumerate(titles):
                if not isinstance(title_val, str) or not title_val:
                    continue
                url = urls[index] if index < len(urls) and isinstance(urls[index], str) else ""
                if not url:
                    continue
                snippet = ""
                if index < len(descriptions) and isinstance(descriptions[index], str):
                    snippet = descriptions[index].strip()
                if not snippet:
                    pending.append(title_val)
                rows.append((title_val, url, snippet))
                if len(rows) >= max_results:
                    break

            extracts = self._fetch_extracts(client, pending) if pending else {}
            results: list[SearchResult] = []
            for title, url, snippet in rows:
                if not snippet:
                    snippet = extracts.get(title.lower(), "")
                results.append(
                    SearchResult(
                        title=title,
                        url=url,
                        snippet=snippet,
                        source="wikipedia",
                        backend=self.name,
                        provenance=self.provenance,
                    )
                )
            return results
        finally:
            if owns_client:
                client.close()

    def _fetch_extracts(self, client: httpx.Client, titles: list[str]) -> dict[str, str]:
        if not titles:
            return {}
        titles_param = "|".join(quote(t, safe="") for t in titles)
        response = client.get(_EXTRACTS.format(titles=titles_param))
        response.raise_for_status()
        payload = response.json()
        pages = (
            payload.get("query", {}).get("pages", {})
            if isinstance(payload, dict)
            else {}
        )
        extracts: dict[str, str] = {}
        if not isinstance(pages, dict):
            return extracts
        for page in pages.values():
            if not isinstance(page, dict):
                continue
            title = page.get("title")
            extract = page.get("extract")
            if isinstance(title, str) and isinstance(extract, str):
                normalized = _normalize_extract(extract)
                if normalized:
                    extracts[title.lower()] = normalized
        return extracts

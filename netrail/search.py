from __future__ import annotations

from typing import Any, Literal

from ddgs import DDGS

SearchMode = Literal["web", "images"]


def search(
    query: str,
    mode: SearchMode = "web",
    max_results: int = 25,
) -> list[dict[str, Any]]:
    query = query.strip()
    if not query:
        return []

    max_results = max(1, min(max_results, 50))

    with DDGS() as ddgs:
        if mode == "images":
            raw = ddgs.images(query, max_results=max_results)
            return [
                {
                    "title": item.get("title") or "Image result",
                    "url": item.get("url") or item.get("image", ""),
                    "snippet": item.get("source", ""),
                    "image": item.get("thumbnail") or item.get("image"),
                    "source": item.get("source", ""),
                }
                for item in raw
                if item.get("url") or item.get("image")
            ]

        raw = ddgs.text(query, max_results=max_results)
        return [
            {
                "title": item.get("title") or item.get("href", "Untitled"),
                "url": item.get("href", ""),
                "snippet": item.get("body", ""),
                "image": None,
                "source": "",
            }
            for item in raw
            if item.get("href")
        ]
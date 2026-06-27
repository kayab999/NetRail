from __future__ import annotations

from typing import Any

from netrail.backends.registry import search_with_fallback
from netrail.backends.types import SearchMode


def search(
    query: str,
    mode: SearchMode = "web",
    max_results: int = 25,
) -> dict[str, Any]:
    response = search_with_fallback(query=query, mode=mode, max_results=max_results)
    if not response.results and response.errors:
        raise RuntimeError("; ".join(response.errors))
    return response.as_dict()
from __future__ import annotations

from typing import Any

from netrail.backends.registry import search_with_fallback
from netrail.backends.types import SearchMode
from netrail.config import load_settings
from netrail.history.store import get_store


def _sovereignty_with_history(step: int) -> int:
    settings = load_settings()
    if settings.get("history_enabled", True):
        store = get_store()
        if store and store.stats()["queries"] > 0:
            return max(step, 4)
    return step


def search(
    query: str,
    mode: SearchMode = "web",
    max_results: int = 25,
) -> dict[str, Any]:
    response = search_with_fallback(query=query, mode=mode, max_results=max_results)
    if not response.results and response.errors:
        raise RuntimeError("; ".join(response.errors))

    payload = response.as_dict()
    payload["sovereignty"]["step"] = _sovereignty_with_history(payload["sovereignty"]["step"])
    payload["sovereignty"]["label"] = _label_for_step(payload["sovereignty"]["step"])

    store = get_store()
    if store and response.results:
        query_id, url_to_result_id = store.record_search(
            query=response.query,
            mode=response.mode,
            backends_used=response.backends_used,
            results=response.results,
        )
        visit_meta = store.get_visit_metadata([r.url for r in response.results])

        enriched = []
        for item in payload["results"]:
            url = item["url"]
            entry = dict(item)
            entry["result_id"] = url_to_result_id.get(url)
            meta = visit_meta.get(url)
            entry["visit_metadata"] = meta if meta else None
            enriched.append(entry)
        payload["results"] = enriched
        payload["query_id"] = query_id

    elif store:
        visit_meta = store.get_visit_metadata([r["url"] for r in payload["results"]])
        for item in payload["results"]:
            item["visit_metadata"] = visit_meta.get(item["url"])

    return payload


def _label_for_step(step: int) -> str:
    labels = {
        1: "Local console — borrowed indexes",
        2: "Pluggable backends enabled",
        3: "Self-hosted discovery (SearXNG)",
        4: "Local history and corpus",
        5: "Owned index — full sovereignty",
    }
    return labels.get(step, labels[1])
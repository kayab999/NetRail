"""Phase 3b: merge follows configured backend order, not response speed.

`merge_fanout` interleaves in input-batch order; the fanout callers
(`registry.search_with_fallback` / Rust `search_with_fanout`) must therefore
hand it batches sorted by configuration, even when a later-configured
backend responds first. Empty backends are dropped before interleave and
consume no slot.
"""

from __future__ import annotations

import time

from netrail.backends import registry
from netrail.backends.merge import merge_fanout
from netrail.backends.types import SearchResult
from netrail.backends.wikipedia import WikipediaBackend


def _result(url: str, backend: str) -> SearchResult:
    return SearchResult(
        title=f"Result from {backend}",
        url=url,
        snippet="Test snippet",
        backend=backend,
        provenance=backend,
    )


class _SlowBackend:
    name = "slow"
    provenance = "slow → test"

    def is_available(self) -> bool:
        return True

    def search(self, query: str, mode, max_results: int) -> list[SearchResult]:
        time.sleep(0.4)
        return [_result("https://slow.example/1", self.name)]


class _FastBackend:
    name = "fast"
    provenance = "fast → test"

    def is_available(self) -> bool:
        return True

    def search(self, query: str, mode, max_results: int) -> list[SearchResult]:
        return [_result("https://fast.example/1", self.name)]


class _EmptyBackend:
    name = "empty"
    provenance = "empty → test"

    def is_available(self) -> bool:
        return True

    def search(self, query: str, mode, max_results: int) -> list[SearchResult]:
        return []


def _stub_wiki_empty(monkeypatch) -> None:
    monkeypatch.setattr(WikipediaBackend, "search", lambda self, *a, **k: [])


def test_fanout_follows_configured_order_not_speed(monkeypatch):
    """Configured [slow, fast]: slow responds last but merges first."""
    _stub_wiki_empty(monkeypatch)
    # Configured order puts the 0.4s backend first; completion order is reverse.
    monkeypatch.setattr(
        registry, "get_enabled_backends", lambda settings=None: [_SlowBackend(), _FastBackend()]
    )
    response = registry.search_with_fallback("q", settings={})
    assert response.backends_used == ["slow", "fast"]
    assert [r.backend for r in response.results] == ["slow", "fast"]


def test_empty_backend_consumes_no_interleave_slot(monkeypatch):
    """Configured [empty, fast]: empty drops out, fast fills position zero."""
    _stub_wiki_empty(monkeypatch)
    monkeypatch.setattr(
        registry, "get_enabled_backends", lambda settings=None: [_EmptyBackend(), _FastBackend()]
    )
    response = registry.search_with_fallback("q", settings={})
    assert response.backends_used == ["fast"]
    assert [r.url for r in response.results] == ["https://fast.example/1"]
    assert any("empty: returned no results" in e for e in response.errors)


def test_merge_fanout_round_robins_in_input_order():
    """`merge_fanout` output follows input batch order (caller contract)."""
    batches = [
        ("ddgs", [_result(f"https://ddgs.com/{i}", "ddgs") for i in range(3)]),
        ("searxng", [_result(f"https://searxng.com/{i}", "searxng") for i in range(3)]),
        ("brave", [_result(f"https://brave.com/{i}", "brave") for i in range(3)]),
    ]
    merged = merge_fanout(batches, 9)
    assert [r.backend for r in merged[:3]] == ["ddgs", "searxng", "brave"]
    # Reversed input → reversed output: order comes from the caller, not names.
    merged_rev = merge_fanout(list(reversed(batches)), 9)
    assert [r.backend for r in merged_rev[:3]] == ["brave", "searxng", "ddgs"]

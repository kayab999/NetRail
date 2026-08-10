from __future__ import annotations

import time

import pytest

from netrail.backends import registry
from netrail.backends.types import SearchResult
from netrail.errors import NetRailError
from netrail.search import search


class _FastBackend:
    name = "fast"
    provenance = "fast → test"

    def is_available(self) -> bool:
        return True

    def search(self, query: str, mode, max_results: int) -> list[SearchResult]:
        return [
            SearchResult(
                title="fast result",
                url="https://fast.example/1",
                snippet="fast",
                backend=self.name,
                provenance=self.provenance,
            )
        ]


class _SlowBackend:
    name = "slow"
    provenance = "slow → test"

    def __init__(self, hang: float = 3.0):
        self.hang = hang

    def is_available(self) -> bool:
        return True

    def search(self, query: str, mode, max_results: int) -> list[SearchResult]:
        time.sleep(self.hang)
        return []


def _deadline_short(monkeypatch) -> None:
    monkeypatch.setattr(registry, "FANOUT_DEADLINE_SECONDS", 0.5)


def test_partial_results_kept_200_with_timed_out_error(monkeypatch):
    _deadline_short(monkeypatch)
    monkeypatch.setattr(
        registry,
        "get_enabled_backends",
        lambda settings=None: [_FastBackend(), _SlowBackend(hang=4.0)],
    )

    start = time.monotonic()
    response = registry.search_with_fallback("q", settings={})
    elapsed = time.monotonic() - start

    assert response.results
    assert response.results[0].url == "https://fast.example/1"
    assert "fanout: timed out after 20 seconds" in response.errors
    assert elapsed < 2.5, (
        f"wall time {elapsed:.1f}s exceeded deadline+grace — executor blocked on a hung thread"
    )


def test_total_failure_raises_502_fanout_total_failure(monkeypatch):
    _deadline_short(monkeypatch)
    monkeypatch.setattr(
        registry,
        "get_enabled_backends",
        lambda settings=None: [_SlowBackend(hang=4.0), _SlowBackend(hang=4.0)],
    )

    with pytest.raises(NetRailError) as exc_info:
        search("q", settings={"history_enabled": False})
    assert exc_info.value.code == "FANOUT_TOTAL_FAILURE"
    assert exc_info.value.status == 502


def test_hung_backend_does_not_stretch_request_past_deadline(monkeypatch):
    _deadline_short(monkeypatch)
    monkeypatch.setattr(
        registry,
        "get_enabled_backends",
        lambda settings=None: [_SlowBackend(hang=6.0)],
    )

    start = time.monotonic()
    response = registry.search_with_fallback("q", settings={})
    elapsed = time.monotonic() - start

    assert not response.results
    assert "fanout: timed out after 20 seconds" in response.errors
    assert elapsed < 2.0, (
        f"wall time {elapsed:.1f}s — deadline is not enforced, shutdown must not block"
    )


def test_fast_path_without_deadline_still_joins_executor(monkeypatch):
    monkeypatch.setattr(
        registry,
        "get_enabled_backends",
        lambda settings=None: [_FastBackend()],
    )

    response = registry.search_with_fallback("q", settings={})

    assert response.results
    assert response.errors == []
    assert response.backends_used == ["fast"]
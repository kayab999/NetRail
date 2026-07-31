"""Lightweight localhost rate limits for search/open. Disable with NETRAIL_RATE_LIMIT=0."""

from __future__ import annotations

import os
import threading
import time

from netrail.errors import NetRailError

_SEARCH_PER_MIN = 90
_OPEN_PER_MIN = 120
_MUTATE_PER_MIN = 60
_WINDOW = 60.0

_lock = threading.Lock()
_search_start = time.monotonic()
_search_count = 0
_open_start = time.monotonic()
_open_count = 0
_mutate_start = time.monotonic()
_mutate_count = 0

# Test seam: when set, overrides default caps (0 = unlimited for that counter).
_test_limits: dict[str, int] | None = None


def _enabled() -> bool:
    raw = os.environ.get("NETRAIL_RATE_LIMIT", "1")
    return raw not in {"0", "false", "False", "FALSE"}


def set_test_limits(
    *,
    search: int | None = None,
    open_limit: int | None = None,
    mutate: int | None = None,
) -> None:
    """Test-only: set per-minute caps. Pass None to clear overrides."""
    global _test_limits, _search_count, _open_count, _mutate_count
    global _search_start, _open_start, _mutate_start
    if search is None and open_limit is None and mutate is None:
        _test_limits = None
    else:
        _test_limits = {
            "search": search if search is not None else _SEARCH_PER_MIN,
            "open": open_limit if open_limit is not None else _OPEN_PER_MIN,
            "mutate": mutate if mutate is not None else _MUTATE_PER_MIN,
        }
    now = time.monotonic()
    _search_start = _open_start = _mutate_start = now
    _search_count = _open_count = _mutate_count = 0


def _limit_for(kind: str) -> int:
    if _test_limits is not None:
        return int(_test_limits[kind])
    if not _enabled():
        return 0
    if kind == "search":
        return _SEARCH_PER_MIN
    if kind == "open":
        return _OPEN_PER_MIN
    return _MUTATE_PER_MIN


def _try_acquire(kind: str) -> None:
    global _search_start, _search_count, _open_start, _open_count
    global _mutate_start, _mutate_count
    limit = _limit_for(kind)
    if limit == 0:
        return
    now = time.monotonic()
    with _lock:
        if kind == "search":
            if now - _search_start >= _WINDOW:
                _search_start = now
                _search_count = 0
            if _search_count >= limit:
                raise NetRailError(
                    "RATE_LIMITED",
                    f"Too many searches (max {_SEARCH_PER_MIN}/minute). Wait a moment.",
                    status=429,
                )
            _search_count += 1
        elif kind == "open":
            if now - _open_start >= _WINDOW:
                _open_start = now
                _open_count = 0
            if _open_count >= limit:
                raise NetRailError(
                    "RATE_LIMITED",
                    f"Too many open requests (max {_OPEN_PER_MIN}/minute). Wait a moment.",
                    status=429,
                )
            _open_count += 1
        else:
            if now - _mutate_start >= _WINDOW:
                _mutate_start = now
                _mutate_count = 0
            if _mutate_count >= limit:
                raise NetRailError(
                    "RATE_LIMITED",
                    f"Too many configuration/history mutations (max {_MUTATE_PER_MIN}/minute).",
                    status=429,
                )
            _mutate_count += 1


def check_search() -> None:
    _try_acquire("search")


def check_open() -> None:
    _try_acquire("open")


def check_mutate() -> None:
    _try_acquire("mutate")


def status_dict() -> dict:
    return {
        "enabled": _enabled(),
        "search_per_minute": _SEARCH_PER_MIN,
        "open_per_minute": _OPEN_PER_MIN,
        "mutate_per_minute": _MUTATE_PER_MIN,
    }

"""Lightweight localhost rate limits for search/open. Disable with NETRAIL_RATE_LIMIT=0.

Buckets are keyed by client identity: when token auth is on, each token gets
its own per-minute budget (auth.client_identity); otherwise everything shares
one process-wide budget. Limits are per-process — two API processes (desktop
+ Docker) do not share counters (A9).
"""

from __future__ import annotations

import os
import threading
import time

from netrail.errors import NetRailError

_SEARCH_PER_MIN = 90
_OPEN_PER_MIN = 120
_MUTATE_PER_MIN = 60
_WINDOW = 60.0
_MAX_IDENTITIES = 1024

_lock = threading.Lock()
# kind -> identity -> (window_start, count)
_buckets: dict[str, dict[str, tuple[float, int]]] = {"search": {}, "open": {}, "mutate": {}}

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
    global _test_limits
    if search is None and open_limit is None and mutate is None:
        _test_limits = None
    else:
        _test_limits = {
            "search": search if search is not None else _SEARCH_PER_MIN,
            "open": open_limit if open_limit is not None else _OPEN_PER_MIN,
            "mutate": mutate if mutate is not None else _MUTATE_PER_MIN,
        }
    _reset_buckets()


def _reset_buckets() -> None:
    global _buckets
    with _lock:
        _buckets = {"search": {}, "open": {}, "mutate": {}}


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


_MESSAGES = {
    "search": f"Too many searches (max {_SEARCH_PER_MIN}/minute). Wait a moment.",
    "open": f"Too many open requests (max {_OPEN_PER_MIN}/minute). Wait a moment.",
    "mutate": (
        f"Too many configuration/history mutations (max {_MUTATE_PER_MIN}/minute)."
    ),
}


def _try_acquire(kind: str, identity: str = "anonymous") -> None:
    limit = _limit_for(kind)
    if limit == 0:
        return
    now = time.monotonic()
    with _lock:
        identities = _buckets[kind]
        start, count = identities.get(identity, (now, 0))
        if now - start >= _WINDOW:
            start, count = now, 0
        if count >= limit:
            raise NetRailError("RATE_LIMITED", _MESSAGES[kind], status=429)
        identities[identity] = (start, count + 1)
        if len(identities) > _MAX_IDENTITIES:
            stale = [k for k, (s, _) in identities.items() if now - s >= _WINDOW * 2]
            for key in stale:
                del identities[key]


def check_search(identity: str = "anonymous") -> None:
    _try_acquire("search", identity)


def check_open(identity: str = "anonymous") -> None:
    _try_acquire("open", identity)


def check_mutate(identity: str = "anonymous") -> None:
    _try_acquire("mutate", identity)


def status_dict() -> dict:
    from netrail.auth import token_required

    return {
        "enabled": _enabled(),
        "mode": "per-token" if token_required() else "process",
        "search_per_minute": _SEARCH_PER_MIN,
        "open_per_minute": _OPEN_PER_MIN,
        "mutate_per_minute": _MUTATE_PER_MIN,
    }

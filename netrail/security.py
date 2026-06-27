from __future__ import annotations

from urllib.parse import urlparse

_BLOCKED_SCHEMES = frozenset({"javascript", "data", "file", "vbscript"})


def validate_open_url(url: str) -> str:
    """Reject dangerous URL forms before spawning a browser."""
    parsed = urlparse(url.strip())

    if parsed.scheme not in {"http", "https"}:
        raise ValueError("Only http:// and https:// URLs are supported.")

    if parsed.scheme in _BLOCKED_SCHEMES:
        raise ValueError(f"Blocked URL scheme: {parsed.scheme}")

    if parsed.username or parsed.password:
        raise ValueError("URLs with embedded credentials are not allowed.")

    host = (parsed.hostname or "").lower()
    if not host:
        raise ValueError("URL must include a host.")

    if host in {"127.0.0.1", "localhost", "::1"}:
        raise ValueError("Localhost URLs cannot be opened from search results.")

    return url.strip()
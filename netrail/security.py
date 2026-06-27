from __future__ import annotations

from urllib.parse import urlparse, unquote

_BLOCKED_SCHEMES = frozenset({"javascript", "data", "file", "vbscript"})
_DDG_HOSTS = frozenset({"duckduckgo.com", "r.duckduckgo.com", "www.duckduckgo.com"})
_MAX_REDIRECT_DEPTH = 5


def _is_ddg_host(host: str) -> bool:
    host = host.lower()
    return host in _DDG_HOSTS or any(host.endswith(f".{h}") for h in _DDG_HOSTS)


def _block_unsafe_host(host: str) -> None:
    host_lower = host.lower()
    if host_lower in {"127.0.0.1", "localhost", "::1", "0.0.0.0", "[::1]"}:
        raise ValueError("Localhost URLs cannot be opened from search results.")

    if (
        host_lower.endswith(".nip.io")
        or host_lower.endswith(".sslip.io")
        or host_lower.endswith(".xip.io")
    ):
        raise ValueError("DNS rebinding hostnames cannot be opened from search results.")

    import ipaddress

    try:
        ip = ipaddress.ip_address(host_lower.strip("[]"))
    except ValueError:
        return

    if ip.is_loopback or ip.is_unspecified or ip.is_link_local:
        raise ValueError(
            "Local or link-local IP addresses cannot be opened from search results."
        )


def _validate_open_url_inner(url: str, depth: int) -> str:
    if depth > _MAX_REDIRECT_DEPTH:
        raise ValueError("Too many redirect wrappers.")

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

    if _is_ddg_host(host):
        from urllib.parse import parse_qs

        params = parse_qs(parsed.query)
        uddg_vals = params.get("uddg")
        if uddg_vals:
            inner = unquote(uddg_vals[0])
            return _validate_open_url_inner(inner, depth + 1)

    _block_unsafe_host(host)
    return url.strip()


def validate_open_url(url: str) -> str:
    """Reject dangerous URL forms before spawning a browser."""
    return _validate_open_url_inner(url, 0)


def _block_backend_host(host: str) -> None:
    host_lower = host.lower()
    if (
        host_lower.endswith(".nip.io")
        or host_lower.endswith(".sslip.io")
        or host_lower.endswith(".xip.io")
    ):
        raise ValueError("DNS rebinding hostnames are not allowed in backend URLs.")

    import ipaddress

    try:
        ip = ipaddress.ip_address(host_lower.strip("[]"))
    except ValueError:
        return

    if ip == ipaddress.ip_address("169.254.169.254"):
        raise ValueError("Cloud metadata addresses cannot be used as backend URLs.")
    if ip == ipaddress.ip_address("fd00:ec2::254"):
        raise ValueError("Cloud metadata addresses cannot be used as backend URLs.")
    if ip.is_unspecified or ip.is_link_local:
        raise ValueError(
            "Unspecified or link-local addresses cannot be used as backend URLs."
        )


def validate_backend_url(url: str) -> str:
    """Validate a user-configured backend URL (e.g. SearXNG)."""
    trimmed = url.strip()
    if not trimmed:
        raise ValueError("Backend URL cannot be empty.")

    parsed = urlparse(trimmed)
    if parsed.scheme not in {"http", "https"}:
        raise ValueError("Backend URL must use http:// or https://.")
    if parsed.username or parsed.password:
        raise ValueError("Backend URLs with embedded credentials are not allowed.")

    host = (parsed.hostname or "").lower()
    if not host:
        raise ValueError("Backend URL must include a host.")

    _block_backend_host(host)
    return trimmed
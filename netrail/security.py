from __future__ import annotations

import ipaddress
import re
from urllib.parse import parse_qs, unquote, urlparse

from netrail.errors import NetRailError

_BLOCKED_SCHEMES = frozenset({"javascript", "data", "file", "vbscript"})
# Base registrable hosts; subdomains (www., r., …) match via suffix.
_DDG_HOSTS = frozenset({"duckduckgo.com", "duck.com"})
_DNS_REBINDING_HELPERS = frozenset({"nip.io", "sslip.io", "xip.io", "localtest.me"})
_CLOUD_METADATA_HOSTS = frozenset(
    {
        "metadata.google.internal",
        "metadata",
        "instance-data",
    }
)
_MAX_REDIRECT_DEPTH = 5
_HEX_INT = re.compile(r"^0[xX][0-9a-fA-F]+$")
_OCTAL_INT = re.compile(r"^0[0-7]+$")


def _is_ddg_host(host: str) -> bool:
    host = host.lower()
    return host in _DDG_HOSTS or any(host.endswith(f".{h}") for h in _DDG_HOSTS)


def _is_dns_rebinding_helper(host: str) -> bool:
    host = host.lower()
    return host in _DNS_REBINDING_HELPERS or any(
        host.endswith(f".{d}") for d in _DNS_REBINDING_HELPERS
    )


def _parse_u32_loose(raw: str) -> int | None:
    s = raw.strip()
    if not s:
        return None
    try:
        if _HEX_INT.match(s):
            return int(s, 16)
        if len(s) > 1 and _OCTAL_INT.match(s):
            return int(s, 8)
        if s.isdigit():
            return int(s, 10)
    except ValueError:
        return None
    return None


def _parse_browser_ipv4(host: str) -> ipaddress.IPv4Address | None:
    """Browser-style IPv4: decimal, hex, octal octets, short forms (127.1 → 127.0.0.1)."""
    host = host.strip().strip("[]")
    if not host:
        return None

    try:
        return ipaddress.IPv4Address(host)
    except ValueError:
        pass

    if "." not in host:
        n = _parse_u32_loose(host)
        if n is None or n > 0xFFFFFFFF:
            return None
        return ipaddress.IPv4Address(n)

    parts = host.split(".")
    try:
        if len(parts) == 2:
            a = _parse_u32_loose(parts[0])
            b = _parse_u32_loose(parts[1])
            if a is None or b is None or a > 255 or b > 0x00FFFFFF:
                return None
            return ipaddress.IPv4Address((a << 24) | b)
        if len(parts) == 3:
            a = _parse_u32_loose(parts[0])
            b = _parse_u32_loose(parts[1])
            c = _parse_u32_loose(parts[2])
            if a is None or b is None or c is None or a > 255 or b > 255 or c > 0xFFFF:
                return None
            return ipaddress.IPv4Address((a << 24) | (b << 16) | c)
        if len(parts) == 4:
            octets: list[int] = []
            for part in parts:
                n = _parse_u32_loose(part)
                if n is None or n > 255:
                    return None
                octets.append(n)
            return ipaddress.IPv4Address(bytes(octets))
    except (ValueError, OverflowError):
        return None
    return None


def _parse_host_ip(host: str) -> ipaddress.IPv4Address | ipaddress.IPv6Address | None:
    host = host.strip().strip("[]")
    try:
        ip = ipaddress.ip_address(host)
    except ValueError:
        return _parse_browser_ipv4(host)
    return _effective_ip(ip)


def _effective_ip(
    ip: ipaddress.IPv4Address | ipaddress.IPv6Address,
) -> ipaddress.IPv4Address | ipaddress.IPv6Address:
    """Unmap IPv4-mapped IPv6 so loopback/private checks apply."""
    if isinstance(ip, ipaddress.IPv6Address) and ip.ipv4_mapped is not None:
        return ip.ipv4_mapped
    return ip


def _is_cloud_metadata_host(host: str) -> bool:
    host = host.lower()
    return host in _CLOUD_METADATA_HOSTS or any(
        host.endswith(f".{h}") for h in _CLOUD_METADATA_HOSTS
    )


def _is_cloud_metadata_ip(ip: ipaddress.IPv4Address | ipaddress.IPv6Address) -> bool:
    ip = _effective_ip(ip)
    if ip == ipaddress.ip_address("169.254.169.254"):
        return True
    if ip == ipaddress.ip_address("fd00:ec2::254"):
        return True
    return False


def _is_non_public_v4(ip: ipaddress.IPv4Address) -> bool:
    return bool(
        ip.is_loopback
        or ip.is_unspecified
        or ip.is_link_local
        or ip.is_private
        or ip.is_multicast
        or ip.is_reserved
        or int(ip) >= 0xF0000000  # 240.0.0.0/4 class E
    )


def _is_non_public_v6(ip: ipaddress.IPv6Address) -> bool:
    return bool(
        ip.is_loopback
        or ip.is_unspecified
        or ip.is_link_local
        or ip.is_private  # ULA
        or ip.is_multicast
        or ip.is_reserved
    )


def _block_unsafe_host(host: str) -> None:
    host_lower = host.lower()
    if host_lower in {"127.0.0.1", "localhost", "::1", "0.0.0.0", "[::1]"}:
        raise NetRailError(
            "OPEN_URL_LOCALHOST",
            "Localhost URLs cannot be opened from search results.",
        )

    if _is_dns_rebinding_helper(host_lower):
        raise NetRailError(
            "OPEN_URL_DNS_REBINDING",
            "DNS rebinding hostnames cannot be opened from search results.",
        )

    if _is_cloud_metadata_host(host_lower):
        raise NetRailError(
            "OPEN_URL_CLOUD_METADATA",
            "Cloud metadata hostnames cannot be opened from search results.",
        )

    ip = _parse_host_ip(host_lower)
    if ip is None:
        return

    if isinstance(ip, ipaddress.IPv4Address):
        if ip.is_loopback or ip.is_unspecified:
            raise NetRailError(
                "OPEN_URL_LOCALHOST",
                "Localhost URLs cannot be opened from search results.",
            )
        if ip.is_link_local:
            raise NetRailError(
                "OPEN_URL_LINK_LOCAL",
                "Local or link-local IP addresses cannot be opened from search results.",
            )
        if _is_non_public_v4(ip):
            raise NetRailError(
                "OPEN_URL_PRIVATE",
                "Private or non-public IP addresses cannot be opened from search results.",
            )
        return

    # IPv6 (already unmapped when possible)
    if ip.is_loopback or ip.is_unspecified:
        raise NetRailError(
            "OPEN_URL_LOCALHOST",
            "Localhost URLs cannot be opened from search results.",
        )
    if ip.is_link_local:
        raise NetRailError(
            "OPEN_URL_LINK_LOCAL",
            "Local or link-local IP addresses cannot be opened from search results.",
        )
    if _is_non_public_v6(ip):
        raise NetRailError(
            "OPEN_URL_PRIVATE",
            "Private or non-public IP addresses cannot be opened from search results.",
        )


def _validate_open_url_inner(url: str, depth: int) -> str:
    if depth > _MAX_REDIRECT_DEPTH:
        raise NetRailError("OPEN_URL_REDIRECT_DEPTH", "Too many redirect wrappers.")

    parsed = urlparse(url.strip())

    if parsed.scheme not in {"http", "https"}:
        if parsed.scheme in _BLOCKED_SCHEMES:
            raise NetRailError(
                "OPEN_URL_INVALID_SCHEME",
                f"Blocked URL scheme: {parsed.scheme}",
            )
        raise NetRailError(
            "OPEN_URL_INVALID",
            "Only http:// and https:// URLs are supported.",
        )

    if parsed.username or parsed.password:
        raise NetRailError(
            "OPEN_URL_CREDENTIALS",
            "URLs with embedded credentials are not allowed.",
        )

    host = (parsed.hostname or "").lower()
    if not host:
        raise NetRailError("OPEN_URL_NO_HOST", "URL must include a host.")

    if _is_ddg_host(host):
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
    if _is_dns_rebinding_helper(host_lower):
        raise NetRailError(
            "BACKEND_URL_DNS_REBINDING",
            "DNS rebinding hostnames are not allowed in backend URLs.",
        )

    if _is_cloud_metadata_host(host_lower):
        raise NetRailError(
            "BACKEND_URL_CLOUD_METADATA",
            "Cloud metadata addresses cannot be used as backend URLs.",
        )

    ip = _parse_host_ip(host_lower)
    if ip is None:
        return

    if _is_cloud_metadata_ip(ip):
        raise NetRailError(
            "BACKEND_URL_CLOUD_METADATA",
            "Cloud metadata addresses cannot be used as backend URLs.",
        )
    if ip.is_unspecified or ip.is_link_local:
        raise NetRailError(
            "BACKEND_URL_LINK_LOCAL",
            "Unspecified or link-local addresses cannot be used as backend URLs.",
        )


def validate_backend_url(url: str) -> str:
    """Validate a user-configured backend URL (e.g. SearXNG)."""
    trimmed = url.strip()
    if not trimmed:
        raise NetRailError("BACKEND_URL_EMPTY", "Backend URL cannot be empty.")

    parsed = urlparse(trimmed)
    if parsed.scheme not in {"http", "https"}:
        raise NetRailError(
            "BACKEND_URL_INVALID_SCHEME",
            "Backend URL must use http:// or https://.",
        )
    if parsed.username or parsed.password:
        raise NetRailError(
            "BACKEND_URL_CREDENTIALS",
            "Backend URLs with embedded credentials are not allowed.",
        )

    host = (parsed.hostname or "").lower()
    if not host:
        raise NetRailError("BACKEND_URL_NO_HOST", "Backend URL must include a host.")

    _block_backend_host(host)
    return trimmed

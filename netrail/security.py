from __future__ import annotations

import ipaddress
import re
import socket
from typing import Callable
from urllib.parse import parse_qs, unquote, urlparse

from netrail.errors import NetRailError

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
_HEX_INT_OR_EMPTY = re.compile(r"^0[xX][0-9a-fA-F]*$")
_OCTAL_INT = re.compile(r"^0[0-7]+$")
# Sentinel: host looks numeric but fails WHATWG IPv4 parsing (Rust `url` parity).
_NUM_INVALID = object()


def _is_ddg_host(host: str) -> bool:
    host = host.lower()
    return host in _DDG_HOSTS or any(host.endswith(f".{h}") for h in _DDG_HOSTS)


def _normalize_host(host: str) -> str:
    """WHATWG-style host normalization: percent-decode, lowercase, strip
    FQDN-root trailing dots (browsers strip the final '.' at DNS resolution,
    so `127.0.0.1.` and `duckduckgo.com.` must be treated as their base host).
    Non-ASCII hosts are IDNA-encoded to punycode (Rust `url` parity); hosts
    that are not valid IDN are returned unchanged and rejected by callers.
    """
    host = unquote(host).lower().rstrip(".")
    if any(ord(c) > 0x7F for c in host):
        try:
            return host.encode("idna").decode("ascii")
        except UnicodeError:
            return host
    return host


def _is_ascii(host: str) -> bool:
    return all(ord(c) <= 0x7F for c in host)


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
        if _HEX_INT_OR_EMPTY.match(s):
            return int(s[2:] or "0", 16)
        if len(s) > 1 and _OCTAL_INT.match(s):
            return int(s, 8)
        if s.isdigit():
            return int(s, 10)
    except ValueError:
        return None
    return None


def _parse_whatwg_num_part(raw: str) -> int | None:
    """WHATWG IPv4 number parser (URL Standard § parse IPv4 number):
    ``0x``-prefixed parts are hex, leading-zero parts are octal (digits 8/9
    make the whole address invalid), everything else decimal."""
    if _HEX_INT_OR_EMPTY.match(raw):
        return int(raw[2:] or "0", 16)
    if len(raw) > 1 and raw.startswith("0"):
        if any(c in "89" for c in raw):
            return None
        try:
            v = int(raw, 8)
        except ValueError:
            return None
        return v if v <= 0xFFFFFFFF else None
    if raw.isdigit():
        v = int(raw, 10)
        return v if v <= 0xFFFFFFFF else None
    return None


def _parse_whatwg_ipv4(host: str) -> ipaddress.IPv4Address | object | None:
    """Mirror the Rust `url` crate's WHATWG IPv4 handling (empirically probed:

    - a single label parses as IPv4 when it is all digits or ``0x``-hex
      (``0x`` alone is 0; ``0xzz``/``abc`` fall back to DNS domains);
    - dotted hosts with an all-digit or ``0x``-hex *last* label attempt the
      strict IPv4 parse and hard-fail the URL when any part is malformed
      (octal 8/9, mid-label ``x``, part > 255, > 4 parts, empty part).

    Returns the parsed address, ``_NUM_INVALID`` for numeric hosts that fail
    WHATWG rules, or ``None`` when the host is a DNS domain.
    """
    if ":" in host:
        # IPv6 literal (urlparse strips brackets) — handled by _parse_host_ip.
        return None
    if "." not in host:
        if not (host.isdigit() or _HEX_INT_OR_EMPTY.match(host)):
            return None
        v = _parse_whatwg_num_part(host)
        if v is None:
            return _NUM_INVALID
        try:
            return ipaddress.IPv4Address(v)
        except ValueError:
            return _NUM_INVALID

    last = host.rsplit(".", 1)[-1]
    if not (last.isdigit() or _HEX_INT_OR_EMPTY.match(last)):
        return None
    parts = host.split(".")
    if len(parts) > 4 or any(not p for p in parts):
        return _NUM_INVALID
    nums: list[int] = []
    for p in parts:
        v = _parse_whatwg_num_part(p)
        if v is None:
            return _NUM_INVALID
        nums.append(v)
    if any(n > 255 for n in nums[:-1]):
        return _NUM_INVALID
    if nums[-1] >= 256 ** (5 - len(nums)):
        return _NUM_INVALID
    total = nums[-1]
    for i, n in enumerate(nums[:-1]):
        total += n << (8 * (3 - i))
    try:
        return ipaddress.IPv4Address(total)
    except ValueError:
        return _NUM_INVALID


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


# Top-96-bits of the IPv6 ranges that embed a real IPv4 in the last 32 bits:
# RFC 6052 NAT64 well-known prefix (64:ff9b::/96), the IPv4-compatible
# mapped range (::ffff:0:0:0/96, i.e. ::ffff:0:a.b.c.d) and the deprecated
# IPv4-compatible range (::/96, i.e. ::a.b.c.d). (The standard IPv4-mapped
# ::ffff:a.b.c.d is handled via ip.ipv4_mapped.)
_NAT64_WKP_96 = int(ipaddress.IPv6Address("64:ff9b::")) >> 32
_COMPAT_MAPPED_96 = int(ipaddress.IPv6Address("::ffff:0:0:0")) >> 32
_IPV4_COMPATIBLE_96 = 0
_LOOPBACK_V6 = ipaddress.ip_address("::1")


def _effective_ip(
    ip: ipaddress.IPv4Address | ipaddress.IPv6Address,
) -> ipaddress.IPv4Address | ipaddress.IPv6Address:
    """Unmap IPv4-mapped and embedded-IPv4 IPv6 so loopback/private checks
    apply to the IPv4 (SSRF). Mirrors Rust `decode_embedded_v4`."""
    if isinstance(ip, ipaddress.IPv6Address):
        if ip.ipv4_mapped is not None:
            return ip.ipv4_mapped
        if ip == _LOOPBACK_V6:
            return ip
        value = int(ip)
        if value >> 32 in (_NAT64_WKP_96, _COMPAT_MAPPED_96, _IPV4_COMPATIBLE_96):
            return ipaddress.IPv4Address(value & 0xFFFFFFFF)
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


# Canonical URL-policy tables (A-10, 2026-08-10): IANA-registered ranges that
# are never globally routable, kept explicit so Python and Rust classify
# identically regardless of stdlib version (Python's `is_private`/`is_reserved`
# only grew CGNAT + these ranges in 3.13; the tables below are what keeps
# parity on older Pythons). Mirrors src-tauri/src/security.rs
# `is_canonical_reserved_v4/v6`. Embedded-IPv4 IPv6 forms are unmapped to
# their IPv4 before classification (`_effective_ip`), so only non-decodable
# NAT64/prefix members land in the v6 table.
_V4_NON_PUBLIC_NETS = [
    ipaddress.ip_network("10.0.0.0/8"),
    ipaddress.ip_network("100.64.0.0/10"),  # CGNAT/shared (RFC 6598)
    ipaddress.ip_network("172.16.0.0/12"),
    ipaddress.ip_network("192.0.0.0/24"),  # IETF protocol assignments
    ipaddress.ip_network("192.0.2.0/24"),  # TEST-NET-1 (documentation)
    ipaddress.ip_network("192.168.0.0/16"),
    ipaddress.ip_network("198.18.0.0/15"),  # benchmarking (RFC 2544)
    ipaddress.ip_network("198.51.100.0/24"),  # TEST-NET-2
    ipaddress.ip_network("203.0.113.0/24"),  # TEST-NET-3
]
_V6_NON_PUBLIC_NETS = [
    # RFC 4291 "reserved for future use" routing-type schema — the exact range
    # set Python `is_reserved` uses (verified vs ipaddress 3.13.3). `::/8`
    # covers the NAT64 well-known prefix 64:ff9b::/96 and deprecated
    # IPv4-compatible remnants; embedded-IPv4 forms are unmapped first.
    ipaddress.ip_network("::/8"),
    ipaddress.ip_network("100::/8"),
    ipaddress.ip_network("200::/7"),
    ipaddress.ip_network("400::/6"),
    ipaddress.ip_network("800::/5"),
    ipaddress.ip_network("1000::/4"),
    ipaddress.ip_network("4000::/3"),
    ipaddress.ip_network("6000::/3"),
    ipaddress.ip_network("8000::/3"),
    ipaddress.ip_network("a000::/3"),
    ipaddress.ip_network("c000::/3"),
    ipaddress.ip_network("e000::/4"),
    ipaddress.ip_network("f000::/5"),
    ipaddress.ip_network("f800::/6"),
    ipaddress.ip_network("fe00::/9"),
]


def _is_non_public_v4(ip: ipaddress.IPv4Address) -> bool:
    return bool(
        ip.is_loopback
        or ip.is_unspecified
        or ip.is_link_local
        or ip.is_multicast
        or int(ip) < 0x01000000  # 0.0.0.0/8 "this network" (Rust parity)
        or int(ip) >= 0xF0000000  # 240.0.0.0/4 class E + 255.255.255.255
        or any(ip in net for net in _V4_NON_PUBLIC_NETS)
    )


def _is_v6_iana_private(ip: ipaddress.IPv6Address) -> bool:
    """IANA ipv6-special-registry ranges that are "not globally reachable"
    (Python 3.13 `is_private` semantics, kept explicit for stdlib parity):
    2001::/23 except ORCHIDv2 2001:20::/28, 2001:db8::/32, 2002::/16,
    3fff::/20, 100::/64, fc00::/7."""
    b = ip.packed
    if b[0] == 0x20 and b[1] == 0x01:
        if (b[2] & 0xFE) == 0:
            if b[2] == 0x00 and (b[3] & 0xF0) == 0x20:  # ORCHIDv2 2001:20::/28
                return False
            return True
        if b[2] == 0x0D and b[3] == 0xB8:  # 2001:db8::/32
            return True
        return False
    if b[0] == 0x20 and b[1] == 0x02:  # 2002::/16
        return True
    if b[0] == 0x3F and b[1] == 0xFF and (b[2] & 0xF0) == 0:  # 3fff::/20
        return True
    if b[0] == 0x01 and b[1] == 0x00 and b[2] == 0 and b[3] == 0:  # 100::/64
        return True
    if (b[0] & 0xFE) == 0xFC:  # fc00::/7 ULA
        return True
    return False


def _is_non_public_v6(ip: ipaddress.IPv6Address) -> bool:
    return bool(
        ip.is_loopback
        or ip.is_unspecified
        or ip.is_link_local
        or ip.is_private  # IANA special-registry "not globally reachable"
        or ip.is_reserved  # RFC 4291 routing-type reserved schema
        or ip.is_multicast
        or _is_v6_iana_private(ip)
        or any(ip in net for net in _V6_NON_PUBLIC_NETS)
    )


def _block_unsafe_host(host: str) -> None:
    host_lower = _normalize_host(host)
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

    _block_ip(ip)


def _block_ip(
    ip: ipaddress.IPv4Address | ipaddress.IPv6Address,
) -> None:
    ip = _effective_ip(ip)
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


def resolve_host_ips(host: str) -> list[ipaddress.IPv4Address | ipaddress.IPv6Address]:
    """System-resolver lookup; empty on failure (NXDOMAIN, no network, ...).
    UnicodeError: non-IDN hosts are rejected earlier by validation, but keep
    this defensive so getaddrinfo can never bubble a 500 (NR-02 parity)."""
    try:
        infos = socket.getaddrinfo(host, None, proto=socket.IPPROTO_TCP)
    except (OSError, UnicodeError):
        return []
    ips: list[ipaddress.IPv4Address | ipaddress.IPv6Address] = []
    for _family, _type, _proto, _canon, sockaddr in infos:
        try:
            ip = _effective_ip(ipaddress.ip_address(sockaddr[0]))
        except ValueError:
            continue
        if ip not in ips:
            ips.append(ip)
    return ips


def check_resolved_host(
    host: str,
    ips: list[ipaddress.IPv4Address | ipaddress.IPv6Address],
) -> None:
    """Reject hostnames that resolve to non-public IPs. Empty resolution
    fails closed — the browser could not open the URL anyway."""
    if not ips:
        raise NetRailError(
            "OPEN_URL_DNS_UNRESOLVABLE",
            f"Could not resolve host {host}.",
        )
    for ip in ips:
        _block_ip(ip)


def pin_open_host(
    safe_url: str,
    resolver: Callable[[str], list[ipaddress.IPv4Address | ipaddress.IPv6Address]]
    | None = None,
) -> None:
    """Pin a validated open URL to its current DNS answers before the browser
    is spawned. IP-literal hosts were already checked by validate_open_url;
    only hostnames are resolved. `resolver` is injectable for tests."""
    parsed = urlparse(safe_url)
    host = _normalize_host(parsed.hostname or "")
    if host and _parse_host_ip(host) is None:
        check_resolved_host(host, (resolver or resolve_host_ips)(host))


def _validate_open_url_inner(url: str, depth: int) -> str:
    if depth > _MAX_REDIRECT_DEPTH:
        raise NetRailError("OPEN_URL_REDIRECT_DEPTH", "Too many redirect wrappers.")

    try:
        parsed = urlparse(url.strip())
    except ValueError:
        raise NetRailError("OPEN_URL_INVALID", "Invalid URL.") from None

    if parsed.scheme not in {"http", "https"}:
        if not parsed.scheme:
            raise NetRailError("OPEN_URL_INVALID", "Invalid URL.") from None
        raise NetRailError(
            "OPEN_URL_INVALID_SCHEME",
            f"Blocked URL scheme: {parsed.scheme}",
        )

    if parsed.username or parsed.password:
        raise NetRailError(
            "OPEN_URL_CREDENTIALS",
            "URLs with embedded credentials are not allowed.",
        )

    # Rust `url` crate rejects malformed port specs (":80:9604", ":8080.",
    # port > 65535) at parse time; urlparse tolerates them, so mirror here.
    try:
        port = parsed.port
    except ValueError:
        raise NetRailError("OPEN_URL_INVALID", "Invalid URL.") from None
    if port is not None and port > 65535:
        raise NetRailError("OPEN_URL_INVALID", "Invalid URL.")

    host = _normalize_host(parsed.hostname or "")
    if not host:
        raise NetRailError("OPEN_URL_NO_HOST", "URL must include a host.")

    if not _is_ascii(host):
        # Invalid IDN (e.g. %B4 → ´) — Rust url crate rejects at parse time
        # with OPEN_URL_INVALID; mirror it instead of crashing in getaddrinfo.
        raise NetRailError("OPEN_URL_INVALID", "Invalid URL.")

    if _parse_whatwg_ipv4(host) is _NUM_INVALID:
        # All-numeric host that fails WHATWG IPv4 rules (octal 8/9, octet >
        # 255, > 4 parts, ...) — Rust url crate fails the whole URL parse.
        raise NetRailError("OPEN_URL_INVALID", "Invalid URL.")

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


def _block_backend_host(host: str, *, strict: bool = False) -> None:
    host_lower = _normalize_host(host)
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

    if strict and host_lower in {"localhost", "127.0.0.1", "::1", "0.0.0.0", "[::1]"}:
        raise NetRailError(
            "BACKEND_URL_STRICT_PRIVATE",
            "strict_backend_urls rejects localhost backends.",
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
    if strict:
        if isinstance(ip, ipaddress.IPv4Address):
            private = _is_non_public_v4(ip) or ip.is_loopback
        else:
            private = _is_non_public_v6(ip) or ip.is_loopback
        if private:
            raise NetRailError(
                "BACKEND_URL_STRICT_PRIVATE",
                "strict_backend_urls rejects private/loopback backend hosts.",
            )


def check_backend_fetch_url(
    url: str,
    *,
    strict: bool = False,
    resolver: Callable[[str], list[ipaddress.IPv4Address | ipaddress.IPv6Address]] | None = None,
) -> str:
    """Fetch-time re-validation of a backend URL (A-05). Hostnames that will
    actually be fetched are resolved and the save-time rules are applied to
    every resolved address: cloud metadata and unspecified/link-local are
    always rejected, other non-public ranges only in strict mode. Empty
    resolution fails closed (BACKEND_URL_DNS_UNRESOLVABLE). Literal-IP
    backends are classified without DNS. ``resolver`` is injectable for tests
    (no DNS in tests).
    """
    trimmed = url.strip()
    try:
        parsed = urlparse(trimmed)
    except ValueError:
        raise NetRailError("BACKEND_URL_INVALID", "Invalid backend URL.") from None

    host = _normalize_host(parsed.hostname or "")
    if not host:
        raise NetRailError("BACKEND_URL_INVALID", "Invalid backend URL.")

    _block_backend_host(host, strict=strict)
    if _parse_host_ip(host) is not None:
        return trimmed

    ips = (resolver or resolve_host_ips)(host)
    if not ips:
        raise NetRailError(
            "BACKEND_URL_DNS_UNRESOLVABLE",
            f"Backend host {host} does not resolve.",
        )
    for raddr in ips:
        raddr = _effective_ip(raddr)  # production resolver returns effective IPs; keep injected forms consistent
        if _is_cloud_metadata_ip(raddr):
            raise NetRailError(
                "BACKEND_URL_CLOUD_METADATA",
                "Cloud metadata addresses cannot be used as backend URLs.",
            )
        if raddr.is_unspecified or raddr.is_link_local:
            raise NetRailError(
                "BACKEND_URL_LINK_LOCAL",
                "Unspecified or link-local addresses cannot be used as backend URLs.",
            )
        if strict:
            if isinstance(raddr, ipaddress.IPv4Address):
                private = _is_non_public_v4(raddr) or raddr.is_loopback
            else:
                private = _is_non_public_v6(raddr) or raddr.is_loopback
            if private:
                raise NetRailError(
                    "BACKEND_URL_STRICT_PRIVATE",
                    "strict_backend_urls rejects private/loopback backend hosts.",
                )
    return trimmed


def validate_backend_url(url: str, *, strict: bool = False) -> str:
    """Validate a user-configured backend URL (e.g. SearXNG).

    When ``strict`` is True, loopback and private/LAN hosts are rejected.
    """
    trimmed = url.strip()
    if not trimmed:
        raise NetRailError("BACKEND_URL_EMPTY", "Backend URL cannot be empty.")

    try:
        parsed = urlparse(trimmed)
    except ValueError:
        raise NetRailError("BACKEND_URL_INVALID", "Invalid backend URL.") from None

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

    try:
        port = parsed.port
    except ValueError:
        raise NetRailError("BACKEND_URL_INVALID", "Invalid backend URL.") from None
    if port is not None and port > 65535:
        raise NetRailError("BACKEND_URL_INVALID", "Invalid backend URL.")

    host = _normalize_host(parsed.hostname or "")
    if not host:
        raise NetRailError("BACKEND_URL_NO_HOST", "Backend URL must include a host.")

    if not _is_ascii(host):
        # Invalid IDN — Rust url crate fails to parse with BACKEND_URL_INVALID.
        raise NetRailError("BACKEND_URL_INVALID", "Invalid backend URL.")

    if _parse_whatwg_ipv4(host) is _NUM_INVALID:
        raise NetRailError("BACKEND_URL_INVALID", "Invalid backend URL.")

    _block_backend_host(host, strict=strict)
    return trimmed

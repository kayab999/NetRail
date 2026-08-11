import ipaddress

import pytest

from netrail.errors import NetRailError
from netrail.security import (
    check_backend_fetch_url,
    check_resolved_host,
    pin_open_host,
    resolve_host_ips,
    validate_backend_url,
    validate_open_url,
)


def test_accepts_https():
    assert validate_open_url("https://example.com/path") == "https://example.com/path"


def test_rejects_javascript():
    with pytest.raises(NetRailError) as exc:
        validate_open_url("javascript:alert(1)")
    assert exc.value.code == "OPEN_URL_INVALID_SCHEME"


def test_rejects_data_uri():
    with pytest.raises(NetRailError) as exc:
        validate_open_url("data:text/html,<script>")
    assert exc.value.code == "OPEN_URL_INVALID_SCHEME"


def test_rejects_credentials():
    with pytest.raises(NetRailError) as exc:
        validate_open_url("https://user:pass@example.com")
    assert exc.value.code == "OPEN_URL_CREDENTIALS"


def test_rejects_localhost():
    with pytest.raises(NetRailError) as exc:
        validate_open_url("http://127.0.0.1:8080/admin")
    assert exc.value.code == "OPEN_URL_LOCALHOST"


def test_rejects_nip_io():
    with pytest.raises(NetRailError) as exc:
        validate_open_url("http://127.0.0.1.nip.io/")
    assert exc.value.code == "OPEN_URL_DNS_REBINDING"


@pytest.mark.parametrize(
    "url",
    [
        "http://localtest.me/",
        "http://nip.io/",
        "http://sslip.io/",
        "http://xip.io/",
    ],
)
def test_rejects_rebinding_apex(url):
    with pytest.raises(NetRailError) as exc:
        validate_open_url(url)
    assert exc.value.code == "OPEN_URL_DNS_REBINDING"


def test_unwraps_duck_com_blocks_inner_localhost():
    ddg = "https://duck.com/l/?uddg=http%3A%2F%2F127.0.0.1%2F"
    with pytest.raises(NetRailError) as exc:
        validate_open_url(ddg)
    assert exc.value.code == "OPEN_URL_LOCALHOST"


def test_unwraps_ddg_redirect_blocks_inner_localhost():
    ddg = "https://duckduckgo.com/l/?uddg=http%3A%2F%2F127.0.0.1%2Fapi"
    with pytest.raises(NetRailError) as exc:
        validate_open_url(ddg)
    assert exc.value.code == "OPEN_URL_LOCALHOST"


def test_unwraps_ddg_redirect_to_safe_url():
    ddg = "https://duckduckgo.com/l/?uddg=https%3A%2F%2Frust-lang.org%2F"
    assert validate_open_url(ddg) == "https://rust-lang.org/"


def test_allows_localhost_searxng_url():
    assert validate_backend_url("http://127.0.0.1:8080") == "http://127.0.0.1:8080"


def test_rejects_metadata_backend_url():
    with pytest.raises(NetRailError) as exc:
        validate_backend_url("http://169.254.169.254/latest/meta-data/")
    assert exc.value.code == "BACKEND_URL_CLOUD_METADATA"


def test_rejects_nip_io_backend_url():
    with pytest.raises(NetRailError) as exc:
        validate_backend_url("http://127.0.0.1.nip.io/")
    assert exc.value.code == "BACKEND_URL_DNS_REBINDING"


@pytest.mark.parametrize(
    "url,code",
    [
        ("http://2130706433/", "OPEN_URL_LOCALHOST"),
        ("http://0x7f000001/", "OPEN_URL_LOCALHOST"),
        ("http://0177.0.0.1/", "OPEN_URL_LOCALHOST"),
        ("http://127.1/", "OPEN_URL_LOCALHOST"),
        ("http://192.168.1.1/", "OPEN_URL_PRIVATE"),
        ("http://10.0.0.1/", "OPEN_URL_PRIVATE"),
        ("http://172.16.0.1/", "OPEN_URL_PRIVATE"),
    ],
)
def test_rejects_encoded_and_private_open_urls(url, code):
    with pytest.raises(NetRailError) as exc:
        validate_open_url(url)
    assert exc.value.code == code


@pytest.mark.parametrize(
    "url",
    [
        "http://011.119.190.078/",
        "http://08.02.01.1/",
        "http://011778/",
        "http://999.1.2.3/",
        "http://1.2.3.4.5/",
        "http://1.2..3/",
        "http://1.16777216/",
        "http://1.1.1.1:65536/",
        "http://139.241.:80:9604/",
        "http://103.66.236.176:8080./",
    ],
)
def test_rejects_whatwg_invalid_numeric_hosts(url):
    with pytest.raises(NetRailError) as exc:
        validate_open_url(url)
    assert exc.value.code == "OPEN_URL_INVALID"


@pytest.mark.parametrize(
    "url",
    [
        "http://1955950671/",
        "http://0x1ddB5d6/",
        "http://1.2/",
        "http://1.16777215/",
        "http://1.1.1.1:0/",
        "http://1.1.1.1:65535/",
    ],
)
def test_allows_whatwg_numeric_hosts(url):
    assert validate_open_url(url) == url


@pytest.mark.parametrize(
    "url",
    [
        "http://08.02.01.1/",
        "http://1.1.1.1:65536/",
        "http://103.66.236.176:8080./",
    ],
)
def test_rejects_whatwg_invalid_backend_urls(url):
    with pytest.raises(NetRailError) as exc:
        validate_backend_url(url)
    assert exc.value.code == "BACKEND_URL_INVALID"


def test_allows_private_backend_for_searxng():
    assert validate_backend_url("http://192.168.0.5:8080") == "http://192.168.0.5:8080"


def test_pin_open_host_blocks_loopback_resolution():
    with pytest.raises(NetRailError) as exc:
        pin_open_host("http://internal.corp/", resolver=lambda _h: [ipaddress.ip_address("127.0.0.1")])
    assert exc.value.code == "OPEN_URL_LOCALHOST"


def test_pin_open_host_blocks_private_resolution():
    with pytest.raises(NetRailError) as exc:
        pin_open_host(
            "https://evil.example/",
            resolver=lambda _h: [ipaddress.ip_address("192.168.1.10")],
        )
    assert exc.value.code == "OPEN_URL_PRIVATE"


def test_pin_open_host_blocks_link_local_resolution():
    with pytest.raises(NetRailError) as exc:
        pin_open_host(
            "http://metadata-helper.example/",
            resolver=lambda _h: [ipaddress.ip_address("169.254.169.254")],
        )
    assert exc.value.code == "OPEN_URL_LINK_LOCAL"


def test_pin_open_host_allows_public_resolution():
    pin_open_host(
        "https://example.org/",
        resolver=lambda _h: [ipaddress.ip_address("93.184.216.34")],
    )


def test_pin_open_host_fails_closed_on_unresolvable_host():
    with pytest.raises(NetRailError) as exc:
        pin_open_host("https://nxdomain.invalid/", resolver=lambda _h: [])
    assert exc.value.code == "OPEN_URL_DNS_UNRESOLVABLE"


def test_pin_open_host_skips_ip_literals():
    pin_open_host(
        "https://93.184.216.34/",
        resolver=lambda h: pytest.fail(f"resolver must not run for IP literals, got {h}"),
    )


def test_pin_open_host_blocks_any_non_public_answer():
    with pytest.raises(NetRailError) as exc:
        pin_open_host(
            "https://dual.example/",
            resolver=lambda _h: [
                ipaddress.ip_address("::1"),
                ipaddress.ip_address("1.2.3.4"),
            ],
        )
    assert exc.value.code == "OPEN_URL_LOCALHOST"


def test_check_resolved_host_empty_is_unresolvable():
    with pytest.raises(NetRailError) as exc:
        check_resolved_host("nx.example", [])
    assert exc.value.code == "OPEN_URL_DNS_UNRESOLVABLE"


def test_resolve_host_ips_returns_public_ip_for_known_host():
    ips = resolve_host_ips("example.com")
    if not ips:
        pytest.skip("system resolver unavailable")
    for ip in ips:
        assert not ip.is_private and not ip.is_loopback and not ip.is_link_local


# --- S1: Invariant & Property Tests ---

def test_property_normalize_host_is_idempotent():
    from netrail.security import _normalize_host

    sample_hosts = [
        "127.0.0.1.",
        "DUCKDUCKGO.COM..",
        "192.168.1.1",
        "metadata.google.internal.",
        "EXAMPLE.COM%2Fpath",
        "127.0.0.1",
        "localhost",
        "[::1]",
    ]
    for host in sample_hosts:
        once = _normalize_host(host)
        twice = _normalize_host(once)
        assert once == twice, f"idempotency failed for host: {host}"


def test_property_parse_browser_ipv4_never_panics_on_arbitrary_input():
    from netrail.security import _parse_browser_ipv4

    fuzz_inputs = [
        "",
        "   ",
        ".",
        "...",
        "0",
        "0x",
        "0xGG",
        "256.256.256.256",
        "127.0.0.1.1",
        "-1",
        "99999999999999999999999999999",
        "0177.0.0.1",
        "0x7f000001",
        "2130706433",
        "127.1",
        "127.0.1",
        "192.168.1.1",
        "../../../etc/passwd",
        "\x00\x01\x02",
    ]
    for inp in fuzz_inputs:
        _ = _parse_browser_ipv4(inp)


# --- A-05: backend fetch-time SSRF guard (check_backend_fetch_url) ---

def _backend_ips(*addrs: str):
    return lambda _h: [ipaddress.ip_address(a) for a in addrs]


def _expect_backend_code(url: str, code: str, *, strict: bool = False, ips=None):
    with pytest.raises(NetRailError) as exc:
        check_backend_fetch_url(url, strict=strict, resolver=ips)
    assert exc.value.code == code


def test_backend_fetch_blocks_metadata_resolution_non_strict():
    _expect_backend_code(
        "http://company-searxng.internal:8080",
        "BACKEND_URL_CLOUD_METADATA",
        ips=_backend_ips("169.254.169.254"),
    )


def test_backend_fetch_blocks_aws_metadata_ipv6_resolution():
    _expect_backend_code(
        "http://searxng.example:8080",
        "BACKEND_URL_CLOUD_METADATA",
        ips=_backend_ips("fd00:ec2::254"),
    )


def test_backend_fetch_blocks_link_local_resolution_non_strict():
    _expect_backend_code(
        "http://searxng.lan:8080",
        "BACKEND_URL_LINK_LOCAL",
        ips=_backend_ips("fe80::1"),
    )


def test_backend_fetch_blocks_unspecified_resolution_non_strict():
    _expect_backend_code(
        "http://searxng.lan:8080",
        "BACKEND_URL_LINK_LOCAL",
        ips=_backend_ips("::"),
    )


def test_backend_fetch_fails_closed_on_empty_resolution():
    _expect_backend_code(
        "http://gone.invalid:8080",
        "BACKEND_URL_DNS_UNRESOLVABLE",
        ips=_backend_ips(),
    )


def test_backend_fetch_blocks_private_resolution_strict_only():
    _expect_backend_code(
        "http://searxng.lan:8080",
        "BACKEND_URL_STRICT_PRIVATE",
        strict=True,
        ips=_backend_ips("10.0.0.5"),
    )
    assert (
        check_backend_fetch_url(
            "http://searxng.lan:8080", strict=False, resolver=_backend_ips("10.0.0.5")
        )
        == "http://searxng.lan:8080"
    )


def test_backend_fetch_allows_public_resolution_strict():
    assert (
        check_backend_fetch_url(
            "http://searxng.example.com:8080",
            strict=True,
            resolver=_backend_ips("93.184.216.34"),
        )
        == "http://searxng.example.com:8080"
    )


def test_backend_fetch_never_resolves_ip_literals():
    check_backend_fetch_url(
        "http://127.0.0.1:8080",
        resolver=lambda h: pytest.fail(f"resolver must not run for literals, got {h}"),
    )
    check_backend_fetch_url(
        "http://[fd00::1]:8080",
        resolver=lambda h: pytest.fail(f"resolver must not run for literals, got {h}"),
    )


def test_backend_fetch_blocked_hostname_search_raises():
    from netrail.backends.searxng import SearXNGBackend

    backend = SearXNGBackend(
        "http://company-searxng.internal:8080",
        resolver=_backend_ips("169.254.169.254"),
    )
    with pytest.raises(NetRailError) as exc:
        backend.search("q", "web", 10)
    assert exc.value.code == "BACKEND_URL_CLOUD_METADATA"


def test_backend_fetch_blocked_hostname_is_available_false():
    from netrail.backends.searxng import SearXNGBackend

    backend = SearXNGBackend(
        "http://company-searxng.internal:8080",
        resolver=_backend_ips("169.254.169.254"),
    )
    assert backend.is_available() is False
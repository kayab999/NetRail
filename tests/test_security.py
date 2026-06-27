import pytest

from netrail.errors import NetRailError
from netrail.security import validate_backend_url, validate_open_url


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
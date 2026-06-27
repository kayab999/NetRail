import pytest

from netrail.security import validate_backend_url, validate_open_url


def test_accepts_https():
    assert validate_open_url("https://example.com/path") == "https://example.com/path"


def test_rejects_javascript():
    with pytest.raises(ValueError, match="http"):
        validate_open_url("javascript:alert(1)")


def test_rejects_data_uri():
    with pytest.raises(ValueError, match="http"):
        validate_open_url("data:text/html,<script>")


def test_rejects_credentials():
    with pytest.raises(ValueError, match="credentials"):
        validate_open_url("https://user:pass@example.com")


def test_rejects_localhost():
    with pytest.raises(ValueError, match="Localhost"):
        validate_open_url("http://127.0.0.1:8080/admin")


def test_rejects_nip_io():
    with pytest.raises(ValueError, match="DNS rebinding"):
        validate_open_url("http://127.0.0.1.nip.io/")


def test_unwraps_ddg_redirect_blocks_inner_localhost():
    ddg = "https://duckduckgo.com/l/?uddg=http%3A%2F%2F127.0.0.1%2Fapi"
    with pytest.raises(ValueError, match="Localhost"):
        validate_open_url(ddg)


def test_unwraps_ddg_redirect_to_safe_url():
    ddg = "https://duckduckgo.com/l/?uddg=https%3A%2F%2Frust-lang.org%2F"
    assert validate_open_url(ddg) == "https://rust-lang.org/"


def test_allows_localhost_searxng_url():
    assert validate_backend_url("http://127.0.0.1:8080") == "http://127.0.0.1:8080"


def test_rejects_metadata_backend_url():
    with pytest.raises(ValueError, match="metadata"):
        validate_backend_url("http://169.254.169.254/latest/meta-data/")


def test_rejects_nip_io_backend_url():
    with pytest.raises(ValueError, match="rebinding"):
        validate_backend_url("http://127.0.0.1.nip.io/")
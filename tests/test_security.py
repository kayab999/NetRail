import pytest

from netrail.security import validate_open_url


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
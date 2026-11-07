"""P0 SearXNG config discovery + overlap diagnostics (offline)."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

import httpx
import pytest

from netrail.backends.searxng import SearXNGBackend, fetch_config, _clear_config_cache


def _httpx_response(payload, status=200):
    response = MagicMock()
    response.status_code = status
    if status >= 400:
        response.raise_for_status.side_effect = httpx.HTTPStatusError(
            f"{status}", request=MagicMock(), response=MagicMock()
        )
    else:
        response.raise_for_status.return_value = None
    response.json.return_value = payload
    return response


def _client_mock(response):
    client = MagicMock()
    client.__enter__.return_value = client
    client.__exit__.return_value = False
    client.get.return_value = response
    return client


def test_fetch_config_complete():
    payload = {
        "categories": ["general", "science", "it"],
        "engines": [
            {"name": "duckduckgo", "enabled": True, "categories": ["general"]},
            {"name": "arxiv", "enabled": True, "categories": ["science"]},
            {"name": "brave", "enabled": False, "categories": ["general"]},
        ],
    }
    with patch("netrail.backends.searxng.httpx.Client", return_value=_client_mock(_httpx_response(payload))):
        _clear_config_cache()
        result = fetch_config("http://127.0.0.1:8080")
    assert result["status"] == "ok"
    assert result["categories"] == ["general", "science", "it"]
    assert len(result["engines"]) == 3


def test_fetch_config_partial_unknown_fields():
    payload = {"engines": [{"name": "x", "categories": ["web"], "foo": "bar"}]}
    with patch("netrail.backends.searxng.httpx.Client", return_value=_client_mock(_httpx_response(payload))):
        _clear_config_cache()
        result = fetch_config("http://127.0.0.1:8080")
    assert result["status"] == "ok"
    assert "web" in result["categories"]  # derived


def test_fetch_config_403_unavailable():
    with patch("netrail.backends.searxng.httpx.Client", return_value=_client_mock(_httpx_response({}, status=403))):
        _clear_config_cache()
        result = fetch_config("http://127.0.0.1:8080")
    assert result["status"] == "unavailable"
    assert "403" in (result["error"] or "")


def test_fetch_config_invalid_json():
    response = MagicMock()
    response.status_code = 200
    response.raise_for_status.return_value = None
    response.json.side_effect = ValueError("bad")
    with patch("netrail.backends.searxng.httpx.Client", return_value=_client_mock(response)):
        _clear_config_cache()
        result = fetch_config("http://127.0.0.1:8080")
    assert result["status"] == "invalid"


def test_fetch_config_cache():
    payload = {"categories": ["general"], "engines": []}
    with patch("netrail.backends.searxng.httpx.Client", return_value=_client_mock(_httpx_response(payload))) as mock_client:
        _clear_config_cache()
        fetch_config("http://127.0.0.1:8080")
        fetch_config("http://127.0.0.1:8080")
        assert mock_client.call_count == 1


def test_search_403_distinct_from_config_403():
    # search endpoint 403 raises HTTPStatusError; config 403 yields unavailable.
    backend = SearXNGBackend("http://127.0.0.1:8080")
    with patch("netrail.backends.searxng.httpx.Client", return_value=_client_mock(_httpx_response({}, status=403))):
        with pytest.raises(httpx.HTTPStatusError):
            backend.search("rust", "web", 10)


# (overlap diagnostics covered via API in future tests)
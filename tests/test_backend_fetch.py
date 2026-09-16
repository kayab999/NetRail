"""Phase 3c: backend fetch matrix — offline coverage for the previously
untested fetch/parse code in brave, ddgs, wikipedia and searxng.

No live network: Brave/SearXNG patch `httpx.Client` in their module
namespace, Wikipedia injects an `httpx.MockTransport` client (supported by
its constructor), DDGS patches the third-party `DDGS` class.
"""

from __future__ import annotations

from unittest.mock import MagicMock, patch

import httpx
import pytest

from netrail.backends.brave import BraveBackend
from netrail.backends.ddgs import DDGSBackend
from netrail.backends.searxng import SearXNGBackend
from netrail.backends.wikipedia import WikipediaBackend
from netrail.errors import NetRailError


def _httpx_response(payload, status: int = 200) -> MagicMock:
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


def _client_mock(response: MagicMock) -> MagicMock:
    client = MagicMock()
    client.__enter__.return_value = client
    client.__exit__.return_value = False
    client.get.return_value = response
    return client


# ── Brave ─────────────────────────────────────────────────────────────

def test_brave_web_success_parses_results():
    payload = {
        "web": {
            "results": [
                {"url": "https://a.test/1", "title": "A", "description": "desc a"},
                {"url": "", "title": "Skipped"},
                {"url": "https://a.test/2"},
            ]
        }
    }
    with patch(
        "netrail.backends.brave.httpx.Client",
        return_value=_client_mock(_httpx_response(payload)),
    ):
        results = BraveBackend("key").search("rust", "web", 10)
    assert len(results) == 2
    assert results[0].url == "https://a.test/1"
    assert results[0].snippet == "desc a"
    assert results[0].backend == "brave"
    assert results[1].title == "https://a.test/2"


def test_brave_images_success_parses_thumbnails():
    payload = {
        "results": [
            {
                "url": "https://img.test/1.jpg",
                "title": "Pic",
                "source": "img.test",
                "thumbnail": {"src": "https://img.test/t1.jpg"},
            }
        ]
    }
    with patch(
        "netrail.backends.brave.httpx.Client",
        return_value=_client_mock(_httpx_response(payload)),
    ):
        results = BraveBackend("key").search("cats", "images", 10)
    assert len(results) == 1
    assert results[0].image == "https://img.test/t1.jpg"


def test_brave_http_error_raises():
    with patch(
        "netrail.backends.brave.httpx.Client",
        return_value=_client_mock(_httpx_response({}, status=429)),
    ):
        with pytest.raises(httpx.HTTPStatusError):
            BraveBackend("key").search("rust", "web", 10)


def test_brave_from_env_missing_is_none(monkeypatch):
    monkeypatch.delenv("BRAVE_SEARCH_API_KEY", raising=False)
    monkeypatch.delenv("NETRAIL_BRAVE_API_KEY", raising=False)
    assert BraveBackend.from_env() is None


# ── Wikipedia (MockTransport, no patching needed) ─────────────────────

OPENSEARCH = [
    "music",
    ["Music"],
    ["Art form"],
    ["https://en.wikipedia.org/wiki/Music"],
]
EXTRACTS = {"query": {"pages": {"1": {"title": "Music", "extract": "Music is\n\nan art."}}}}


def _wiki_client(opensearch_payload, extracts_payload=None) -> httpx.Client:
    def handler(request: httpx.Request) -> httpx.Response:
        params = dict(request.url.params)
        if params.get("action") == "opensearch":
            return httpx.Response(200, json=opensearch_payload)
        return httpx.Response(200, json=extracts_payload or {})

    return httpx.Client(transport=httpx.MockTransport(handler))


def test_wikipedia_opensearch_with_descriptions_needs_no_extracts():
    backend = WikipediaBackend(client=_wiki_client(OPENSEARCH))
    results = backend.search("music", "web", 10)
    assert len(results) == 1
    assert results[0].snippet == "Art form"
    assert results[0].backend == "wikipedia"


def test_wikipedia_empty_description_falls_back_to_extracts():
    opensearch = ["music", ["Music"], [""], ["https://en.wikipedia.org/wiki/Music"]]
    backend = WikipediaBackend(client=_wiki_client(opensearch, EXTRACTS))
    results = backend.search("music", "web", 10)
    assert len(results) == 1
    assert results[0].snippet == "Music is an art."


def test_wikipedia_http_error_raises():
    client = httpx.Client(transport=httpx.MockTransport(lambda req: httpx.Response(500)))
    with pytest.raises(httpx.HTTPStatusError):
        WikipediaBackend(client=client).search("music", "web", 10)


def test_wikipedia_malformed_payload_is_empty():
    backend = WikipediaBackend(client=_wiki_client({"unexpected": "shape"}))
    assert backend.search("music", "web", 10) == []


def test_wikipedia_non_web_needs_no_network():
    assert WikipediaBackend().search("cats", "images", 10) == []


# ── DDGS (third-party class patched) ──────────────────────────────────

def _ddgs_mock(text=None, images=None) -> MagicMock:
    mock = MagicMock()
    mock.__enter__.return_value = mock
    mock.__exit__.return_value = False
    mock.text.return_value = text or []
    mock.images.return_value = images or []
    return mock


def test_ddgs_text_maps_fields_and_filters_missing_href():
    raw = [
        {"title": "A", "href": "https://a.test/1", "body": "snippet a"},
        {"title": "No href"},
        {"href": "https://a.test/2", "body": "untitled uses href"},
    ]
    with patch("netrail.backends.ddgs.DDGS", return_value=_ddgs_mock(text=raw)):
        results = DDGSBackend().search("rust", "web", 10)
    assert len(results) == 2
    assert results[0].title == "A"
    assert results[0].snippet == "snippet a"
    assert results[1].title == "https://a.test/2"


def test_ddgs_images_maps_thumbnail_and_source():
    raw = [
        {
            "title": "Pic",
            "url": "https://img.test/1.jpg",
            "image": "https://img.test/1.jpg",
            "thumbnail": "https://img.test/t1.jpg",
            "source": "img.test",
        },
        {"title": "Skipped"},
    ]
    with patch("netrail.backends.ddgs.DDGS", return_value=_ddgs_mock(images=raw)):
        results = DDGSBackend().search("cats", "images", 10)
    assert len(results) == 1
    assert results[0].image == "https://img.test/t1.jpg"
    assert results[0].source == "img.test"


# ── SearXNG ───────────────────────────────────────────────────────────

def test_searxng_search_parses_results():
    payload = {
        "results": [
            {
                "url": "https://a.test/1",
                "title": "A",
                "content": "snippet",
                "engine": "bing",
                "thumbnail": "https://a.test/t.jpg",
            },
            {"title": "Skipped, no url"},
        ]
    }
    backend = SearXNGBackend("http://127.0.0.1:8080")
    with patch(
        "netrail.backends.searxng.httpx.Client",
        return_value=_client_mock(_httpx_response(payload)),
    ):
        results = backend.search("rust", "web", 10)
    assert len(results) == 1
    assert results[0].title == "A"
    assert results[0].source == "bing"
    assert results[0].image == "https://a.test/t.jpg"


def test_searxng_search_http_error_raises():
    backend = SearXNGBackend("http://127.0.0.1:8080")
    with patch(
        "netrail.backends.searxng.httpx.Client",
        return_value=_client_mock(_httpx_response({}, status=500)),
    ):
        with pytest.raises(httpx.HTTPStatusError):
            backend.search("rust", "web", 10)


def test_searxng_strict_rejects_loopback_before_network():
    backend = SearXNGBackend("http://127.0.0.1:8080", strict=True)
    with pytest.raises(NetRailError) as exc_info:
        backend.search("rust", "web", 10)
    assert exc_info.value.code == "BACKEND_URL_STRICT_PRIVATE"

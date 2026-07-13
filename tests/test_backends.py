from unittest.mock import MagicMock, patch

from netrail.backends.ddgs import DDGSBackend, PROVENANCE
from netrail.backends.registry import search_with_fallback
from netrail.backends.types import SearchResult
from netrail.backends.wikipedia import WikipediaBackend, _normalize_extract


def test_ddgs_backend_metadata():
    backend = DDGSBackend()
    assert backend.name == "ddgs"
    assert "Bing" in backend.provenance
    assert "site:" in backend.supports_operators


def test_fallback_returns_ddgs_results():
    fake = [
        SearchResult(title="A", url="https://a.test", backend="ddgs", provenance=PROVENANCE),
        SearchResult(title="B", url="https://b.test", backend="ddgs", provenance=PROVENANCE),
    ]
    with patch.object(DDGSBackend, "search", return_value=fake):
        response = search_with_fallback("python", max_results=5, settings={"ddgs_enabled": True, "backend_order": ["ddgs"]})
    assert len(response.results) == 2
    assert response.backends_used == ["ddgs"]
    assert response.sovereignty_step == 1


def test_dedupe_on_merge():
    fake = [SearchResult(title="A", url="https://dup.test/", backend="ddgs", provenance=PROVENANCE)]
    backend_a = MagicMock()
    backend_a.name = "ddgs"
    backend_a.provenance = PROVENANCE
    backend_a.is_available.return_value = True
    backend_a.search.return_value = fake

    backend_b = MagicMock()
    backend_b.name = "searxng"
    backend_b.provenance = "SearXNG local"
    backend_b.is_available.return_value = True
    backend_b.search.return_value = [
        SearchResult(title="A2", url="https://dup.test", backend="searxng", provenance="SearXNG local")
    ]

    with patch("netrail.backends.registry.get_enabled_backends", return_value=[backend_a, backend_b]):
        response = search_with_fallback("test", max_results=10)
    assert len(response.results) == 1
    assert response.sovereignty_step == 3


def test_empty_backend_surfaces_error_and_wikipedia_fallback():
    empty = MagicMock()
    empty.name = "ddgs"
    empty.provenance = PROVENANCE
    empty.is_available.return_value = True
    empty.search.return_value = []

    wiki_hit = [
        SearchResult(
            title="Music",
            url="https://en.wikipedia.org/wiki/Music",
            snippet="Art form",
            backend="wikipedia",
            provenance="Wikipedia",
        )
    ]
    with (
        patch("netrail.backends.registry.get_enabled_backends", return_value=[empty]),
        patch.object(WikipediaBackend, "search", return_value=wiki_hit),
    ):
        response = search_with_fallback("music", max_results=5, settings={"backend_order": ["ddgs"]})

    assert len(response.results) == 1
    assert response.results[0].backend == "wikipedia"
    assert "ddgs: returned no results" in response.errors
    assert "wikipedia" in response.backends_used


def test_wikipedia_normalize_extract():
    assert _normalize_extract("Music is\n\nan art.") == "Music is an art."
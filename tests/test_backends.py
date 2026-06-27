from unittest.mock import MagicMock, patch

from netrail.backends.ddgs import DDGSBackend, PROVENANCE
from netrail.backends.registry import search_with_fallback
from netrail.backends.types import SearchResult


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
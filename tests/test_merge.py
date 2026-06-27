from netrail.backends.merge import dedupe_results, interleave_batches, merge_fanout, normalize_url_key
from netrail.backends.types import SearchResult


def _result(url: str, snippet: str, backend: str) -> SearchResult:
    return SearchResult(title="T", url=url, snippet=snippet, backend=backend, provenance="")


def test_normalize_strips_tracking():
    a = normalize_url_key("https://www.Example.com/x?utm_source=ads&id=1")
    b = normalize_url_key("https://example.com/x?id=1")
    assert a == b


def test_dedupe_keeps_richer_snippet():
    items = [
        _result("https://a.test", "short", "ddgs"),
        _result("https://a.test/", "much longer snippet", "searxng"),
    ]
    merged = dedupe_results(items)
    assert len(merged) == 1
    assert merged[0].snippet == "much longer snippet"
    assert merged[0].backend == "searxng"


def test_interleave_backends():
    batches = [
        [_result("https://a/1", "", "ddgs"), _result("https://a/2", "", "ddgs")],
        [_result("https://b/1", "", "searxng")],
    ]
    out = interleave_batches(batches, 10)
    assert [r.backend for r in out] == ["ddgs", "searxng", "ddgs"]


def test_merge_fanout_dedupes_and_interleaves():
    batches = [
        ("ddgs", [_result("https://shared", "a", "ddgs"), _result("https://only-ddgs", "", "ddgs")]),
        ("searxng", [_result("https://shared/", "longer snippet wins", "searxng")]),
    ]
    out = merge_fanout(batches, 10)
    assert len(out) == 2
    assert out[0].url.startswith("https://shared")
    assert out[0].snippet == "longer snippet wins"
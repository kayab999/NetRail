from netrail.backends.merge import (
    dedupe_results,
    interleave_batches,
    merge_fanout,
    normalize_url_key,
    resolve_result_url,
)
from netrail.backends.types import SearchResult


def _result(url: str, snippet: str, backend: str) -> SearchResult:
    return SearchResult(title="T", url=url, snippet=snippet, backend=backend, provenance="")


def test_resolve_ddg_redirect():
    raw = "https://duckduckgo.com/l/?uddg=https%3A%2F%2Frust-lang.org%2F"
    assert resolve_result_url(raw) == "https://rust-lang.org/"


def test_normalize_strips_tracking():
    a = normalize_url_key("https://www.Example.com/x?utm_source=ads&id=1")
    b = normalize_url_key("https://example.com/x?id=1")
    assert a == b


def test_normalize_scheme_and_host_case_insensitive():
    """RFC 3986: scheme + host are case-insensitive."""
    assert normalize_url_key("HTTPS://EXAMPLE.com/Page") == normalize_url_key(
        "https://example.com/Page"
    )
    assert normalize_url_key("https://www.example.com/Page") == normalize_url_key(
        "https://example.com/Page"
    )


def test_normalize_path_case_sensitive():
    """Path is case-sensitive — /Page and /page are different resources."""
    assert normalize_url_key("https://example.com/Page") != normalize_url_key(
        "https://example.com/page"
    )


def test_normalize_query_case_sensitive():
    assert normalize_url_key("https://example.com/search?q=Hello") != normalize_url_key(
        "https://example.com/search?q=hello"
    )
    assert normalize_url_key("https://example.com/Page?A=1") != normalize_url_key(
        "https://example.com/Page?a=1"
    )


def test_normalize_query_order_canonical():
    assert normalize_url_key("https://example.com/Page?a=1&b=2") == normalize_url_key(
        "https://example.com/Page?b=2&a=1"
    )


def test_normalize_fragment_stripped():
    assert normalize_url_key("https://example.com/Page#section") == normalize_url_key(
        "https://example.com/Page"
    )


def test_normalize_default_port_stripped_non_default_kept():
    assert normalize_url_key("https://example.com:443/Page") == normalize_url_key(
        "https://example.com/Page"
    )
    assert normalize_url_key("https://example.com:8443/Page") != normalize_url_key(
        "https://example.com/Page"
    )


def test_normalize_tracking_match_case_insensitive():
    assert normalize_url_key("https://example.com/Page?UTM_SOURCE=x&id=1") == (
        normalize_url_key("https://example.com/Page?id=1")
    )


def test_normalize_trailing_slash_and_root():
    assert normalize_url_key("https://example.com/Page/") == normalize_url_key(
        "https://example.com/Page"
    )
    assert normalize_url_key("https://example.com") == normalize_url_key(
        "https://example.com/"
    )


def test_normalize_plus_unifies_with_percent20_space():
    """Raw `+` in a query is form-encoding for space — same key as `%20`."""
    assert normalize_url_key(
        "https://example.com/search?q=rust+programming"
    ) == normalize_url_key("https://example.com/search?q=rust%20programming")


def test_normalize_encoded_plus_stays_distinct_from_space():
    """`%2B` is a literal plus — must NOT merge with a space."""
    assert normalize_url_key("https://example.com/calc?a=1%2B2") != normalize_url_key(
        "https://example.com/calc?a=1%202"
    )


def test_normalize_plus_in_path_untouched():
    """Only the query portion normalizes `+`; path `+` is literal."""
    key = normalize_url_key("https://example.com/a+b?x=1")
    assert "a+b" in key
    assert "?x=1" in key


def test_normalize_exact_keys_match_rust_parity():
    """Byte-exact pins shared with Rust — identical dedupe keys per input."""
    assert (
        normalize_url_key("https://EXAMPLE.com/Page") == "https://example.com/Page"
    )
    assert (
        normalize_url_key("https://www.example.com/Page?a=1&b=2&utm_source=x")
        == "https://example.com/Page?a=1&b=2"
    )
    assert (
        normalize_url_key("https://example.com:443/Page#s")
        == "https://example.com/Page"
    )
    assert (
        normalize_url_key("http://example.com:8080/A?B=C")
        == "http://example.com:8080/A?B=C"
    )
    assert normalize_url_key("https://example.com") == "https://example.com"
    assert (
        normalize_url_key("https://example.com/?id=1")
        == "https://example.com/?id=1"
    )
    assert (
        normalize_url_key("https://example.com/search?q=rust+programming")
        == "https://example.com/search?q=rust%20programming"
    )
    assert (
        normalize_url_key("https://example.com/calc?a=1%2B2")
        == "https://example.com/calc?a=1+2"
    )


def test_dedupe_respects_path_case():
    items = [
        _result("https://a.test/Page", "first", "ddgs"),
        _result("https://a.test/page", "second", "searxng"),
    ]
    assert len(dedupe_results(items)) == 2


def test_merge_fanout_dedupes_host_case_not_path_case():
    batches = [
        ("ddgs", [_result("https://EXAMPLE.com/Page", "a", "ddgs")]),
        ("searxng", [_result("https://example.com/Page", "longer snippet", "searxng")]),
    ]
    out = merge_fanout(batches, 10)
    assert len(out) == 1
    batches = [
        ("ddgs", [_result("https://example.com/Page", "a", "ddgs")]),
        ("searxng", [_result("https://example.com/page", "b", "searxng")]),
    ]
    assert len(merge_fanout(batches, 10)) == 2


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
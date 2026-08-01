from fastapi.testclient import TestClient

from netrail import rate_limit
from netrail.main import app

client = TestClient(app)


def test_health_reports_provenance():
    response = client.get("/api/health")
    assert response.status_code == 200
    payload = response.json()
    assert payload["telemetry"] == "none"
    assert "Bing" in payload["default_provenance"]
    assert "search_recovery" in payload
    recovery = payload["search_recovery"]
    assert "searxng_configured" in recovery
    assert "brave_key_present" in recovery
    assert "hints" in recovery
    assert isinstance(recovery["hints"], list)


def test_health_parity_shape_with_rust_contract():
    """Python health must expose the same product fields as Rust netrail-api."""
    from netrail import __version__

    payload = client.get("/api/health").json()
    assert payload["status"] == "ok"
    assert payload["version"] == __version__
    assert payload.get("api_contract") == "1.4"
    assert "rate_limit" in payload
    assert "enabled" in payload["rate_limit"]
    assert "search_per_minute" in payload["rate_limit"]
    assert "open_per_minute" in payload["rate_limit"]
    assert "mutate_per_minute" in payload["rate_limit"]
    assert "search_recovery" in payload
    assert "backends_configured" in payload
    assert "auth" in payload
    assert "token_required" in payload["auth"]
    assert "strict_backend_urls" in payload
    assert "audit_log" in payload


def test_backends_endpoint():
    response = client.get("/api/backends")
    assert response.status_code == 200
    backends = response.json()
    assert any(item["name"] == "ddgs" for item in backends)


def test_open_rejects_localhost_with_code():
    response = client.post("/api/open", json={"url": "http://127.0.0.1:8080"})
    assert response.status_code == 400
    payload = response.json()
    assert payload["code"] == "OPEN_URL_LOCALHOST"
    assert payload["status"] == 400
    assert "detail" in payload


def test_open_rejects_encoded_loopback_and_private():
    for url, code in [
        ("http://2130706433/", "OPEN_URL_LOCALHOST"),
        ("http://192.168.1.1/", "OPEN_URL_PRIVATE"),
    ]:
        response = client.post("/api/open", json={"url": url})
        assert response.status_code == 400, url
        assert response.json()["code"] == code


def test_search_empty_query_returns_typed_code():
    response = client.post("/api/search", json={"query": "", "mode": "web"})
    assert response.status_code == 400
    payload = response.json()
    assert payload["code"] == "QUERY_INVALID"
    assert payload["status"] == 400


def test_unknown_doc_returns_doc_not_found_with_code():
    response = client.get("/api/docs/unknown-slug")
    assert response.status_code == 404
    payload = response.json()
    assert payload["code"] == "DOC_NOT_FOUND"
    assert payload["status"] == 404


def test_csp_header_on_index():
    response = client.get("/")
    assert "Content-Security-Policy" in response.headers


def test_history_disabled_returns_history_disabled(monkeypatch, tmp_path):
    monkeypatch.setenv("NETRAIL_HISTORY_ENABLED", "0")
    from netrail.history.store import reset_store_for_tests

    reset_store_for_tests()
    response = client.get("/api/history")
    assert response.status_code == 400
    assert response.json()["code"] == "HISTORY_DISABLED"
    monkeypatch.delenv("NETRAIL_HISTORY_ENABLED", raising=False)
    reset_store_for_tests()


def test_collection_empty_name_returns_collection_name_invalid():
    response = client.post("/api/collections", json={"name": ""})
    assert response.status_code == 400
    assert response.json()["code"] == "COLLECTION_NAME_INVALID"


def test_invalid_mode_returns_query_invalid():
    response = client.post(
        "/api/search", json={"query": "rust", "mode": "video", "max_results": 5}
    )
    assert response.status_code == 400
    assert response.json()["code"] == "QUERY_INVALID"


def test_api_token_required_when_configured(monkeypatch):
    monkeypatch.setenv("NETRAIL_API_TOKEN", "test-secret-token")
    # Health remains open
    assert client.get("/api/health").status_code == 200
    denied = client.get("/api/backends")
    assert denied.status_code == 401
    assert denied.json()["code"] == "AUTH_REQUIRED"
    ok = client.get(
        "/api/backends", headers={"Authorization": "Bearer test-secret-token"}
    )
    assert ok.status_code == 200
    monkeypatch.delenv("NETRAIL_API_TOKEN", raising=False)


def test_rate_limit_search_returns_429():
    rate_limit.set_test_limits(search=2, open_limit=100, mutate=100)
    try:
        r1 = client.post("/api/search", json={"query": "a", "mode": "web", "max_results": 1})
        r2 = client.post("/api/search", json={"query": "b", "mode": "web", "max_results": 1})
        r3 = client.post("/api/search", json={"query": "c", "mode": "web", "max_results": 1})
        # First two may 200/502 depending on network; third must be limited.
        assert r3.status_code == 429
        assert r3.json()["code"] == "RATE_LIMITED"
        assert r1.status_code != 429 or r2.status_code != 429
    finally:
        rate_limit.set_test_limits()


def test_strict_backend_rejects_localhost(monkeypatch):
    from netrail.security import validate_backend_url
    from netrail.errors import NetRailError
    import pytest

    validate_backend_url("http://127.0.0.1:8080", strict=False)
    with pytest.raises(NetRailError) as exc:
        validate_backend_url("http://127.0.0.1:8080", strict=True)
    assert exc.value.code == "BACKEND_URL_STRICT_PRIVATE"


def test_malformed_bodies_return_typed_codes():
    missing = client.post("/api/search", content=b"{}", headers={"Content-Type": "application/json"})
    assert missing.status_code == 400
    assert missing.json()["code"] == "QUERY_INVALID"

    wrong_type = client.post(
        "/api/search",
        content=b'{"query": 123}',
        headers={"Content-Type": "application/json"},
    )
    assert wrong_type.status_code == 400
    assert wrong_type.json()["code"] == "QUERY_INVALID"

    bad_json = client.post("/api/search", content=b"{bad", headers={"Content-Type": "application/json"})
    assert bad_json.status_code == 400
    assert bad_json.json()["code"] == "REQUEST_INVALID"

    out_of_range = client.post(
        "/api/search",
        content=b'{"query": "rust", "max_results": 999}',
        headers={"Content-Type": "application/json"},
    )
    assert out_of_range.status_code == 400
    assert out_of_range.json()["code"] == "CONFIG_MAX_RESULTS"

    missing_url = client.post("/api/open", content=b"{}", headers={"Content-Type": "application/json"})
    assert missing_url.status_code == 400
    assert missing_url.json()["code"] == "OPEN_URL_INVALID"


def test_token_injection_csp_allows_script_hash(monkeypatch):
    monkeypatch.setenv("NETRAIL_API_TOKEN", "hash-test-token")
    response = client.get("/")
    csp = response.headers.get("Content-Security-Policy", "")
    assert "script-src 'self' 'sha256-" in csp, csp
    assert "NETRAIL_API_TOKEN=\"hash-test-token\"" in response.text
    assert "script-src 'self' 'unsafe-inline'" not in csp
    monkeypatch.delenv("NETRAIL_API_TOKEN", raising=False)
    response = client.get("/")
    assert response.headers.get("Content-Security-Policy") == (
        "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; "
        "img-src 'self' https: data:; connect-src 'self'; frame-ancestors 'none'; "
        "base-uri 'self'; form-action 'self'"
    )
    assert "NETRAIL_API_TOKEN" not in response.text
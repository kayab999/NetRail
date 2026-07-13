from fastapi.testclient import TestClient

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
    assert payload.get("api_contract") == "1.2"
    assert "rate_limit" in payload
    assert "enabled" in payload["rate_limit"]
    assert "search_per_minute" in payload["rate_limit"]
    assert "open_per_minute" in payload["rate_limit"]
    assert "search_recovery" in payload
    assert "backends_configured" in payload


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
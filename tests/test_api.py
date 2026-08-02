import ipaddress

from fastapi.testclient import TestClient

from netrail import API_CONTRACT, rate_limit
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
    assert payload.get("api_contract") == API_CONTRACT
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


def test_open_pins_hostname_resolution_before_spawn(monkeypatch):
    monkeypatch.setattr(
        "netrail.security.resolve_host_ips",
        lambda host: [ipaddress.ip_address("192.168.1.10")],
    )
    response = client.post("/api/open", json={"url": "http://evil.example/"})
    assert response.status_code == 400
    assert response.json()["code"] == "OPEN_URL_PRIVATE"


def test_open_fails_closed_when_hostname_unresolvable(monkeypatch):
    monkeypatch.setattr("netrail.security.resolve_host_ips", lambda host: [])
    response = client.post("/api/open", json={"url": "http://nxdomain.invalid/"})
    assert response.status_code == 400
    assert response.json()["code"] == "OPEN_URL_DNS_UNRESOLVABLE"


def test_readonly_mode_rejects_mutations(monkeypatch, tmp_path):
    monkeypatch.setenv("HOME", str(tmp_path))
    monkeypatch.setenv("NETRAIL_READONLY", "1")
    body = _valid_settings_body()

    r = client.put("/api/settings", json=body)
    assert r.status_code == 403
    assert r.json()["code"] == "READONLY_MODE"

    r = client.post("/api/collections", json={"name": "Research"})
    assert r.status_code == 403
    assert r.json()["code"] == "READONLY_MODE"

    r = client.post(
        "/api/collections/1/items",
        json={"url": "https://example.com/", "title": "A"},
    )
    assert r.status_code == 403
    assert r.json()["code"] == "READONLY_MODE"

    r = client.delete("/api/history/1")
    assert r.status_code == 403
    assert r.json()["code"] == "READONLY_MODE"

    r = client.delete("/api/history")
    assert r.status_code == 403
    assert r.json()["code"] == "READONLY_MODE"


def test_readonly_mode_keeps_read_endpoints(monkeypatch, tmp_path):
    monkeypatch.setenv("HOME", str(tmp_path))
    monkeypatch.setenv("NETRAIL_READONLY", "1")
    assert client.get("/api/health").status_code == 200
    assert client.get("/api/settings").status_code == 200
    assert client.get("/api/history").status_code == 200
    assert client.get("/api/collections").status_code == 200
    assert client.get("/api/docs/manual").status_code == 200


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
        "img-src 'self' https: data:; connect-src 'self'; upgrade-insecure-requests; "
        "frame-ancestors 'none'; base-uri 'self'; form-action 'self'"
    )
    assert "NETRAIL_API_TOKEN" not in response.text


def _valid_settings_body() -> dict:
    return {
        "search_strategy": "fanout",
        "backend_order": ["ddgs"],
        "ddgs_enabled": True,
        "searxng_url": None,
        "brave_enabled": False,
        "history_enabled": True,
        "history_encrypt": False,
        "history_ttl_days": 90,
        "max_results": 25,
    }


def test_settings_etag_roundtrip_and_conflict(monkeypatch, tmp_path):
    monkeypatch.setenv("HOME", str(tmp_path))
    r = client.get("/api/settings")
    assert r.status_code == 200
    etag = r.headers.get("etag")
    assert etag and etag.startswith('"')

    stale = client.put(
        "/api/settings",
        json=_valid_settings_body(),
        headers={"If-Match": '"stale-etag"'},
    )
    assert stale.status_code == 409
    assert stale.json()["code"] == "SETTINGS_CONFLICT"

    fresh = client.put(
        "/api/settings",
        json=_valid_settings_body(),
        headers={"If-Match": etag},
    )
    assert fresh.status_code == 200
    assert fresh.headers.get("etag") and fresh.headers["etag"] != etag

    plain = client.put("/api/settings", json=_valid_settings_body())
    assert plain.status_code == 200


def test_rate_limit_buckets_are_per_identity():
    import pytest
    from netrail.errors import NetRailError

    rate_limit.set_test_limits(search=1, open_limit=100, mutate=100)
    try:
        rate_limit._try_acquire("search", "alice")
        with pytest.raises(NetRailError) as exc:
            rate_limit._try_acquire("search", "alice")
        assert exc.value.code == "RATE_LIMITED"
        rate_limit._try_acquire("search", "bob")
    finally:
        rate_limit.set_test_limits()


def test_client_identity_is_token_hash_or_anonymous(monkeypatch):
    from netrail.auth import client_identity

    monkeypatch.setenv("NETRAIL_API_TOKEN", "secret")
    assert client_identity(None, None) == "anonymous"
    a = client_identity("Bearer secret", None)
    b = client_identity("Bearer other", None)
    assert a != b
    assert a.startswith("token:") and b.startswith("token:")
    assert client_identity("Bearer secret", None) == a
    assert client_identity(None, "secret") == a
    monkeypatch.delenv("NETRAIL_API_TOKEN", raising=False)
    assert client_identity("Bearer secret", None) == "anonymous"


def test_audit_log_rotates_at_max_bytes(monkeypatch, tmp_path):
    import netrail.audit as audit

    audit.reset_for_tests()
    monkeypatch.setenv("NETRAIL_AUDIT_LOG_PATH", str(tmp_path / "audit.log"))
    monkeypatch.setenv("NETRAIL_AUDIT_MAX_BYTES", "512")
    monkeypatch.setenv("NETRAIL_AUDIT_MAX_FILES", "2")
    for i in range(10):
        audit.log_event("test.action", {"i": i})
    assert (tmp_path / "audit.log.1").exists()
    assert (tmp_path / "audit.log").stat().st_size < 512
    audit.reset_for_tests()


def test_rate_limit_add_collection_item_returns_429(monkeypatch, tmp_path):
    from netrail.history.store import reset_store_for_tests

    monkeypatch.setenv("HOME", str(tmp_path))
    monkeypatch.setenv("NETRAIL_DB_PATH", str(tmp_path / "test.db"))
    reset_store_for_tests()
    rate_limit.set_test_limits(search=100, open_limit=100, mutate=1)
    try:
        col_res = client.post("/api/collections", json={"name": "AuditCol"})
        assert col_res.status_code == 200
        col_id = col_res.json()["id"]
        # Second mutate call (add item) must trigger 429
        res = client.post(
            f"/api/collections/{col_id}/items",
            json={"url": "https://example.com/item1", "title": "Item 1"},
        )
        assert res.status_code == 429
        assert res.json()["code"] == "RATE_LIMITED"
    finally:
        rate_limit.set_test_limits()


def test_audit_log_collection_item_add(monkeypatch, tmp_path):
    import json
    import netrail.audit as audit
    from netrail.history.store import reset_store_for_tests

    monkeypatch.setenv("HOME", str(tmp_path))
    monkeypatch.setenv("NETRAIL_DB_PATH", str(tmp_path / "test.db"))
    log_file = tmp_path / "audit.jsonl"
    monkeypatch.setenv("NETRAIL_AUDIT_LOG_PATH", str(log_file))
    audit.reset_for_tests()
    reset_store_for_tests()

    col_res = client.post("/api/collections", json={"name": "AuditCol"})
    assert col_res.status_code == 200
    col_id = col_res.json()["id"]
    item = client.post(
        f"/api/collections/{col_id}/items",
        json={"url": "https://example.com/test", "title": "Test Title"},
    )
    assert item.status_code == 200

    lines = log_file.read_text(encoding="utf-8").strip().splitlines()
    events = [json.loads(line) for line in lines]
    add_events = [e for e in events if e.get("action") == "collection.item.add"]
    assert len(add_events) == 1
    assert add_events[0]["detail"]["collection_id"] == col_id
    assert add_events[0]["detail"]["url_host"] == "example.com"
    audit.reset_for_tests()


def test_save_settings_atomic(monkeypatch, tmp_path):
    from netrail.config import save_settings, load_settings, config_file, config_dir

    monkeypatch.setenv("HOME", str(tmp_path))
    saved = save_settings({"max_results": 42})
    assert saved["max_results"] == 42
    assert config_file().is_file()
    assert load_settings()["max_results"] == 42
    leftovers = [
        p
        for p in config_dir().iterdir()
        if p.name != "settings.json" and "settings.json" in p.name and p.suffix == ".tmp"
    ]
    assert leftovers == []


def test_save_settings_concurrent_no_race(monkeypatch, tmp_path):
    """NR-08: unique temps must not FileNotFoundError under concurrent saves."""
    import threading

    from netrail.config import save_settings, load_settings, config_file

    monkeypatch.setenv("HOME", str(tmp_path))
    barrier = threading.Barrier(16)
    errors: list[str] = []
    results: list[int] = []

    def worker(n: int) -> None:
        try:
            barrier.wait()
            saved = save_settings({"max_results": 1 + (n % 50)})
            results.append(int(saved["max_results"]))
        except Exception as exc:  # noqa: BLE001
            errors.append(repr(exc))

    threads = [threading.Thread(target=worker, args=(i,)) for i in range(16)]
    for t in threads:
        t.start()
    for t in threads:
        t.join()
    assert errors == [], f"concurrent save errors: {errors[:5]}"
    assert len(results) == 16
    assert config_file().is_file()
    assert 1 <= load_settings()["max_results"] <= 50

from __future__ import annotations

import time

from fastapi.testclient import TestClient

import netrail.main as main_module
from netrail.main import app


def _client():
    return TestClient(app, raise_server_exceptions=False)


def test_slow_handler_times_out_with_typed_408(monkeypatch):
    """A handler slower than the budget must yield REQUEST_TIMEOUT 408."""
    monkeypatch.setenv("NETRAIL_REQUEST_TIMEOUT_SECS", "0.2")

    def _slow_search(*args, **kwargs):
        time.sleep(2.0)
        return {"results": [], "errors": []}

    monkeypatch.setattr(main_module, "search", _slow_search)
    with _client() as client:
        response = client.post("/api/search", json={"query": "hello"})
    assert response.status_code == 408, response.text
    payload = response.json()
    assert payload["code"] == "REQUEST_TIMEOUT"
    assert payload["status"] == 408
    assert "detail" in payload


def test_health_not_affected_by_timeout(monkeypatch):
    monkeypatch.setenv("NETRAIL_REQUEST_TIMEOUT_SECS", "5")
    with _client() as client:
        response = client.get("/api/health")
    assert response.status_code == 200


def test_timeout_zero_disables_middleware(monkeypatch):
    """0 disables the timeout: even a slow handler completes normally."""
    monkeypatch.setenv("NETRAIL_REQUEST_TIMEOUT_SECS", "0")

    def _slow_search(*args, **kwargs):
        time.sleep(0.3)
        return {"results": [], "errors": []}

    monkeypatch.setattr(main_module, "search", _slow_search)
    with _client() as client:
        response = client.post("/api/search", json={"query": "hello"})
    # Slow handler completes; search wrapper returns 200 (no fanout involved).
    assert response.status_code == 200, response.text


def test_invalid_timeout_env_falls_back_to_default(monkeypatch):
    monkeypatch.setenv("NETRAIL_REQUEST_TIMEOUT_SECS", "not-a-number")
    assert main_module._request_timeout_secs() == main_module.DEFAULT_REQUEST_TIMEOUT_SECS

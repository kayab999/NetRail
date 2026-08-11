"""A-11: settings directivity gold — PUT /api/settings must rebind the history
store immediately (Rust SharedStore::ensure semantics: effective mode follows
the latest (history_enabled, history_encrypt) at every access).

Consumes the shared fixture tests/fixtures/settings_transitions.json (also
consumed by the Rust SharedStore unit gold).
"""

import json
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

from netrail.main import app

FIXTURE = json.loads(
    (Path(__file__).parent / "fixtures" / "settings_transitions.json").read_text()
)

BASE = {
    "search_strategy": "fanout",
    "backend_order": ["ddgs"],
    "ddgs_enabled": True,
    "searxng_url": None,
    "brave_enabled": False,
    "private_mode": False,
    "history_enabled": True,
    "history_encrypt": True,
    "history_ttl_days": 90,
    "max_results": 25,
}


def _settings(enabled, encrypt) -> dict:
    body = dict(BASE)
    body["history_enabled"] = enabled
    body["history_encrypt"] = encrypt
    return body


def _health_state(client) -> dict:
    payload = client.get("/api/health").json()["history"]
    if payload.get("enabled") is False:
        return {"enabled": False}
    return {
        "enabled": True,
        "encrypt_requested": payload["encrypt_requested"],
        "encryption_state": payload["encryption_state"],
    }


@pytest.fixture(autouse=True)
def _isolated_home_and_key(monkeypatch, tmp_path):
    from cryptography.fernet import Fernet

    monkeypatch.setenv("NETRAIL_DB_KEY", Fernet.generate_key().decode())
    monkeypatch.delenv("NETRAIL_HISTORY_ENCRYPT", raising=False)
    monkeypatch.setenv("HOME", str(tmp_path / "home"))


@pytest.mark.parametrize(
    "vector", FIXTURE["settings_transitions"], ids=lambda v: v["id"]
)
def test_settings_transitions_rebind_store(vector):
    from netrail.history import store as history_store

    client = TestClient(app)
    frm = vector["from"]
    put = vector["put"]
    expect = vector["expect"]

    first = client.put(
        "/api/settings", json=_settings(frm["history_enabled"], frm["history_encrypt"])
    )
    assert first.status_code == 200, first.text

    # The transition under test: PUT the new mode and assert the store rebinds.
    resp = client.put(
        "/api/settings", json=_settings(put["history_enabled"], put["history_encrypt"])
    )
    assert resp.status_code == 200, resp.text

    state = _health_state(client)
    for key, value in expect.items():
        assert state.get(key) == value, (vector["id"], key, state)

    # Directivity observable: the singleton must be rebound — a stale store
    # built for the 'from' mode must not survive the transition.
    got = history_store.get_store()
    if not put["history_enabled"]:
        assert got is None, vector["id"]
    else:
        mode = (put["history_enabled"], put["history_encrypt"])
        assert history_store._store_mode == mode, (
            vector["id"],
            history_store._store_mode,
            mode,
        )
"""A-06: canonical cipher-state model (ENCRYPTED/DEGRADED/PLAINTEXT).

The mapping is derived from the existing /api/health flags and pinned by the
shared fixture tests/fixtures/cipher_state.json (also consumed by the Rust
`cipher_state_golden_fixture` unit test).
"""

import json
from pathlib import Path

import pytest

from netrail.history.crypto import cipher_state, reset_for_tests

FIXTURE = json.loads(
    (Path(__file__).parent / "fixtures" / "cipher_state.json").read_text()
)


@pytest.mark.parametrize("vector", FIXTURE["cipher_state"], ids=lambda v: v["id"])
def test_cipher_state_golden(vector):
    got = cipher_state(
        vector["encrypt_requested"], vector["encryption_active"]
    )
    assert got == vector["expect"], vector["id"]


def test_health_encrypted_with_key(monkeypatch):
    from cryptography.fernet import Fernet

    reset_for_tests()
    monkeypatch.setenv("NETRAIL_DB_KEY", Fernet.generate_key().decode())
    monkeypatch.setenv("NETRAIL_HISTORY_ENCRYPT", "true")
    from fastapi.testclient import TestClient

    from netrail.main import app

    payload = TestClient(app).get("/api/health").json()
    history = payload["history"]
    assert history["encrypt_requested"] is True
    assert history["encryption_active"] is True
    assert history["encryption_state"] == "encrypted"


def test_health_degraded_without_key(monkeypatch):
    import sys

    reset_for_tests()
    monkeypatch.delenv("NETRAIL_DB_KEY", raising=False)
    monkeypatch.setenv("NETRAIL_HISTORY_ENCRYPT", "true")

    class _NoKeyring:
        def get_password(self, *a, **k):
            raise RuntimeError("no keyring (headless)")

        def set_password(self, *a, **k):
            raise RuntimeError("no keyring (headless)")

    monkeypatch.setitem(sys.modules, "keyring", _NoKeyring())
    from fastapi.testclient import TestClient

    from netrail.main import app

    payload = TestClient(app).get("/api/health").json()
    history = payload["history"]
    assert history["encrypt_requested"] is True
    assert history["encryption_active"] is False
    assert history["encryption_state"] == "degraded"


def test_health_plaintext_when_encryption_not_requested(monkeypatch):
    monkeypatch.setenv("NETRAIL_HISTORY_ENCRYPT", "false")
    from fastapi.testclient import TestClient

    from netrail.main import app

    payload = TestClient(app).get("/api/health").json()
    history = payload["history"]
    assert history["encrypt_requested"] is False
    assert history["encryption_state"] == "plaintext"
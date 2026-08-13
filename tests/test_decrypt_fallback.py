"""T2: undecryptable Fernet blobs surface a marker, not base64 garbage.

A Fernet token is base64 and always starts with the version byte 0x80,
which encodes to the literal prefix "gAAAAA". Legacy plaintext rows from a
pre-encryption era lack that prefix and are passed through untouched.
"""

import pytest
from cryptography.fernet import Fernet

from netrail.history.crypto import (
    DECRYPTION_FAILED_MARKER,
    decrypt_text,
    encrypt_text,
    reset_for_tests,
)


@pytest.fixture(autouse=True)
def _reset(capsys):
    yield
    reset_for_tests()


def _set_key(monkeypatch):
    reset_for_tests()
    monkeypatch.setenv("NETRAIL_DB_KEY", Fernet.generate_key().decode())


def test_encrypted_blob_roundtrips_with_matching_key(monkeypatch):
    _set_key(monkeypatch)
    blob = encrypt_text("battery regulations EU")
    assert decrypt_text(blob) == "battery regulations EU"


def test_wrong_key_surfaces_marker_not_garbage(monkeypatch):
    _set_key(monkeypatch)
    blob = encrypt_text("battery regulations EU")
    _set_key(monkeypatch)  # rotate the key; cache must not pin the old one
    assert decrypt_text(blob) == DECRYPTION_FAILED_MARKER


def test_corrupt_token_surfaces_marker(monkeypatch):
    _set_key(monkeypatch)
    token = b"gAAAAA" + b"not-a-valid-token"
    assert decrypt_text(token) == DECRYPTION_FAILED_MARKER


def test_missing_key_surfaces_marker(monkeypatch):
    import sys

    class _NoKeyring:
        def get_password(self, *a, **k):
            raise RuntimeError("no keyring (headless)")

    _set_key(monkeypatch)
    blob = encrypt_text("battery regulations EU")
    reset_for_tests()
    monkeypatch.delenv("NETRAIL_DB_KEY", raising=False)
    monkeypatch.setitem(sys.modules, "keyring", _NoKeyring())
    assert decrypt_text(blob) == DECRYPTION_FAILED_MARKER


def test_force_plain_refuses_encrypted_blob(monkeypatch):
    _set_key(monkeypatch)
    blob = encrypt_text("battery regulations EU")
    assert decrypt_text(blob, force_plain=True) == DECRYPTION_FAILED_MARKER
    assert decrypt_text(blob) == "battery regulations EU"


def test_legacy_plaintext_passes_through(monkeypatch):
    _set_key(monkeypatch)
    plain = b"pre-encryption era row"
    assert decrypt_text(plain) == "pre-encryption era row"


def test_empty_and_none(monkeypatch):
    _set_key(monkeypatch)
    assert decrypt_text(b"") == ""
    assert decrypt_text(None) == ""
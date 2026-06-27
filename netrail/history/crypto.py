from __future__ import annotations

import os
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from cryptography.fernet import Fernet

_fernet: "Fernet | None" = None
_encryption_enabled = True


def _load_fernet() -> "Fernet | None":
    global _fernet, _encryption_enabled
    if _fernet is not None:
        return _fernet

    from cryptography.fernet import Fernet

    key_material: bytes | None = None

    env_key = os.environ.get("NETRAIL_DB_KEY")
    if env_key:
        key_material = env_key.encode("utf-8")

    if key_material is None:
        try:
            import keyring

            stored = keyring.get_password("netrail", "db-key")
            if stored:
                key_material = stored.encode("utf-8")
        except Exception:  # noqa: BLE001 — keyring optional on headless
            pass

    if key_material is None:
        _encryption_enabled = False
        return None

    _fernet = Fernet(key_material)
    return _fernet


def ensure_encryption_key() -> bool:
    """Create and store a Fernet key when encryption is requested."""
    global _fernet, _encryption_enabled

    from cryptography.fernet import Fernet

    if os.environ.get("NETRAIL_DB_KEY"):
        _fernet = Fernet(os.environ["NETRAIL_DB_KEY"].encode("utf-8"))
        _encryption_enabled = True
        return True

    try:
        import keyring

        stored = keyring.get_password("netrail", "db-key")
        if stored:
            _fernet = Fernet(stored.encode("utf-8"))
            _encryption_enabled = True
            return True

        key = Fernet.generate_key()
        keyring.set_password("netrail", "db-key", key.decode("utf-8"))
        _fernet = Fernet(key)
        _encryption_enabled = True
        return True
    except Exception:  # noqa: BLE001
        _encryption_enabled = False
        _fernet = None
        return False


def encryption_active() -> bool:
    return _load_fernet() is not None


def encrypt_text(value: str, *, force_plain: bool = False) -> bytes:
    if force_plain or not value:
        return value.encode("utf-8")

    fernet = _load_fernet()
    if fernet is None:
        return value.encode("utf-8")
    return fernet.encrypt(value.encode("utf-8"))


def decrypt_text(blob: bytes | None, *, force_plain: bool = False) -> str:
    if blob is None:
        return ""
    if force_plain:
        return blob.decode("utf-8")

    fernet = _load_fernet()
    if fernet is None:
        return blob.decode("utf-8")
    try:
        return fernet.decrypt(blob).decode("utf-8")
    except Exception:  # noqa: BLE001 — legacy plaintext rows
        return blob.decode("utf-8")
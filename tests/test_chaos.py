import json
import os
import sqlite3
import stat

from fastapi.testclient import TestClient

from netrail import audit
from netrail.history.store import reset_store_for_tests
from netrail.main import app

client = TestClient(app)


def _set_db_env(monkeypatch, tmp_path, name="netrail.db", subdir="data"):
    base = tmp_path / subdir
    base.mkdir(parents=True, exist_ok=True)
    monkeypatch.setenv("HOME", str(tmp_path))
    monkeypatch.setenv("NETRAIL_DB_PATH", str(base / name))
    reset_store_for_tests()


def _is_root() -> bool:
    return os.geteuid() == 0


def test_sqlite_write_lock_typed_error_then_recovery(monkeypatch, tmp_path):
    _set_db_env(monkeypatch, tmp_path)
    db_path = tmp_path / "data" / "netrail.db"

    assert client.post("/api/collections", json={"name": "seed"}).status_code == 200

    raw = sqlite3.connect(str(db_path))
    try:
        raw.execute("BEGIN IMMEDIATE")
        raw.execute("INSERT INTO collections (name) VALUES ('lock-holder')")

        assert client.get("/api/history").status_code == 200

        response = client.post("/api/collections", json={"name": "blocked"})
        assert response.status_code == 500
        body = response.json()
        assert body["code"] == "DB_ERROR"
        assert body["status"] == 500
        assert isinstance(body["detail"], str)

        raw.rollback()
    finally:
        raw.close()

    response = client.post("/api/collections", json={"name": "blocked"})
    assert response.status_code == 200
    assert response.json()["name"] == "blocked"

    reset_store_for_tests()


def test_unwritable_db_dir_degrades_then_recovers(monkeypatch, tmp_path):
    if _is_root():
        return

    _set_db_env(monkeypatch, tmp_path, subdir="db")
    db_dir = tmp_path / "db"
    os.chmod(db_dir, stat.S_IRUSR | stat.S_IXUSR)

    response = client.post("/api/collections", json={"name": "ro"})
    assert response.status_code == 400
    body = response.json()
    assert body["code"] == "HISTORY_DISABLED"
    assert body["status"] == 400

    assert client.get("/api/health").status_code == 200

    os.chmod(db_dir, stat.S_IRUSR | stat.S_IWUSR | stat.S_IXUSR)
    response = client.post("/api/collections", json={"name": "back"})
    assert response.status_code == 200
    assert response.json()["name"] == "back"

    reset_store_for_tests()


def test_audit_external_rotation_no_entry_loss(monkeypatch, tmp_path):
    monkeypatch.setenv("NETRAIL_AUDIT_LOG_PATH", str(tmp_path / "audit.log"))
    monkeypatch.setenv("NETRAIL_AUDIT_MAX_BYTES", str(1024 * 1024 * 1024))
    monkeypatch.setenv("NETRAIL_AUDIT_MAX_FILES", "1")
    audit.reset_for_tests()

    for i in range(3):
        audit.log_event("pre.rotation", {"i": i})

    rotated = tmp_path / "audit.log.rotated"
    (tmp_path / "audit.log").rename(rotated)

    for i in range(3):
        audit.log_event("post.rotation", {"i": i})

    def count_lines(path):
        with path.open(encoding="utf-8") as handle:
            return [json.loads(line) for line in handle if line.strip()]

    pre = count_lines(rotated)
    post = count_lines(tmp_path / "audit.log")
    assert len(pre) == 3
    assert len(post) == 3
    assert {e["action"] for e in pre} == {"pre.rotation"}
    assert {e["action"] for e in post} == {"post.rotation"}
    assert len(pre) + len(post) == 6

    audit.reset_for_tests()

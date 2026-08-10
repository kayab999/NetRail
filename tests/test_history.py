import pytest
from concurrent.futures import ThreadPoolExecutor
from cryptography.fernet import Fernet

from netrail.backends.types import SearchResult
from netrail.history.db import SCHEMA_VERSION, connect, db_path
from netrail.history.store import HistoryStore


@pytest.fixture
def temp_store(tmp_path, monkeypatch):
    db_path = tmp_path / "test.db"
    monkeypatch.setenv("NETRAIL_DB_KEY", Fernet.generate_key().decode())
    monkeypatch.setenv("NETRAIL_DB_PATH", str(db_path))
    from netrail.history import crypto
    import netrail.history.store as store_mod

    crypto._fernet = None
    crypto._encryption_enabled = True
    store_mod._store = None
    crypto.ensure_encryption_key()

    conn = connect()
    store = HistoryStore(conn, encrypt=True)
    yield store
    conn.close()


def test_record_search_and_history(temp_store):
    results = [
        SearchResult(title="A", url="https://example.com/a", snippet="sa", backend="ddgs"),
        SearchResult(title="B", url="https://example.com/b", snippet="sb", backend="ddgs"),
    ]
    query_id, url_map = temp_store.record_search("python tutorial", "web", ["ddgs"], results)
    assert query_id > 0
    assert len(url_map) == 2

    listed = temp_store.list_history()
    assert listed["items"][0]["query"] == "python tutorial"
    assert listed["items"][0]["result_count"] == 2


def test_visit_metadata_and_revisit(temp_store):
    results = [SearchResult(title="A", url="https://example.com/page", backend="ddgs")]
    _, url_map = temp_store.record_search("test", "web", ["ddgs"], results)
    result_id = url_map["https://example.com/page"]

    temp_store.record_visit("https://example.com/page", result_id=result_id, browser_id="firefox")
    meta = temp_store.get_visit_metadata(["https://example.com/page"])
    assert meta["https://example.com/page"]["visit_count"] == 1


def test_fts_history_search(temp_store):
    temp_store.record_search("battery regulations EU", "web", ["ddgs"], [])
    temp_store.record_search("cat pictures", "images", ["ddgs"], [])
    hits = temp_store.list_history(q='"battery"')
    assert len(hits["items"]) == 1
    assert "battery" in hits["items"][0]["query"]


def _counts(temp_store):
    queries = temp_store._conn.execute("SELECT COUNT(*) FROM queries").fetchone()[0]
    fts = temp_store._conn.execute("SELECT COUNT(*) FROM queries_fts").fetchone()[0]
    orphans = temp_store._conn.execute(
        "SELECT COUNT(*) FROM queries_fts f LEFT JOIN queries q ON q.id = f.rowid WHERE q.id IS NULL"
    ).fetchone()[0]
    return queries, fts, orphans


def test_fts_stays_synced_through_lifecycle(temp_store):
    assert _counts(temp_store) == (0, 0, 0)
    for q in ["battery regulations EU", "cat pictures", "python tutorial"]:
        temp_store.record_search(q, "web", ["ddgs"], [])
    assert _counts(temp_store) == (3, 3, 0)

    first_id = temp_store.list_history()["items"][0]["id"]
    assert temp_store.delete_history_entry(first_id)
    assert _counts(temp_store) == (2, 2, 0)

    assert temp_store.purge_all_history() == 2
    assert _counts(temp_store) == (0, 0, 0)


def test_purge_expired_keeps_fts_synced(temp_store):
    temp_store.record_search("fresh query", "web", ["ddgs"], [])
    temp_store._conn.execute(
        "INSERT INTO queries (query_text_enc, mode, backends_used, timestamp) "
        "VALUES (?, 'web', '[]', datetime('now', '-2000 days'))",
        (temp_store._enc("old query"),),
    )
    temp_store._rebuild_fts()
    temp_store._conn.commit()
    assert _counts(temp_store) == (2, 2, 0)
    assert temp_store.purge_expired(1000) == 1
    assert _counts(temp_store) == (1, 1, 0)


def test_fts_still_searches_after_rebuild(temp_store):
    temp_store.record_search("battery regulations EU", "web", ["ddgs"], [])
    temp_store.record_search("cat pictures", "images", ["ddgs"], [])
    items = temp_store.list_history()["items"]
    cat_id = next(i["id"] for i in items if i["query"] == "cat pictures")
    assert temp_store.delete_history_entry(cat_id)
    hits = temp_store.list_history(q='"battery"')
    assert len(hits["items"]) == 1


def test_collections_export(temp_store):
    collection = temp_store.create_collection("Research")
    temp_store.add_collection_item(collection["id"], url="https://a.test", title="Alpha", notes="note")
    exported = temp_store.export_collection(collection["id"], fmt="json")
    assert "Research" in exported
    assert "https://a.test" in exported

    csv_data = temp_store.export_collection(collection["id"], fmt="csv")
    assert "url,title" in csv_data


def test_purge_expired(temp_store):
    temp_store.record_search("old query", "web", ["ddgs"], [])
    purged = temp_store.purge_expired(0)
    assert purged >= 0


def test_connect_enables_wal_and_stamps_schema_version(tmp_path, monkeypatch):
    monkeypatch.setenv("NETRAIL_DB_PATH", str(tmp_path / "n.db"))
    conn = connect()
    try:
        assert conn.execute("PRAGMA journal_mode").fetchone()[0] == "wal"
        assert conn.execute("PRAGMA user_version").fetchone()[0] == SCHEMA_VERSION
    finally:
        conn.close()


def test_db_path_resolves_home_per_call(monkeypatch):
    """QA-16: db_path() must not freeze $HOME at import (config.py/browsers.py
    convention); changing $HOME between calls changes the default path."""
    monkeypatch.delenv("NETRAIL_DB_PATH", raising=False)
    monkeypatch.setenv("HOME", "/tmp/qa16-home-a")
    first = db_path()
    monkeypatch.setenv("HOME", "/tmp/qa16-home-b")
    second = db_path()
    assert str(first).startswith("/tmp/qa16-home-a")
    assert str(second).startswith("/tmp/qa16-home-b")


def test_concurrent_store_access_is_serialized(temp_store):
    """Sprint 3: the shared sqlite3 connection (check_same_thread=False) must
    never be used from two threads at once — before the lock, concurrent
    `stats()` returned None from fetchone (TypeError) under load."""
    results = [SearchResult(title="A", url="https://example.com/a", backend="ddgs")]
    errors: list[Exception] = []

    def hammer(i: int) -> None:
        try:
            for _ in range(100):
                temp_store.stats()
                temp_store.list_history()
                temp_store.record_search(f"concurrent {i}", "web", ["ddgs"], results)
        except Exception as exc:  # pragma: no cover - regression case
            errors.append(exc)

    with ThreadPoolExecutor(max_workers=16) as pool:
        list(pool.map(hammer, range(16)))

    assert errors == []
    stats = temp_store.stats()
    assert stats["queries"] == 1600
    assert stats["visits"] == 0
    assert stats["collections"] == 0
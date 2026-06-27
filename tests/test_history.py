import pytest
from cryptography.fernet import Fernet

from netrail.backends.types import SearchResult
from netrail.history.db import connect
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
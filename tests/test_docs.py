from fastapi.testclient import TestClient

from netrail.docs_content import load_doc
from netrail.main import app


def test_load_doc_manual():
    doc = load_doc("manual")
    assert doc["title"] == "User Manual"
    assert "NetRail" in doc["markdown"]


def test_load_doc_about():
    doc = load_doc("about")
    assert doc["title"] == "About NetRail"
    assert "Search first" in doc["markdown"]


def test_load_doc_missing_file_is_doc_not_found(monkeypatch, tmp_path):
    from netrail.errors import NetRailError
    import netrail.docs_content as docs_content

    monkeypatch.setattr(docs_content, "PROJECT_ROOT", tmp_path)
    try:
        docs_content.load_doc("manual")
        raise AssertionError("expected DOC_NOT_FOUND")
    except NetRailError as exc:
        assert exc.code == "DOC_NOT_FOUND"
        assert exc.status == 404


def test_docs_api_routes():
    client = TestClient(app)
    manual = client.get("/api/docs/manual")
    assert manual.status_code == 200
    assert manual.json()["slug"] == "manual"

    about = client.get("/api/docs/about")
    assert about.status_code == 200
    assert about.json()["title"] == "About NetRail"

    missing = client.get("/api/docs/unknown")
    assert missing.status_code == 404
    assert missing.json()["code"] == "DOC_NOT_FOUND"

    asset = client.get("/api/docs/assets/netrail-demo.png")
    assert asset.status_code == 200
    assert asset.headers["content-type"].startswith("image/")
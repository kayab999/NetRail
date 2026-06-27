from fastapi.testclient import TestClient

from netrail.main import app

client = TestClient(app)


def test_health_reports_provenance():
    response = client.get("/api/health")
    assert response.status_code == 200
    payload = response.json()
    assert payload["telemetry"] == "none"
    assert "Bing" in payload["default_provenance"]


def test_backends_endpoint():
    response = client.get("/api/backends")
    assert response.status_code == 200
    backends = response.json()
    assert any(item["name"] == "ddgs" for item in backends)


def test_open_rejects_localhost():
    response = client.post("/api/open", json={"url": "http://127.0.0.1:8080"})
    assert response.status_code == 400


def test_csp_header_on_index():
    response = client.get("/")
    assert "Content-Security-Policy" in response.headers
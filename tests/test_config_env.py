import pytest

from netrail.config import load_settings


def test_searxng_url_from_env(monkeypatch):
    monkeypatch.delenv("NETRAIL_SEARXNG_URL", raising=False)
    monkeypatch.setenv("SEARXNG_URL", "http://searxng:8080")
    settings = load_settings()
    assert settings["searxng_url"] == "http://searxng:8080"
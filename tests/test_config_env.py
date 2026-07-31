import pytest

from netrail.config import load_settings


def test_searxng_url_from_env(monkeypatch):
    monkeypatch.delenv("NETRAIL_SEARXNG_URL", raising=False)
    monkeypatch.setenv("SEARXNG_URL", "http://searxng:8080")
    settings = load_settings()
    assert settings["searxng_url"] == "http://searxng:8080"


def test_invalid_searxng_env_is_ignored(monkeypatch):
    """Hostile/misconfigured env must not bypass backend URL policy (SEC-2026-02)."""
    monkeypatch.delenv("SEARXNG_URL", raising=False)
    monkeypatch.setenv("NETRAIL_SEARXNG_URL", "http://169.254.169.254/latest/meta-data/")
    settings = load_settings()
    assert settings["searxng_url"] != "http://169.254.169.254/latest/meta-data/"
    assert settings["searxng_url"] in (None, "") or not str(settings["searxng_url"]).startswith(
        "http://169.254.169.254"
    )


def test_rebinding_searxng_env_is_ignored(monkeypatch):
    monkeypatch.delenv("SEARXNG_URL", raising=False)
    monkeypatch.setenv("NETRAIL_SEARXNG_URL", "http://localtest.me/")
    settings = load_settings()
    assert settings["searxng_url"] != "http://localtest.me/"
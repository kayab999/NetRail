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


def test_empty_searxng_env_does_not_fall_through(monkeypatch):
    """QA-13 parity: Rust skips an empty NETRAIL_SEARXNG_URL entirely
    (or_else + is_empty); Python must not fall through to SEARXNG_URL."""
    monkeypatch.setenv("NETRAIL_SEARXNG_URL", "")
    monkeypatch.setenv("SEARXNG_URL", "http://searxng:8080")
    settings = load_settings()
    assert settings["searxng_url"] != "http://searxng:8080"


def test_invalid_searxng_env_logs_warning(monkeypatch, caplog):
    """QA-13 parity: operator must see that the env override was rejected."""
    monkeypatch.delenv("SEARXNG_URL", raising=False)
    monkeypatch.setenv("NETRAIL_SEARXNG_URL", "http://169.254.169.254/latest/meta-data/")
    with caplog.at_level("WARNING", logger="netrail.config"):
        load_settings()
    assert any("NETRAIL_SEARXNG_URL" in r.message for r in caplog.records)


def test_brave_key_forces_enabled_over_env_false(monkeypatch):
    """QA-15 parity with config.rs: BRAVE_SEARCH_API_KEY presence force-enables
    Brave even when NETRAIL_BRAVE_ENABLED=false (Rust-primary semantic)."""
    monkeypatch.setenv("BRAVE_SEARCH_API_KEY", "test-key")
    monkeypatch.setenv("NETRAIL_BRAVE_ENABLED", "false")
    settings = load_settings()
    assert settings["brave_enabled"] is True
    brave = next(b for b in settings["backends"] if b.get("id") == "brave")
    assert brave["enabled"] is True
    assert "brave" in settings["backend_order"]
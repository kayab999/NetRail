from __future__ import annotations

import pytest

from netrail import auth


def test_gate_missing_exits_1_with_setup_message(monkeypatch, capsys):
    monkeypatch.delenv("NETRAIL_API_TOKEN", raising=False)
    with pytest.raises(SystemExit) as exc_info:
        auth.headless_token_gate()
    assert exc_info.value.code == 1
    err = capsys.readouterr().err
    assert "requires an API token" in err
    assert "openssl rand -hex 32" in err


def test_gate_explicit_empty_warns_but_runs(monkeypatch, capsys):
    for value in ("", "   "):
        monkeypatch.setenv("NETRAIL_API_TOKEN", value)
        assert auth.headless_token_gate() is None
        assert "explicitly empty" in capsys.readouterr().err


def test_gate_configured_runs(monkeypatch, capsys):
    monkeypatch.setenv("NETRAIL_API_TOKEN", "headless-secret")
    assert auth.headless_token_gate() is None
    assert capsys.readouterr().err == ""


def test_main_calls_gate_before_serving(monkeypatch):
    """`main()` must fail fast without reaching uvicorn when unset."""
    import netrail.main as main_module

    monkeypatch.delenv("NETRAIL_API_TOKEN", raising=False)
    called: list = []
    monkeypatch.setattr(
        "uvicorn.run", lambda *a, **k: called.append((a, k))
    )
    with pytest.raises(SystemExit) as exc_info:
        main_module.main()
    assert exc_info.value.code == 1
    assert called == []


def test_main_warns_but_serves_when_explicit_empty(monkeypatch):
    import netrail.main as main_module

    monkeypatch.setenv("NETRAIL_API_TOKEN", "")
    called: list = []
    monkeypatch.setattr(
        "uvicorn.run", lambda *a, **k: called.append((a, k))
    )
    monkeypatch.setattr(main_module, "_schedule_ui_open", lambda: None)
    main_module.main()
    assert len(called) == 1

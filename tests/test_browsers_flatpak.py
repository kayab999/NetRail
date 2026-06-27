from unittest.mock import MagicMock, patch

from netrail.browsers import _spawn_process, open_url


def test_spawn_uses_flatpak_host(monkeypatch):
    monkeypatch.setattr("netrail.browsers.is_flatpak", lambda: True)
    popen = MagicMock()
    with patch("netrail.browsers.subprocess.Popen", popen):
        _spawn_process(["firefox", "https://example.com"], {})
    popen.assert_called_once()
    cmd = popen.call_args[0][0]
    assert cmd[:2] == ["flatpak-spawn", "--host"]
    assert "firefox" in cmd


def test_open_url_reports_flatpak_sandbox(monkeypatch):
    monkeypatch.setattr("netrail.browsers.is_flatpak", lambda: True)
    fake_browser = MagicMock()
    fake_browser.name = "Firefox"
    fake_browser.executable = "/usr/bin/firefox"
    fake_browser.private_flag = "--private-window"
    with patch("netrail.browsers.find_browser", return_value=fake_browser):
        with patch("netrail.browsers._spawn_process"):
            result = open_url("https://example.com", private_mode=True)
    assert result["sandbox"] == "flatpak-host"
from __future__ import annotations

import json
from pathlib import Path

from netrail.browsers import KNOWN_BROWSERS, discover_browsers

FIXTURE = Path(__file__).parent / "fixtures" / "browsers.json"


def test_known_browsers_match_canonical_fixture():
    fixture = json.loads(FIXTURE.read_text())
    expected = {
        b["id"]: (b["name"], b["private_flag"])
        for b in fixture["known_browsers"]
    }
    assert KNOWN_BROWSERS == expected, (
        "KNOWN_BROWSERS drifted from tests/fixtures/browsers.json (QA-09 contract)"
    )


def test_unknown_browser_gets_no_fabricated_private_flag(tmp_path, monkeypatch):
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    fake = bin_dir / "unknown-browser-bin"
    fake.write_text("#!/bin/sh\n")
    fake.chmod(0o755)

    monkeypatch.setenv("HOME", str(tmp_path / "home"))
    (tmp_path / "home" / ".local" / "share" / "applications").mkdir(parents=True)
    (tmp_path / "home" / ".local" / "share" / "applications"
     / "unknown-browser.desktop").write_text(
        "[Desktop Entry]\n"
        "Type=Application\n"
        "Name=Unknown Co.\n"
        f"Exec={fake}\n"
        "Categories=Network;WebBrowser;\n"
    )
    monkeypatch.setenv("PATH", str(bin_dir) + ":" + str(Path("/usr/bin")))

    found = [b for b in discover_browsers() if b.id == "unknown-browser-bin"]
    assert len(found) == 1
    unknown = found[0]
    assert unknown.name == "Unknown Co."
    assert unknown.private_flag is None, (
        "unknown binaries must never receive a fabricated private flag"
    )


def test_localized_name_variant_does_not_override_entry_name(tmp_path, monkeypatch):
    apps = tmp_path / "home" / ".local" / "share" / "applications"
    apps.mkdir(parents=True)
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    fake = bin_dir / "fake-browser"
    fake.write_text("#!/bin/sh\n")
    fake.chmod(0o755)
    (apps / "fake-browser.desktop").write_text(
        "[Desktop Entry]\n"
        "Type=Application\n"
        "Name=Real Name\n"
        "Name[de]=Lokaler Name\n"
        f"Exec={fake}\n"
        "MimeType=x-scheme-handler/http;\n"
    )
    monkeypatch.setenv("HOME", str(tmp_path / "home"))
    monkeypatch.setenv("PATH", str(bin_dir))

    found = [b for b in discover_browsers() if b.id == "fake-browser"]
    assert found and found[0].name == "Real Name"
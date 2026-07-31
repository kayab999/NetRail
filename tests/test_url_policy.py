"""Shared golden vectors: tests/fixtures/url_policy.json (also loaded by Rust)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from netrail.errors import NetRailError
from netrail.security import validate_backend_url, validate_open_url

FIXTURE = Path(__file__).parent / "fixtures" / "url_policy.json"


def _load_fixture() -> dict:
    return json.loads(FIXTURE.read_text(encoding="utf-8"))


def _open_cases():
    data = _load_fixture()
    return [pytest.param(case, id=case["id"]) for case in data["open_url"]]


def _backend_cases():
    data = _load_fixture()
    return [pytest.param(case, id=case["id"]) for case in data["backend_url"]]


@pytest.mark.parametrize("case", _open_cases())
def test_open_url_golden(case: dict):
    url = case["url"]
    if case["expect"] == "allow":
        got = validate_open_url(url)
        if "normalized" in case:
            assert got == case["normalized"]
    elif case["expect"] == "block":
        with pytest.raises(NetRailError) as exc:
            validate_open_url(url)
        if "code" in case:
            assert exc.value.code == case["code"]
    else:
        raise AssertionError(f"bad expect: {case['expect']}")


@pytest.mark.parametrize("case", _backend_cases())
def test_backend_url_golden(case: dict):
    url = case["url"]
    if case["expect"] == "allow":
        got = validate_backend_url(url)
        if "normalized" in case:
            assert got == case["normalized"]
    elif case["expect"] == "block":
        with pytest.raises(NetRailError) as exc:
            validate_backend_url(url)
        if "code" in case:
            assert exc.value.code == case["code"]
    else:
        raise AssertionError(f"bad expect: {case['expect']}")

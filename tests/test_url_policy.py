"""Shared golden vectors: tests/fixtures/url_policy.json (also loaded by Rust)."""

from __future__ import annotations

import ipaddress
import json
from pathlib import Path

import pytest

from netrail.errors import NetRailError
from netrail.security import (
    check_backend_fetch_url,
    validate_backend_url,
    validate_open_url,
)

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


def _backend_check(case: dict):
    strict = bool(case.get("strict", False))
    if "resolved_ips" in case:

        def check(url: str) -> str:
            ips = [ipaddress.ip_address(x) for x in case["resolved_ips"]]
            return check_backend_fetch_url(url, strict=strict, resolver=lambda _host: ips)

        return check
    return lambda url: validate_backend_url(url, strict=strict)


@pytest.mark.parametrize("case", _backend_cases())
def test_backend_url_golden(case: dict):
    url = case["url"]
    check = _backend_check(case)
    if case["expect"] == "allow":
        got = check(url)
        if "normalized" in case:
            assert got == case["normalized"]
    elif case["expect"] == "block":
        with pytest.raises(NetRailError) as exc:
            check(url)
        if "code" in case:
            assert exc.value.code == case["code"]
    else:
        raise AssertionError(f"bad expect: {case['expect']}")
"""Tests for generated TypeScript inventory comparison."""

from __future__ import annotations

import importlib.util
from pathlib import Path

_SCRIPT_PATH = Path(__file__).parents[1] / "check_generated_ts.py"
_SPEC = importlib.util.spec_from_file_location("check_generated_ts", _SCRIPT_PATH)
assert _SPEC is not None
assert _SPEC.loader is not None
_MODULE = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_MODULE)


def test_inventory_drift_detects_case_only_rename() -> None:
    """Linux TypeScript resolution fails when git casing disagrees with imports."""
    expected = {"CDSTrancheQuote.ts": b"export type CdsTrancheQuote = {};\n"}
    actual = {"CdsTrancheQuote.ts": b"export type CdsTrancheQuote = {};\n"}

    missing, extra, changed = _MODULE.inventory_drift(expected, actual)

    assert missing == ["CDSTrancheQuote.ts"]
    assert extra == ["CdsTrancheQuote.ts"]
    assert changed == []

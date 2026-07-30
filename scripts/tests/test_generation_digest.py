"""Tests for generated-artifact inventory and content drift checks."""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest

_SCRIPT_PATH = Path(__file__).parents[1] / "generation_digest.py"
_SPEC = importlib.util.spec_from_file_location("generation_digest", _SCRIPT_PATH)
assert _SPEC is not None
assert _SPEC.loader is not None
_MODULE = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_MODULE)


def _generated_tree(root: Path) -> list[Path]:
    generated = root / "generated"
    generated.mkdir()
    first = generated / "first.json"
    second = generated / "second.ts"
    first.write_text("{}\n", encoding="utf-8")
    second.write_text("export {};\n", encoding="utf-8")
    return [first, second]


def test_manifest_check_rejects_missing_and_extra_generated_files(tmp_path: Path) -> None:
    """The checked inventory fails closed for deleted and obsolete artifacts."""
    files = _generated_tree(tmp_path)
    manifest = tmp_path / "generated-artifacts.txt"
    _MODULE.write_manifest(tmp_path, [Path("generated")], manifest)

    _MODULE.check_manifest(tmp_path, [Path("generated")], manifest)

    files[0].unlink()
    with pytest.raises(ValueError, match=r"missing=.*first.json"):
        _MODULE.check_manifest(tmp_path, [Path("generated")], manifest)

    files[0].write_text("{}\n", encoding="utf-8")
    stale = tmp_path / "generated" / "obsolete.json"
    stale.write_text("{}\n", encoding="utf-8")
    with pytest.raises(ValueError, match=r"extra=.*obsolete.json"):
        _MODULE.check_manifest(tmp_path, [Path("generated")], manifest)


def test_manifest_updates_only_through_explicit_write(tmp_path: Path) -> None:
    """Normal checks never rewrite the expected generated path inventory."""
    _generated_tree(tmp_path)
    manifest = tmp_path / "generated-artifacts.txt"
    _MODULE.write_manifest(tmp_path, [Path("generated")], manifest)
    before = manifest.read_bytes()

    (tmp_path / "generated" / "obsolete.json").write_text("{}\n", encoding="utf-8")
    with pytest.raises(ValueError, match="manifest mismatch"):
        _MODULE.check_manifest(tmp_path, [Path("generated")], manifest)

    assert manifest.read_bytes() == before

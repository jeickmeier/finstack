"""Tests for the source line-count check."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys

_SCRIPT_PATH = Path(__file__).parents[1] / "check_loc.py"
_SPEC = importlib.util.spec_from_file_location("check_loc", _SCRIPT_PATH)
assert _SPEC is not None
assert _SPEC.loader is not None
_MODULE = importlib.util.module_from_spec(_SPEC)
sys.modules[_SPEC.name] = _MODULE
_SPEC.loader.exec_module(_MODULE)
collect_violations = _MODULE.collect_violations


def _write_long_file(path: Path, *, lines: int = 2000) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join("x" for _ in range(lines)), encoding="utf-8")


def test_excludes_exact_wasm_index_d_ts(tmp_path: Path) -> None:
    """The hand-maintained WASM facade declaration file is not counted."""
    _write_long_file(tmp_path / "finstack-quant-wasm" / "index.d.ts")

    violation_paths = [path for path, _ in collect_violations(tmp_path, 1500)]

    assert "finstack-quant-wasm/index.d.ts" not in violation_paths


def test_counts_other_d_ts_files(tmp_path: Path) -> None:
    """Other .d.ts files still count because Path.suffix is .ts."""
    relative_path = "finstack-quant-wasm/types.d.ts"
    _write_long_file(tmp_path / relative_path)

    violation_paths = [path for path, _ in collect_violations(tmp_path, 1500)]

    assert relative_path in violation_paths


def test_counts_similarly_named_index_d_ts_elsewhere(tmp_path: Path) -> None:
    """Only the exact wasm facade path is excluded, not homonyms elsewhere."""
    relative_path = "other-package/index.d.ts"
    _write_long_file(tmp_path / relative_path)

    violation_paths = [path for path, _ in collect_violations(tmp_path, 1500)]

    assert relative_path in violation_paths

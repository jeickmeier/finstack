"""Tests for the tiny source-file collapse-candidate report."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys

_SCRIPT_PATH = Path(__file__).parents[1] / "check_tiny_loc.py"
_SPEC = importlib.util.spec_from_file_location("check_tiny_loc", _SCRIPT_PATH)
assert _SPEC is not None
assert _SPEC.loader is not None
_MODULE = importlib.util.module_from_spec(_SPEC)
sys.modules[_SPEC.name] = _MODULE
_SPEC.loader.exec_module(_MODULE)
collect_candidates = _MODULE.collect_candidates
group_by_parent = _MODULE.group_by_parent
run_report = _MODULE.run_report


def _write_file(path: Path, *, lines: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(f"line_{i}" for i in range(lines)) + "\n", encoding="utf-8")


def test_reports_files_at_or_below_threshold(tmp_path: Path) -> None:
    """Files within the limit are candidates; larger files are not."""
    _write_file(tmp_path / "src" / "tiny.rs", lines=10)
    _write_file(tmp_path / "src" / "border.rs", lines=25)
    _write_file(tmp_path / "src" / "large.rs", lines=26)

    candidates = collect_candidates(tmp_path, 25)
    paths = {path for path, _ in candidates}

    assert "src/tiny.rs" in paths
    assert "src/border.rs" in paths
    assert "src/large.rs" not in paths


def test_excludes_glue_basenames(tmp_path: Path) -> None:
    """Module-wiring glue files are never reported."""
    _write_file(tmp_path / "crate" / "mod.rs", lines=5)
    _write_file(tmp_path / "crate" / "lib.rs", lines=5)
    _write_file(tmp_path / "py_mod" / "__init__.py", lines=5)
    _write_file(tmp_path / "crate" / "logic.rs", lines=5)

    paths = {path for path, _ in collect_candidates(tmp_path, 25)}

    assert "crate/mod.rs" not in paths
    assert "crate/lib.rs" not in paths
    assert "py_mod/__init__.py" not in paths
    assert "crate/logic.rs" in paths


def test_excludes_pure_reexport_index_ts(tmp_path: Path) -> None:
    """Pure TypeScript re-export facades are treated as glue."""
    reexport = tmp_path / "facade" / "index.ts"
    reexport.parent.mkdir(parents=True, exist_ok=True)
    reexport.write_text(
        "export { Foo } from './foo';\nexport * from './bar';\n",
        encoding="utf-8",
    )
    _write_file(tmp_path / "facade" / "foo.ts", lines=5)

    paths = {path for path, _ in collect_candidates(tmp_path, 25)}

    assert "facade/index.ts" not in paths
    assert "facade/foo.ts" in paths


def test_counts_nontrivial_index_ts(tmp_path: Path) -> None:
    """Non-trivial index.ts files are still collapse candidates when tiny."""
    index = tmp_path / "facade" / "index.ts"
    index.parent.mkdir(parents=True, exist_ok=True)
    index.write_text(
        "export { Foo } from './foo';\nexport function helper(): number {\n  return 1;\n}\n",
        encoding="utf-8",
    )

    paths = {path for path, _ in collect_candidates(tmp_path, 25)}

    assert "facade/index.ts" in paths


def test_groups_by_parent_directory(tmp_path: Path) -> None:
    """Candidates are clustered by parent with densest groups first."""
    _write_file(tmp_path / "a" / "one.rs", lines=8)
    _write_file(tmp_path / "a" / "two.rs", lines=12)
    _write_file(tmp_path / "b" / "alone.rs", lines=15)

    candidates = collect_candidates(tmp_path, 25)
    groups = group_by_parent(candidates)

    assert [parent for parent, _ in groups] == ["a", "b"]
    assert [path for path, _ in groups[0][1]] == ["a/one.rs", "a/two.rs"]


def test_skips_zero_line_files(tmp_path: Path) -> None:
    """Empty files are not reported as collapse candidates."""
    empty = tmp_path / "src" / "empty.py"
    empty.parent.mkdir(parents=True, exist_ok=True)
    empty.write_text("", encoding="utf-8")
    _write_file(tmp_path / "src" / "tiny.py", lines=3)

    paths = {path for path, _ in collect_candidates(tmp_path, 25)}

    assert "src/empty.py" not in paths
    assert "src/tiny.py" in paths


def test_run_report_always_exits_zero(tmp_path: Path, capsys) -> None:
    """Advisory report always returns 0 even when candidates exist."""
    _write_file(tmp_path / "src" / "tiny.rs", lines=5)

    assert run_report(tmp_path, 25) == 0
    captured = capsys.readouterr()
    assert "tiny file" in captured.out
    assert "src/tiny.rs" in captured.out


def test_threshold_override(tmp_path: Path) -> None:
    """Positional limit override changes which files qualify."""
    _write_file(tmp_path / "src" / "mid.rs", lines=40)

    assert collect_candidates(tmp_path, 25) == []
    paths = {path for path, _ in collect_candidates(tmp_path, 50)}
    assert "src/mid.rs" in paths


def test_excludes_test_directories(tmp_path: Path) -> None:
    """Files under test/tests/__tests__ directories are not candidates."""
    _write_file(tmp_path / "src" / "logic.rs", lines=5)
    _write_file(tmp_path / "tests" / "unit.rs", lines=5)
    _write_file(tmp_path / "src" / "test" / "helper.py", lines=5)
    _write_file(tmp_path / "crate" / "__tests__" / "smoke.ts", lines=5)

    paths = {path for path, _ in collect_candidates(tmp_path, 25)}

    assert "src/logic.rs" in paths
    assert "tests/unit.rs" not in paths
    assert "src/test/helper.py" not in paths
    assert "crate/__tests__/smoke.ts" not in paths


def test_excludes_generated_directories(tmp_path: Path) -> None:
    """Files under generated/ directories are not candidates."""
    _write_file(tmp_path / "src" / "logic.rs", lines=5)
    _write_file(tmp_path / "types" / "generated" / "Enum.ts", lines=6)

    paths = {path for path, _ in collect_candidates(tmp_path, 25)}

    assert "src/logic.rs" in paths
    assert "types/generated/Enum.ts" not in paths

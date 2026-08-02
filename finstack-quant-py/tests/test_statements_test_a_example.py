from __future__ import annotations

from pathlib import Path
import stat
import subprocess
import sys

WORKSPACE_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = WORKSPACE_ROOT / "finstack-quant-py/examples/scripts/statements_test_a.py"
CANONICAL_REPORT_PATH = WORKSPACE_ROOT / "finstack-quant-py/examples/reports/statements_test_a_report.md"


def run_example(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(  # noqa: S603 - fixed interpreter and repository-owned script path.
        [sys.executable, str(SCRIPT_PATH), *args],
        cwd=WORKSPACE_ROOT,
        capture_output=True,
        check=False,
        text=True,
    )


def test_statements_test_a_default_passes_mechanical_checks() -> None:
    result = run_example()
    assert result.returncode == 0, result.stderr
    assert "A1 PASS — 40 nodes, 3 periods, 2 adjustments" in result.stdout
    assert "A2 PASS — normalization schema published and validated" in result.stdout
    assert "A3 CONDITIONAL — synthetic unsupported share 40.0%" in result.stdout
    assert "A4 CONDITIONAL — production evidence pending Test B" in result.stdout


def test_statements_test_a_strict_mode_exposes_conditional_readiness() -> None:
    result = run_example("--strict-readiness")
    assert result.returncode == 2, result.stderr
    assert "production evidence pending Test B" in result.stdout


def test_statements_test_a_writes_named_report_atomically(tmp_path: Path) -> None:
    report_path = tmp_path / "test_a_report.md"
    result = run_example(
        "--write-report",
        "--report-path",
        str(report_path),
        "--signer",
        "Jon",
        "--signed-on",
        "2026-08-02",
    )
    assert result.returncode == 0, result.stderr
    report = report_path.read_text(encoding="utf-8")
    assert "# Test A — Financial Statements" in report
    assert "Decision: **CONDITIONAL**" in report
    assert "Unsupported synthetic share: **4 / 10 (40.0%)**" in report
    assert "Signer: **Jon**" in report
    assert "Signed on: **2026-08-02**" in report
    # The management-fee adjustment now carries a self-referential cap
    # (base_node="ebitda", value=0.05) with base_mode omitted, so the
    # default `CapBaseMode::Reported` binds against the *reported*
    # (pre-adjustment) 2025Q1 EBITDA of 170.0: 170.0 * 0.05 = 8.5, below the
    # uncapped 1% of revenue (1000.0 * 0.01 = 10.0). Final: 170.0 + 17.0 +
    # 8.5 = 195.5.
    assert "| 2025Q1 | 170.0 | 17.0 | 8.5 | 195.5 |" in report
    # Reproducibility content: the model schema path and the exact
    # report-generation command must be recorded in the generated report.
    assert "finstack-quant/statements/schemas/statements/1/financial_model_spec.schema.json" in report
    assert (
        "uv run python finstack-quant-py/examples/scripts/statements_test_a.py "
        "--write-report --signer Jon --signed-on 2026-08-02" in report
    )


def test_statements_test_a_default_run_does_not_mutate_canonical_report() -> None:
    before = CANONICAL_REPORT_PATH.read_bytes()
    before_mtime = CANONICAL_REPORT_PATH.stat().st_mtime_ns
    result = run_example()
    after = CANONICAL_REPORT_PATH.read_bytes()
    assert result.returncode == 0, result.stderr
    assert "Wrote" not in result.stdout
    assert after == before, "the default no-argument run must not mutate the checked-in report"
    assert CANONICAL_REPORT_PATH.stat().st_mtime_ns == before_mtime


def test_statements_test_a_report_write_failure_leaves_destination_intact(tmp_path: Path) -> None:
    # A pre-existing directory at the report path makes the atomic
    # `Path.replace` call fail deterministically (`IsADirectoryError`)
    # without depending on filesystem permissions.
    report_path = tmp_path / "existing_directory"
    report_path.mkdir()
    marker_path = report_path / "marker.txt"
    marker_path.write_text("do not touch", encoding="utf-8")
    marker_mode_before = stat.S_IMODE(marker_path.stat().st_mode)

    result = run_example(
        "--write-report",
        "--report-path",
        str(report_path),
        "--signer",
        "Jon",
        "--signed-on",
        "2026-08-02",
    )

    assert result.returncode == 1
    assert "Test A report write failure" in result.stderr
    assert marker_path.read_text(encoding="utf-8") == "do not touch"
    assert stat.S_IMODE(marker_path.stat().st_mode) == marker_mode_before
    # No stray temporary file left behind in the destination directory.
    assert list(report_path.iterdir()) == [marker_path]

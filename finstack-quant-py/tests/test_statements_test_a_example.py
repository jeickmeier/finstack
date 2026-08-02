from __future__ import annotations

from pathlib import Path
import subprocess
import sys

WORKSPACE_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = WORKSPACE_ROOT / "finstack-quant-py/examples/scripts/statements_test_a.py"


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

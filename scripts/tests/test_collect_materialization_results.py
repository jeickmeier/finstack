"""Tests for materialization benchmark result aggregation."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import sys

import pytest

_SCRIPT_PATH = Path(__file__).parents[1] / "collect_materialization_results.py"
sys.path.insert(0, str(_SCRIPT_PATH.parent))
_SPEC = importlib.util.spec_from_file_location("collect_materialization_results", _SCRIPT_PATH)
assert _SPEC is not None
assert _SPEC.loader is not None
_MODULE = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_MODULE)


def test_independent_overhead_accepts_different_sample_counts_and_is_deterministic() -> None:
    """Language distributions are never aligned by sample index."""
    binding = {"case": [3.0] * 101}
    rust = {"native": [1.0] * 137}

    first = _MODULE.independent_overhead(binding, rust, {"case": "native"})
    second = _MODULE.independent_overhead(binding, rust, {"case": "native"})

    assert first == second
    result = first["case"]
    assert result["estimate_type"] == "independent_samples"
    assert result["binding_samples"] == 101
    assert result["rust_samples"] == 137
    assert result["median_difference_ms"] == 2.0
    assert result["p95_difference_95pct_bootstrap_ci_ms"] == {
        "lower": 2.0,
        "upper": 2.0,
    }
    assert "raw_ms" not in result


def test_criterion_results_include_paths_and_point_range_estimates(tmp_path: Path) -> None:
    """The machine record preserves auditable Criterion identities and intervals."""
    for case in _MODULE._CRITERION_CASES:
        benchmark = tmp_path / "portfolio_materialization" / case
        for name, estimate in (
            ("named-v1", (10_000_000.0, 9_000_000.0, 11_000_000.0)),
            ("new", (12_000_000.0, 11_000_000.0, 13_000_000.0)),
            ("change", (0.2, 0.1, 0.3)),
        ):
            destination = benchmark / name
            destination.mkdir(parents=True)
            point, lower, upper = estimate
            (destination / "estimates.json").write_text(
                json.dumps({
                    "median": {
                        "point_estimate": point,
                        "confidence_interval": {
                            "lower_bound": lower,
                            "upper_bound": upper,
                        },
                    }
                }),
                encoding="utf-8",
            )

    results = _MODULE.criterion_results(tmp_path, "named-v1")

    cold_a = results["cold_a_5000_unique"]
    assert cold_a["benchmark_id"] == "portfolio_materialization/cold_a_5000_unique"
    assert cold_a["baseline_median_ms"] == {"point": 10.0, "lower": 9.0, "upper": 11.0}
    assert cold_a["current_median_ms"] == {"point": 12.0, "lower": 11.0, "upper": 13.0}
    assert cold_a["relative_median_change"] == {"point": 0.2, "lower": 0.1, "upper": 0.3}
    assert cold_a["paths"]["baseline"].endswith("named-v1/estimates.json")


def test_python_comparison_rejects_mismatched_baseline_provenance() -> None:
    """A result collected against another Python baseline cannot enter the record."""
    cases = ["cold_a_5000_unique", "cold_b_5000_50", "warm_b_5000_50"]
    baseline = {
        "baseline_name": "named-v1",
        "tree_revision": "tree-baseline",
        "fixture_sha256": "a" * 64,
        "created_at_ns": 1,
    }
    comparison = {
        "schema": "finstack_quant.python_materialization_comparison/1",
        "baseline": {**baseline, "tree_revision": "wrong-tree"},
        "current": {
            "tree_revision": "tree-current",
            "fixture_sha256": "a" * 64,
        },
        "expected_cases": cases,
        "median_changes": dict.fromkeys(cases, 0.0),
        "passed": True,
    }

    with pytest.raises(ValueError, match="baseline provenance mismatch"):
        _MODULE.validate_python_comparison(
            baseline,
            comparison,
            "a" * 64,
            "tree-current",
        )


def test_recorded_provenance_retains_exact_measurement_metadata(tmp_path: Path) -> None:
    """Record migration cannot relabel old measurements as a new tree or toolchain."""
    source = tmp_path / "record.json"
    source.write_text(
        json.dumps({
            "recorded_at": "2026-07-29",
            "revision": {
                "base_revision": "base",
                "tree_revision": "base-tree-dirty",
            },
            "environment": {
                "rustc": "rustc recorded",
                "python": "python recorded",
                "node": "node recorded",
            },
        }),
        encoding="utf-8",
    )

    recorded_at, revision, environment = _MODULE.recorded_provenance(source)

    assert recorded_at == "2026-07-29"
    assert revision == {
        "base_revision": "base",
        "tree_revision": "base-tree-dirty",
    }
    assert environment["rustc"] == "rustc recorded"

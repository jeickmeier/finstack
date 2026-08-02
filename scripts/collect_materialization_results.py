"""Combine raw Rust/Python/WASM materialization samples into the tracked record."""

from __future__ import annotations

import argparse
from datetime import UTC, datetime
import hashlib
import json
import math
from pathlib import Path
import platform
import random
import shutil
import subprocess
import sys
from typing import Any

from prepare_materialization_criterion_run import combined_fixture_digest, tree_revision

_CRITERION_CASES = (
    "cold_a_5000_unique",
    "cold_b_5000_50",
    "warm_b_5000_50",
    "validation_b_5000_50",
)


def command_output(*command: str) -> str:
    """Return command output, or an explicit unavailable marker."""
    try:
        executable = shutil.which(command[0])
        if executable is None:
            return f"unavailable: {command[0]} not found"
        return subprocess.check_output(  # noqa: S603 -- executable resolved from fixed callers
            (executable, *command[1:]), text=True
        ).strip()
    except (OSError, subprocess.CalledProcessError) as error:
        return f"unavailable: {error}"


def nearest_rank(samples: list[float], percentile: float) -> float:
    """Return a finite nearest-rank percentile."""
    ordered = sorted(samples)
    rank = math.ceil(len(ordered) * percentile / 100)
    return ordered[rank - 1]


def validate_samples(name: str, values: Any) -> list[float]:
    """Require at least 100 finite numeric acceptance samples."""
    if not isinstance(values, list) or len(values) < 100:
        raise ValueError(f"{name} requires at least 100 raw samples")
    samples = [float(value) for value in values]
    if not all(math.isfinite(value) and value >= 0.0 for value in samples):
        raise ValueError(f"{name} contains a non-finite or negative sample")
    return samples


def normalized_cases(payload: dict[str, Any]) -> dict[str, list[float]]:
    """Normalize language fragment case shapes."""
    cases: dict[str, list[float]] = {}
    for name, value in payload["cases"].items():
        samples = value["samples"] if isinstance(value, dict) else value
        cases[name] = validate_samples(f"{payload['language']}:{name}", samples)
    return cases


def normalized_counters(payload: dict[str, Any]) -> dict[str, Any] | None:
    """Normalize top-level and per-case counter fragment shapes."""
    if payload.get("counters") is not None:
        return payload["counters"]
    counters = {
        name: value["counters"]
        for name, value in payload["cases"].items()
        if isinstance(value, dict) and value.get("counters") is not None
    }
    return counters or None


def stats(samples: list[float]) -> dict[str, float | int]:
    """Summarize one raw sample vector without discarding it."""
    return {
        "samples": len(samples),
        "median_ms": nearest_rank(samples, 50),
        "p95_ms": nearest_rank(samples, 95),
        "min_ms": min(samples),
        "max_ms": max(samples),
    }


def independent_overhead(
    binding_cases: dict[str, list[float]],
    rust_cases: dict[str, list[float]],
    case_map: dict[str, str],
) -> dict[str, Any]:
    """Estimate independent-sample binding overhead with deterministic bootstrap intervals."""
    result = {}
    for binding_case, rust_case in case_map.items():
        binding = binding_cases[binding_case]
        rust = rust_cases[rust_case]
        seed = int.from_bytes(
            hashlib.sha256(f"{binding_case}\0{rust_case}".encode()).digest()[:8],
            byteorder="big",
        )
        generator = random.Random(seed)
        median_differences = []
        p95_differences = []
        for _ in range(5_000):
            binding_sample = generator.choices(binding, k=len(binding))
            rust_sample = generator.choices(rust, k=len(rust))
            median_differences.append(nearest_rank(binding_sample, 50) - nearest_rank(rust_sample, 50))
            p95_differences.append(nearest_rank(binding_sample, 95) - nearest_rank(rust_sample, 95))
        result[binding_case] = {
            "rust_case": rust_case,
            "estimate_type": "independent_samples",
            "binding_samples": len(binding),
            "rust_samples": len(rust),
            "median_difference_ms": nearest_rank(binding, 50) - nearest_rank(rust, 50),
            "median_difference_95pct_bootstrap_ci_ms": {
                "lower": nearest_rank(median_differences, 2.5),
                "upper": nearest_rank(median_differences, 97.5),
            },
            "p95_difference_ms": nearest_rank(binding, 95) - nearest_rank(rust, 95),
            "p95_difference_95pct_bootstrap_ci_ms": {
                "lower": nearest_rank(p95_differences, 2.5),
                "upper": nearest_rank(p95_differences, 97.5),
            },
            "uncertainty": {
                "method": "independent_nonparametric_bootstrap",
                "confidence_level": 0.95,
                "resamples": 5_000,
                "seed": seed,
            },
        }
    return result


def sha256(path: Path) -> str:
    """Hash one fixture file."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def recorded_provenance(previous_record: Path | None) -> tuple[str, dict[str, str], dict[str, Any]]:
    """Return the date, revision, and toolchain that produced the measurements."""
    if previous_record is None:
        revision = {
            "base_revision": command_output("git", "rev-parse", "HEAD"),
            "tree_revision": tree_revision(),
        }
        environment = {
            "platform": platform.platform(),
            "machine": platform.machine(),
            "cpu": command_output("sysctl", "-n", "machdep.cpu.brand_string"),
            "physical_cpus": command_output("sysctl", "-n", "hw.physicalcpu"),
            "logical_cpus": command_output("sysctl", "-n", "hw.logicalcpu"),
            "rustc": command_output("rustc", "-Vv"),
            "python": sys.version,
            "node": command_output("node", "--version"),
        }
        return datetime.now(UTC).date().isoformat(), revision, environment

    previous = json.loads(previous_record.read_text(encoding="utf-8"))
    recorded_at = previous.get("recorded_at")
    revision = previous.get("revision")
    environment = previous.get("environment")
    if not isinstance(recorded_at, str) or not recorded_at:
        raise ValueError("provenance record requires recorded_at")
    if not isinstance(revision, dict) or not all(
        isinstance(revision.get(key), str) and revision[key] for key in ("base_revision", "tree_revision")
    ):
        raise ValueError("provenance record requires exact base and tree revisions")
    if not isinstance(environment, dict):
        raise TypeError("provenance record requires environment metadata")
    return recorded_at, revision, environment


def criterion_results(root: Path, baseline_name: str) -> dict[str, Any]:
    """Read named Criterion baseline, current, and change estimates."""
    results = {}
    for case in _CRITERION_CASES:
        benchmark_id = f"portfolio_materialization/{case}"
        paths = {
            "baseline": Path(benchmark_id) / baseline_name / "estimates.json",
            "current": Path(benchmark_id) / "new" / "estimates.json",
            "change": Path(benchmark_id) / "change" / "estimates.json",
        }
        payloads = {name: json.loads((root / path).read_text(encoding="utf-8")) for name, path in paths.items()}

        def median_estimate(name: str, *, nanos: bool, sources: dict[str, Any] = payloads) -> dict[str, float]:
            estimate = sources[name]["median"]
            divisor = 1_000_000.0 if nanos else 1.0
            return {
                "point": float(estimate["point_estimate"]) / divisor,
                "lower": float(estimate["confidence_interval"]["lower_bound"]) / divisor,
                "upper": float(estimate["confidence_interval"]["upper_bound"]) / divisor,
            }

        results[case] = {
            "benchmark_id": benchmark_id,
            "paths": {name: path.as_posix() for name, path in paths.items()},
            "baseline_median_ms": median_estimate("baseline", nanos=True),
            "current_median_ms": median_estimate("current", nanos=True),
            "relative_median_change": median_estimate("change", nanos=False),
        }
    return results


def validate_python_comparison(
    baseline: dict[str, Any],
    comparison: dict[str, Any],
    combined_digest: str,
    current_revision: str,
) -> dict[str, float]:
    """Validate Python comparison provenance and return exact median changes."""
    if comparison.get("schema") != "finstack_quant.python_materialization_comparison/1":
        raise ValueError("unsupported Python comparison schema")
    expected_cases = ["cold_a_5000_unique", "cold_b_5000_50", "warm_b_5000_50"]
    if comparison.get("expected_cases") != expected_cases:
        raise ValueError("Python comparison case set mismatch")
    expected_baseline = {
        "baseline_name": baseline.get("baseline_name"),
        "tree_revision": baseline.get("tree_revision"),
        "fixture_sha256": baseline.get("fixture_sha256"),
    }
    if comparison.get("baseline") != expected_baseline:
        raise ValueError("Python comparison baseline provenance mismatch")
    if baseline.get("fixture_sha256") != combined_digest:
        raise ValueError("Python named baseline fixture digest mismatch")
    current = comparison.get("current", {})
    if current.get("fixture_sha256") != combined_digest:
        raise ValueError("Python comparison current fixture digest mismatch")
    if current.get("tree_revision") != current_revision:
        raise ValueError("Python comparison current revision is stale")
    changes = comparison.get("median_changes")
    if not isinstance(changes, dict) or set(changes) != set(expected_cases):
        raise ValueError("Python comparison median-change case set mismatch")
    normalized = {case: float(change) for case, change in changes.items()}
    if not all(math.isfinite(change) for change in normalized.values()):
        raise ValueError("Python comparison contains a non-finite median change")
    if not comparison.get("passed"):
        raise ValueError("Python materialization regression gate failed")
    return normalized


def main() -> int:
    """Build the complete tracked machine-readable acceptance record."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rust", type=Path, required=True)
    parser.add_argument("--python", type=Path, required=True)
    parser.add_argument("--wasm", type=Path, required=True)
    parser.add_argument("--fixture-a", type=Path, required=True)
    parser.add_argument("--fixture-b", type=Path, required=True)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--rust-baseline", type=Path, required=True)
    parser.add_argument("--rust-comparison", type=Path, required=True)
    parser.add_argument("--python-baseline", type=Path, required=True)
    parser.add_argument("--python-comparison", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument(
        "--provenance-record",
        type=Path,
        help="Existing measured record whose exact date, revision, and toolchain are retained",
    )
    parser.add_argument("--command", action="append", default=[])
    args = parser.parse_args()

    fragments = {
        language: json.loads(path.read_text(encoding="utf-8"))
        for language, path in (
            ("rust", args.rust),
            ("python", args.python),
            ("wasm", args.wasm),
        )
    }
    cases = {language: normalized_cases(payload) for language, payload in fragments.items()}
    digest_a = sha256(args.fixture_a)
    digest_b = sha256(args.fixture_b)
    for language in ("python", "wasm"):
        if fragments[language]["fixture_sha256"] != {"a": digest_a, "b": digest_b}:
            raise ValueError(f"{language} measured different fixture bytes")

    baseline_manifest = json.loads(args.baseline.read_text(encoding="utf-8"))
    combined_digest = combined_fixture_digest(args.fixture_a, args.fixture_b)
    if baseline_manifest["fixture_sha256"] != combined_digest:
        raise ValueError("baseline manifest fixture digest does not match measured fixtures")

    rust_baseline = json.loads(args.rust_baseline.read_text(encoding="utf-8"))
    rust_comparison = json.loads(args.rust_comparison.read_text(encoding="utf-8"))
    python_baseline = json.loads(args.python_baseline.read_text(encoding="utf-8"))
    python_comparison = json.loads(args.python_comparison.read_text(encoding="utf-8"))
    expected_rust_manifest = {
        "baseline_name": baseline_manifest.get("rust_baseline_name"),
        "tree_revision": baseline_manifest.get("rust_baseline_tree_revision"),
        "fixture_sha256": baseline_manifest.get("rust_baseline_fixture_sha256"),
        "sha256": baseline_manifest.get("rust_baseline_sha256"),
        "expected_cases": baseline_manifest.get("rust_baseline_cases"),
    }
    actual_rust_manifest = {
        "baseline_name": rust_baseline.get("baseline_name"),
        "tree_revision": rust_baseline.get("tree_revision"),
        "fixture_sha256": rust_baseline.get("fixture_sha256"),
        "sha256": sha256(args.rust_baseline),
        "expected_cases": rust_baseline.get("expected_cases"),
    }
    if actual_rust_manifest != expected_rust_manifest:
        raise ValueError("immutable Rust baseline manifest mismatch")
    expected_python_manifest = {
        "baseline_name": baseline_manifest.get("python_baseline_name"),
        "tree_revision": baseline_manifest.get("python_baseline_tree_revision"),
        "fixture_sha256": baseline_manifest.get("python_baseline_fixture_sha256"),
        "sha256": baseline_manifest.get("python_baseline_sha256"),
        "expected_cases": baseline_manifest.get("python_baseline_cases"),
    }
    actual_python_manifest = {
        "baseline_name": python_baseline.get("baseline_name"),
        "tree_revision": python_baseline.get("tree_revision"),
        "fixture_sha256": python_baseline.get("fixture_sha256"),
        "sha256": sha256(args.python_baseline),
        "expected_cases": python_baseline.get("expected_cases"),
    }
    if actual_python_manifest != expected_python_manifest:
        raise ValueError("immutable Python baseline manifest mismatch")
    recorded_at, revision, environment = recorded_provenance(args.provenance_record)
    current_tree_revision = revision["tree_revision"]
    if rust_comparison.get("schema") != "finstack_quant.materialization_rust_comparison/1":
        raise ValueError("unsupported Rust materialization comparison schema")
    if rust_comparison.get("baseline") != {
        "baseline_name": rust_baseline["baseline_name"],
        "tree_revision": rust_baseline["tree_revision"],
        "fixture_sha256": rust_baseline["fixture_sha256"],
    }:
        raise ValueError("Rust comparison baseline provenance mismatch")
    if rust_comparison.get("current") != {
        "tree_revision": current_tree_revision,
        "fixture_sha256": combined_digest,
    }:
        raise ValueError("Rust comparison current provenance mismatch")
    if not rust_comparison.get("passed"):
        raise ValueError("Rust materialization regression gate failed")
    criterion = rust_comparison.get("cases")
    if not isinstance(criterion, dict) or set(criterion) != set(_CRITERION_CASES):
        raise ValueError("Rust comparison case set mismatch")
    python_changes = validate_python_comparison(
        python_baseline,
        python_comparison,
        combined_digest,
        current_tree_revision,
    )

    rust_stats = {name: stats(samples) for name, samples in cases["rust"].items()}
    regression_changes = {case: result["relative_median_change"]["point"] for case, result in criterion.items()}
    output = {
        "schema": "finstack_quant.materialization_benchmark_results/1",
        "recorded_at": recorded_at,
        "baselines": {
            "manifest": {
                "path": str(args.baseline),
                "sha256": sha256(args.baseline),
            },
            "rust": {
                "path": str(args.rust_baseline),
                "sha256": sha256(args.rust_baseline),
                "baseline_name": rust_baseline["baseline_name"],
                "tree_revision": rust_baseline["tree_revision"],
                "fixture_sha256": rust_baseline["fixture_sha256"],
            },
            "python": {
                "path": str(args.python_baseline),
                "sha256": sha256(args.python_baseline),
                "baseline_name": python_baseline["baseline_name"],
                "tree_revision": python_baseline["tree_revision"],
                "fixture_sha256": python_baseline["fixture_sha256"],
            },
        },
        "revision": revision,
        "fixtures": {
            "a": {"path": str(args.fixture_a), "sha256": digest_a},
            "b": {"path": str(args.fixture_b), "sha256": digest_b},
            "combined_sha256": combined_digest,
        },
        "commands": args.command,
        "timing_boundaries": {language: payload["timing_boundary"] for language, payload in fragments.items()},
        "environment": environment,
        "languages": {
            language: {
                "raw_samples_ms": language_cases,
                "statistics": {name: stats(samples) for name, samples in language_cases.items()},
                "counters": normalized_counters(fragments[language]),
            }
            for language, language_cases in cases.items()
        },
        "independent_binding_overhead": {
            "interpretation": (
                "Binding and Rust measurements are independent distributions; differences are "
                "independent-sample estimates with deterministic bootstrap uncertainty."
            ),
            "python_minus_rust": independent_overhead(
                cases["python"],
                cases["rust"],
                {
                    "cold_a_5000_unique": "cold_a_5000_unique",
                    "cold_b_5000_50": "cold_b_5000_50",
                    "warm_b_5000_50": "warm_b_5000_50",
                },
            ),
            "wasm_minus_rust": independent_overhead(
                cases["wasm"],
                cases["rust"],
                {
                    "fromMaterialization cold A (5000/5000)": "cold_a_5000_unique",
                    "fromMaterialization cold B (5000/50)": "cold_b_5000_50",
                    "fromMaterialization warm B (5000/50)": "warm_b_5000_50",
                    "validateMaterializationJson B (5000/50)": "validation_b_5000_50",
                },
            ),
        },
        "gates": {
            "rust_cold_a_p95_below_1000ms": rust_stats["cold_a_5000_unique"]["p95_ms"] < 1_000,
            "rust_cold_a_p95_below_500ms_target": rust_stats["cold_a_5000_unique"]["p95_ms"] < 500,
            "rust_warm_b_p95_below_250ms_target": rust_stats["warm_b_5000_50"]["p95_ms"] < 250,
            "criterion_median_changes": regression_changes,
            "criterion_all_changes_at_or_below_10pct": all(change <= 0.10 for change in regression_changes.values()),
            "python_median_changes": python_changes,
            "python_all_changes_at_or_below_10pct": all(float(change) <= 0.10 for change in python_changes.values()),
        },
        "criterion": criterion,
        "rust_regression": rust_comparison,
        "python_regression": python_comparison,
    }
    serialized = json.dumps(output, indent=2) + "\n"
    args.output.write_text(serialized, encoding="utf-8")
    written = args.output.read_text(encoding="utf-8")
    if written != serialized or json.loads(written) != output:
        raise OSError(f"failed to verify collected benchmark record: {args.output}")
    print(f"Materialization benchmark record collected and verified: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

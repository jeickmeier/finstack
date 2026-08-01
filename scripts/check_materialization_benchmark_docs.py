"""Verify benchmark documentation toolchains match the machine artifact."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


def expected_toolchain_lines(environment: dict[str, Any]) -> tuple[str, ...]:
    """Build exact documentation lines from machine-recorded environment fields."""
    rust_lines = dict(
        line.split(": ", maxsplit=1) for line in str(environment["rustc"]).splitlines()[1:] if ": " in line
    )
    rust_version = str(environment["rustc"]).splitlines()[0]
    return (
        f"- Platform: {environment['platform']}.",
        (
            f"- CPU: {environment['cpu']}, {environment['physical_cpus']} physical / "
            f"{environment['logical_cpus']} logical cores."
        ),
        (f"- Rust: {rust_version}, LLVM {rust_lines['LLVM version']}, host\n  {rust_lines['host']}."),
        f"- Python: CPython {environment['python']}.",
        f"- Node.js: {environment['node']}.",
    )


def verify_document(document: str, artifact: dict[str, Any]) -> None:
    """Require every machine-derived toolchain line exactly once."""
    for line in expected_toolchain_lines(artifact["environment"]):
        count = document.count(line)
        if count != 1:
            raise ValueError(f"expected exactly one documented toolchain line: {line!r}")


def file_sha256(path: Path) -> str:
    """Return the lowercase SHA-256 digest of one tracked artifact."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_baseline_links(
    artifact: dict[str, Any],
    manifest: dict[str, Any],
    manifest_path: Path,
    rust_baseline_path: Path,
    python_baseline_path: Path,
) -> None:
    """Verify checked-in baseline identities and record links without measuring."""
    expected_paths = {
        "rust": Path("benchmarks/materialization/materialization-rust-baseline.json"),
        "python": Path("benchmarks/materialization/materialization-python-baseline.json"),
    }
    supplied_paths = {
        "rust": rust_baseline_path,
        "python": python_baseline_path,
    }
    for language, expected_path in expected_paths.items():
        declared_path = manifest.get(f"{language}_baseline_path")
        if declared_path != expected_path.as_posix() or supplied_paths[language] != expected_path:
            raise ValueError(
                f"{language} baseline path mismatch: expected {expected_path.as_posix()}, "
                f"manifest={declared_path!r}, supplied={supplied_paths[language].as_posix()!r}"
            )
        if not expected_path.is_file():
            raise ValueError(f"{language} baseline does not exist: {expected_path.as_posix()}")

    rust_baseline = json.loads(rust_baseline_path.read_text(encoding="utf-8"))
    python_baseline = json.loads(python_baseline_path.read_text(encoding="utf-8"))
    if manifest.get("schema") != "finstack_quant.materialization_benchmark_baseline_manifest/1":
        raise ValueError("unsupported materialization baseline manifest schema")
    if artifact.get("schema") != "finstack_quant.materialization_benchmark_results/1":
        raise ValueError("unsupported materialization benchmark result schema")

    for language, path, baseline in (
        ("rust", rust_baseline_path, rust_baseline),
        ("python", python_baseline_path, python_baseline),
    ):
        digest = file_sha256(path)
        expected = {
            "path": path.as_posix(),
            "sha256": digest,
            "baseline_name": baseline["baseline_name"],
            "tree_revision": baseline["tree_revision"],
            "fixture_sha256": baseline["fixture_sha256"],
        }
        if artifact.get("baselines", {}).get(language) != expected:
            raise ValueError(f"{language} baseline link or digest mismatch")
        prefix = f"{language}_baseline"
        manifest_identity = {
            "baseline_name": manifest.get(f"{prefix}_name"),
            "tree_revision": manifest.get(f"{prefix}_tree_revision"),
            "fixture_sha256": manifest.get(f"{prefix}_fixture_sha256"),
            "sha256": manifest.get(f"{prefix}_sha256"),
            "expected_cases": manifest.get(f"{prefix}_cases"),
        }
        baseline_identity = {
            "baseline_name": baseline.get("baseline_name"),
            "tree_revision": baseline.get("tree_revision"),
            "fixture_sha256": baseline.get("fixture_sha256"),
            "sha256": digest,
            "expected_cases": baseline.get("expected_cases"),
        }
        if baseline_identity != manifest_identity:
            raise ValueError(f"{language} baseline manifest identity mismatch")

    manifest_link = artifact.get("baselines", {}).get("manifest")
    if manifest_link != {
        "path": manifest_path.as_posix(),
        "sha256": file_sha256(manifest_path),
    }:
        raise ValueError("baseline manifest link or digest mismatch")


def main() -> int:
    """Check one benchmark document against one machine artifact."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifact", type=Path, required=True)
    parser.add_argument("--document", type=Path, required=True)
    parser.add_argument("--baseline-manifest", type=Path, required=True)
    parser.add_argument("--rust-baseline", type=Path, required=True)
    parser.add_argument("--python-baseline", type=Path, required=True)
    args = parser.parse_args()
    artifact = json.loads(args.artifact.read_text(encoding="utf-8"))
    document = args.document.read_text(encoding="utf-8")
    manifest = json.loads(args.baseline_manifest.read_text(encoding="utf-8"))
    try:
        verify_document(document, artifact)
        verify_baseline_links(
            artifact,
            manifest,
            args.baseline_manifest,
            args.rust_baseline,
            args.python_baseline,
        )
    except (KeyError, TypeError, ValueError) as error:
        parser.error(str(error))
    print("Materialization benchmark documentation and tracked artifacts are consistent")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

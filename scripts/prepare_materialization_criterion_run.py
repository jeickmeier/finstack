"""Write fresh Criterion comparison provenance for materialization benchmarks."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time
from typing import Any

_PROVENANCE_OUTPUTS = {
    b"docs/MATERIALIZATION_BENCHMARKS.md",
    b"docs/materialization-benchmark-baseline.json",
    b"docs/materialization-benchmark-results.json",
}
_PROVENANCE_PATHSPECS = (
    ":(exclude)docs/MATERIALIZATION_BENCHMARKS.md",
    ":(exclude)docs/materialization-benchmark-baseline.json",
    ":(exclude)docs/materialization-benchmark-results.json",
)


def git_output(*args: str) -> bytes:
    """Return one deterministic Git command result."""
    git = shutil.which("git")
    if git is None:
        raise RuntimeError("git is required to identify the benchmark tree")
    return subprocess.check_output((git, *args))  # noqa: S603 -- resolved executable


def tree_revision() -> str:
    """Identify committed revision plus all tracked and untracked tree content."""
    head = git_output("rev-parse", "HEAD").decode().strip()
    digest = hashlib.sha256()
    digest.update(git_output("diff", "--binary", "HEAD", "--", ".", *_PROVENANCE_PATHSPECS))
    untracked = git_output("ls-files", "--others", "--exclude-standard", "-z").split(b"\0")
    for encoded_path in sorted(path for path in untracked if path and path not in _PROVENANCE_OUTPUTS):
        digest.update(encoded_path)
        digest.update(Path(encoded_path.decode()).read_bytes())
    return f"{head}-tree-{digest.hexdigest()}"


def combined_fixture_digest(fixture_a: Path, fixture_b: Path) -> str:
    """Hash both fixture byte strings with an unambiguous separator."""
    digest = hashlib.sha256()
    digest.update(fixture_a.read_bytes())
    digest.update(b"\0")
    digest.update(fixture_b.read_bytes())
    return digest.hexdigest()


def main() -> int:
    """Validate the named baseline and write current-run metadata."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--fixture-a", type=Path, required=True)
    parser.add_argument("--fixture-b", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    baseline: dict[str, Any] = json.loads(args.baseline.read_text(encoding="utf-8"))
    fixture_sha256 = combined_fixture_digest(args.fixture_a, args.fixture_b)
    if fixture_sha256 != baseline.get("fixture_sha256"):
        parser.error(
            "fixture digest differs from the named baseline; establish a new baseline "
            "instead of comparing unlike fixtures"
        )
    payload = {
        "baseline_name": baseline["baseline_name"],
        "baseline_revision": baseline["tree_revision"],
        "current_revision": tree_revision(),
        "fixture_sha256": fixture_sha256,
        "started_at_ns": time.time_ns(),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

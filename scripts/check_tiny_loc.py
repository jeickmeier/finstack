"""Report tiny source files that are candidates to collapse into neighbors.

Files at or below the line-count threshold are grouped by parent directory.
Rust files are counted after removing inline test blocks such as ``#[test]``
functions, ``#[tokio::test]`` functions, and ``#[cfg(test)]`` modules.
"""

from __future__ import annotations

import argparse
from collections import defaultdict
from dataclasses import dataclass
import os
from pathlib import Path
import re
import sys

DEFAULT_LIMIT = 25
EXCLUDED_DIRS = {
    ".git",
    ".venv",
    "__pycache__",
    "__tests__",
    "build",
    "dist",
    "generated",
    "node_modules",
    "pkg",
    "pkg-node",
    "target",
    "target-codex",
    "test",
    "tests",
    "vendor",
}
SOURCE_EXTENSIONS = {".js", ".jsx", ".py", ".rs", ".sh", ".ts", ".tsx"}
# Hand-maintained WASM declaration/API facade; size tracks the exhaustive surface/docs.
EXCLUDED_RELATIVE_PATHS: frozenset[str] = frozenset({"finstack-quant-wasm/index.d.ts"})
GLUE_BASENAMES: frozenset[str] = frozenset({"mod.rs", "lib.rs", "__init__.py"})
REEXPORT_BASENAMES: frozenset[str] = frozenset({"mod.ts", "index.ts"})
TEST_ATTRIBUTE_RE = re.compile(r"^\s*#\[\s*(?:[A-Za-z_]\w*::)*test\b[^\]]*\]\s*$")
CFG_TEST_RE = re.compile(r"^\s*#\[\s*cfg\s*\([^\]]*\btest\b[^\]]*\)\s*\]\s*$")
# Blank/comment lines plus TypeScript re-export forms.
TS_REEXPORT_LINE_RE = re.compile(
    r"^\s*(?:"
    r"//.*"
    r"|/\*.*\*/"
    r"|export\s+(?:\*|\{[^}]*\})\s+from\s+['\"][^'\"]+['\"]\s*;?"
    r"|export\s+\{[^}]*\}\s*;?"
    r")\s*$"
)


@dataclass(frozen=True, slots=True)
class Args:
    """Parsed command-line arguments for the tiny-file report."""

    limit: int


def parse_args() -> Args:
    """Parse command-line options for the tiny-file report."""
    parser = argparse.ArgumentParser(
        description=(
            "List tiny source files at or below the line-count threshold, "
            "grouped by parent directory as collapse candidates. "
            "Rust inline test blocks are excluded from the count. "
            "Always exits 0 (advisory only)."
        )
    )
    parser.add_argument("limit", nargs="?", type=int, default=DEFAULT_LIMIT)
    ns = parser.parse_args()
    return Args(limit=ns.limit)


def should_skip_dir(dirname: str) -> bool:
    """Return whether a directory should be excluded from the scan."""
    return dirname in EXCLUDED_DIRS or dirname.startswith(".")


def is_excluded_relative_path(relative_path: str) -> bool:
    """Return whether an exact repo-relative path is excluded from line counts."""
    return relative_path in EXCLUDED_RELATIVE_PATHS


def iter_source_files(root: Path) -> list[Path]:
    """Collect source files under ``root`` that count toward the limit."""
    files: list[Path] = []
    for current_root, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(name for name in dirnames if not should_skip_dir(name))
        base = Path(current_root)
        for filename in sorted(filenames):
            path = base / filename
            if path.suffix in SOURCE_EXTENSIONS:
                files.append(path)
    return files


def count_braces(line: str) -> int:
    """Return the net change in brace depth for a Rust source line."""
    return line.count("{") - line.count("}")


def is_rust_test_attribute(line: str) -> bool:
    """Return whether a line starts a Rust test-only item."""
    return bool(TEST_ATTRIBUTE_RE.match(line) or CFG_TEST_RE.match(line))


def count_rust_lines(path: Path) -> int:
    """Count Rust lines while excluding inline test-only items."""
    effective_lines = 0
    skipping_item = False
    item_started = False
    brace_depth = 0

    with path.open("r", encoding="utf-8", errors="ignore") as handle:
        for line in handle:
            if skipping_item:
                if not item_started:
                    if "{" in line:
                        item_started = True
                        brace_depth = count_braces(line)
                        if brace_depth <= 0:
                            skipping_item = False
                            item_started = False
                            brace_depth = 0
                    elif ";" in line:
                        skipping_item = False
                else:
                    brace_depth += count_braces(line)
                    if brace_depth <= 0:
                        skipping_item = False
                        item_started = False
                        brace_depth = 0
                continue

            if is_rust_test_attribute(line):
                skipping_item = True
                item_started = False
                brace_depth = 0
                continue

            effective_lines += 1

    return effective_lines


def count_lines(path: Path) -> int:
    """Count effective lines for a supported source file."""
    if path.suffix == ".rs":
        return count_rust_lines(path)

    with path.open("r", encoding="utf-8", errors="ignore") as handle:
        return sum(1 for _ in handle)


def is_pure_ts_reexport(path: Path) -> bool:
    """Return whether a TypeScript file is almost entirely re-export statements."""
    with path.open("r", encoding="utf-8", errors="ignore") as handle:
        lines = [line.rstrip("\n") for line in handle]

    if not lines:
        return False

    for line in lines:
        stripped = line.strip()
        if not stripped:
            continue
        if not TS_REEXPORT_LINE_RE.match(line):
            return False
    return True


def is_glue_file(path: Path) -> bool:
    """Return whether ``path`` is intentional module-wiring glue."""
    name = path.name
    if name in GLUE_BASENAMES:
        return True
    return name in REEXPORT_BASENAMES and is_pure_ts_reexport(path)


def collect_candidates(root: Path, limit: int) -> list[tuple[str, int]]:
    """Return tiny files with ``0 < line_count <= limit``, excluding glue."""
    candidates: list[tuple[str, int]] = []
    for path in iter_source_files(root):
        relative_path = path.relative_to(root).as_posix()
        if is_excluded_relative_path(relative_path):
            continue
        if is_glue_file(path):
            continue
        line_count = count_lines(path)
        if 0 < line_count <= limit:
            candidates.append((relative_path, line_count))
    return candidates


def group_by_parent(
    candidates: list[tuple[str, int]],
) -> list[tuple[str, list[tuple[str, int]]]]:
    """Group candidates by parent directory for collapse-cluster reporting.

    Groups are ordered by most siblings first, then total LOC desc, then path.
    Within each group, files are ordered smallest-first.
    """
    groups: dict[str, list[tuple[str, int]]] = defaultdict(list)
    for relative_path, line_count in candidates:
        parent = str(Path(relative_path).parent.as_posix())
        if parent == ".":
            parent = "."
        groups[parent].append((relative_path, line_count))

    ordered: list[tuple[str, list[tuple[str, int]]]] = []
    for parent, files in groups.items():
        files.sort(key=lambda item: (item[1], item[0]))
        ordered.append((parent, files))

    ordered.sort(key=lambda item: (-len(item[1]), -sum(count for _, count in item[1]), item[0]))
    return ordered


def format_parent_label(parent: str) -> str:
    """Return a display label for a parent directory group."""
    if parent == ".":
        return "./"
    return f"{parent}/"


def run_report(root: Path, limit: int) -> int:
    """Print the tiny-file report for ``root`` and always return 0."""
    candidates = collect_candidates(root, limit)

    if not candidates:
        print(f"No source files at or below the {limit}-line threshold.")
        return 0

    groups = group_by_parent(candidates)
    print(f"Found {len(candidates)} tiny file(s) at or below {limit} lines (collapse candidates):")
    print()

    for parent, files in groups:
        total_loc = sum(count for _, count in files)
        print(f"{format_parent_label(parent)}  ({len(files)} files, {total_loc} total LOC)")
        print(f"  {'LINES':>6}  FILE")
        print(f"  {'-----':>6}  ----")
        for relative_path, line_count in files:
            print(f"  {line_count:6d}  {relative_path}")
        print()

    return 0


def main() -> int:
    """Run the tiny-file report and print grouped candidates."""
    args = parse_args()
    root = Path(__file__).resolve().parent.parent
    return run_report(root, args.limit)


if __name__ == "__main__":
    sys.exit(main())

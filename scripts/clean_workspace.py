#!/usr/bin/env python3
"""Remove build artifacts, virtualenvs, and generated caches across the workspace.

Invoked by `mise run all-clean` after `cargo clean`. Targeted flags reclaim
incremental or WASM artifacts without a full clean.
"""

from __future__ import annotations

import argparse
from pathlib import Path
import shutil

ROOT_DIRS = (
    ".venv",
    "finstack-quant-wasm/pkg",
    "finstack-quant-wasm/pkg-node",
    "book/book",
)

GLOB_DIRS = ("__pycache__", "*.egg-info")

WASM_DIRS = (
    "finstack-quant-wasm/pkg",
    "finstack-quant-wasm/pkg-node",
    "target/wasm32-unknown-unknown",
)


def _rmtree(path: Path) -> None:
    """Remove ``path`` if it exists."""
    shutil.rmtree(path, ignore_errors=True)


def _unlink_python_extension_sos(root: Path) -> None:
    """Delete gitignored Maturin ``.so`` files under the Python package tree."""
    package_root = root / "finstack-quant-py"
    if not package_root.is_dir():
        return
    for path in package_root.rglob("*.so"):
        if path.is_file():
            path.unlink(missing_ok=True)


def clean_default(root: Path) -> None:
    """Remove virtualenvs, generated dirs, caches, and in-tree Python extensions."""
    for rel in ROOT_DIRS:
        _rmtree(root / rel)
    for pattern in GLOB_DIRS:
        for path in root.rglob(pattern):
            _rmtree(path)
    _unlink_python_extension_sos(root)
    print("Workspace cleaned.")


def clean_incremental(root: Path) -> None:
    """Delete Cargo incremental caches under every ``target`` directory."""
    removed = 0
    target = root / "target"
    if target.is_dir():
        for incremental in target.rglob("incremental"):
            if incremental.is_dir() and incremental.name == "incremental":
                _rmtree(incremental)
                removed += 1
    print(f"Removed {removed} incremental cache director{'y' if removed == 1 else 'ies'}.")


def clean_wasm(root: Path) -> None:
    """Remove wasm-pack output and the wasm32 target directory."""
    for rel in WASM_DIRS:
        _rmtree(root / rel)
    print("WASM artifacts cleaned.")


def parse_args() -> argparse.Namespace:
    """Parse CLI flags for a full or targeted clean."""
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group()
    group.add_argument(
        "--incremental",
        action="store_true",
        help="Delete Cargo incremental caches without a full cargo clean",
    )
    group.add_argument(
        "--wasm",
        action="store_true",
        help="Remove wasm-pack output and wasm32 target artifacts",
    )
    return parser.parse_args()


def main() -> None:
    """Dispatch a full workspace clean or a targeted incremental/WASM clean."""
    args = parse_args()
    root = Path.cwd()
    if args.incremental:
        clean_incremental(root)
        return
    if args.wasm:
        clean_wasm(root)
        return
    clean_default(root)


if __name__ == "__main__":
    main()

"""Generate the TypeScript barrel for Rust-owned ts-rs artifacts."""

from __future__ import annotations

import argparse
from pathlib import Path

DEFAULT_DIRECTORY = Path("finstack-quant-wasm/types/generated")


def expected_index(directory: Path) -> str:
    """Return deterministic exports for every generated TypeScript module."""
    modules = sorted(path.stem for path in directory.glob("*.ts") if path.name != "index.ts")
    return "".join(f'export * from "./{module}";\n' for module in modules)


def main() -> int:
    """Write or check the generated TypeScript barrel."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--directory", type=Path, default=DEFAULT_DIRECTORY)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    index_path = args.directory / "index.ts"
    expected = expected_index(args.directory)
    if args.check:
        return 0 if index_path.is_file() and index_path.read_text() == expected else 1
    index_path.write_text(expected)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

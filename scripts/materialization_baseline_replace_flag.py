"""Validate the explicit materialization baseline replacement environment flag."""

from __future__ import annotations

import os

VARIABLE = "FQ_REPLACE_MATERIALIZATION_BASELINE"


def replacement_requested(value: str | None) -> bool:
    """Accept only an unset flag or the exact replacement value ``1``."""
    if value is None:
        return False
    if value == "1":
        return True
    raise ValueError(f"{VARIABLE} must be unset or exactly '1'; received {value!r}")


def main() -> int:
    """Print the CLI replacement argument after strict environment validation."""
    try:
        replace = replacement_requested(os.environ.get(VARIABLE))
    except ValueError as error:
        raise SystemExit(str(error)) from error
    if replace:
        print("--replace")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

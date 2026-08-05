"""Stable repository paths for example notebooks."""

from __future__ import annotations

from pathlib import Path
from typing import Final

NOTEBOOKS_ROOT: Final = Path(__file__).resolve().parents[1]
PYTHON_PACKAGE_ROOT: Final = NOTEBOOKS_ROOT.parents[1]
REPOSITORY_ROOT: Final = PYTHON_PACKAGE_ROOT.parent

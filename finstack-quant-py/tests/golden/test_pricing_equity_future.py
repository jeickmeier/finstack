"""Listed equity future pricing goldens."""

from __future__ import annotations

import pytest

from .conftest import discover_fixtures, run_golden


@pytest.mark.parametrize("fixture", discover_fixtures("pricing/equity_future"))
def test_pricing_equity_future(fixture: str) -> None:
    """Run every listed equity future pricing fixture through the Python bindings."""
    run_golden(fixture)

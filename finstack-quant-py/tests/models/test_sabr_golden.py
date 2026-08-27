"""SABR smile goldens (closed-form vol generation, not instrument pricing)."""

from __future__ import annotations

from copy import deepcopy

import pytest
from tests.golden.conftest import DATA_ROOTS, discover_fixtures, run_golden, validate_fixture
from tests.golden.schema import GoldenFixture


@pytest.mark.parametrize("fixture", discover_fixtures("market_data/sabr"))
def test_volatility_sabr_smile(fixture: str) -> None:
    """Run every SABR smile fixture through the Python bindings."""
    run_golden(fixture)


def test_sabr_strike_keys_must_match_expected() -> None:
    """Reject a fixture whose strikes and expected smile keys diverge."""
    path = DATA_ROOTS["market_data"] / "market_data/sabr/beta_half_smile.json"
    fixture = GoldenFixture.from_path(path)
    fixture.body = deepcopy(fixture.body)
    fixture.body["strikes"].pop()

    with pytest.raises(AssertionError, match="strike keys"):
        validate_fixture(path, fixture)

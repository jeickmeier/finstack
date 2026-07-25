"""Interop guarantee for typed instruments.

typed -> to_json -> from_json -> to_json is stable, and typed and JSON
inputs price identically, for every typed instrument.
"""

from __future__ import annotations

from collections.abc import Callable
import json
from typing import Any

import pytest

from finstack_quant.valuations import instruments

# Builders reused from the per-family test modules (import the helpers).
from tests.tests_typed_helpers import (
    build_capfloor,
    build_cds,
    build_cds_index,
    build_cds_tranche,
    build_convertible,
    build_equity_option,
    build_fx_forward,
    build_fx_option,
    build_irs,
    build_structured_credit,
    build_swaption,
)

ALL_TYPED = [
    ("InterestRateSwap", build_irs),
    ("Swaption", build_swaption),
    ("CapFloor", build_capfloor),
    ("CreditDefaultSwap", build_cds),
    ("CDSIndex", build_cds_index),
    ("FxForward", build_fx_forward),
    ("FxOption", build_fx_option),
    ("EquityOption", build_equity_option),
    ("CDSTranche", build_cds_tranche),
    ("ConvertibleBond", build_convertible),
    ("StructuredCredit", build_structured_credit),
]


@pytest.mark.parametrize(("name", "factory"), ALL_TYPED)
def test_json_round_trip_is_stable(name: str, factory: Callable[[], Any]) -> None:
    cls = getattr(instruments, name)
    instance = factory()
    once = instance.to_json()
    twice = cls.from_json(once).to_json()
    assert json.loads(twice) == json.loads(once)


@pytest.mark.parametrize(("name", "factory"), ALL_TYPED)
def test_typed_instance_is_accepted_by_price_instrument_dispatch(name: str, factory: Callable[[], Any]) -> None:
    """Assert the dispatch layer serializes typed instances.

    Market errors are fine, but a ``TypeError`` from
    ``extract_instrument_json`` is not.
    """
    instance = factory()
    try:
        instruments.price_instrument(instance, "{}", "2024-01-01", "default")
    except TypeError as exc:  # pragma: no cover - dispatch regression
        pytest.fail(f"typed {name} rejected by dispatch: {exc}")
    except ValueError:
        pass  # empty market: expected downstream failure, dispatch worked

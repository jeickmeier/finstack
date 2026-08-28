"""Instrument pricing and risk metrics.

Bindings for the ``finstack-quant-valuations`` Rust crate.

Examples:
--------
>>> from finstack_quant.valuations import instruments
>>> hasattr(instruments, "price_instrument")
True

"""

import json as _json
import sys as _sys
from typing import TYPE_CHECKING as _TYPE_CHECKING, Any as _Any

from finstack_quant.finstack_quant import valuations as _valuations
from finstack_quant.valuations import (
    composite as composite,
    credit_derivatives as credit_derivatives,
    instruments as instruments,
    market as market,
)

if _TYPE_CHECKING:
    import pandas as pd

ValuationResult = _valuations.ValuationResult
tarn_coupon_profile = _valuations.tarn_coupon_profile
snowball_coupon_profile = _valuations.snowball_coupon_profile
inverse_floater_coupon_profile = _valuations.inverse_floater_coupon_profile
cms_spread_option_intrinsic = _valuations.cms_spread_option_intrinsic
callable_range_accrual_accrued = _valuations.callable_range_accrual_accrued
# `schema` is a compiled submodule with no pure-Python shim package, so alias it
# onto the public dotted path that `import finstack_quant.valuations.schema` uses.
schema = _valuations.schema
_sys.modules.setdefault("finstack_quant.valuations.schema", schema)


def instrument_cashflows(
    instrument_json: str,
    market: _Any,
    as_of: str,
    *,
    model: str,
) -> tuple[dict, "pd.DataFrame"]:
    """Per-flow DF / survival / PV DataFrame for a discountable instrument.

    Supports ``model in {"discounting", "hazard_rate"}``. The returned
    ``envelope["total_pv"]`` reconciles with the instrument's ``base_value``
    for the supported model-instrument pairs.

    Args:
        instrument_json: Canonical ``finstack_quant.instrument/1`` envelope.
        market: ``MarketContext`` instance or JSON string.
        as_of: ISO 8601 valuation date.
        model: ``"discounting"`` (DF only) or ``"hazard_rate"`` (adds survival
            probability, conditional default probability, and recovery-adjusted
            principal PV). ``"default"`` is not accepted.

    Returns:
        ``(envelope, df)`` where ``envelope`` is the parsed JSON dict and
        ``df`` is a ``pandas.DataFrame`` of the per-flow rows with ``date``
        / ``reset_date`` parsed as ``datetime64``.

    Raises:
        ValueError: If ``model`` is unsupported or the instrument type isn't
            priced under that model.

    Examples:
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.dates import StubKind
    >>> from finstack_quant.core.market_data import DiscountCurve, MarketContext
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.core.types import Rate
    >>> from finstack_quant.valuations.instruments import Bond
    >>> as_of = datetime.date(2024, 1, 1)
    >>> bond = Bond.fixed(
    ...     "B", Money(1000.0, Currency("USD")), Rate(0.05), as_of, datetime.date(2026, 1, 1), StubKind.NONE, "USD-OIS"
    ... )
    >>> market = MarketContext().insert(DiscountCurve.flat("USD-OIS", as_of, 0.04))
    >>> from finstack_quant.valuations import instrument_cashflows
    >>> header, frame = instrument_cashflows(bond.to_json(), market, "2024-01-01", model="discounting")
    >>> (header["instrument_id"], len(frame))
    ('B', 6)

    """
    import pandas as pd

    payload = instruments.instrument_cashflows_json(instrument_json, market, as_of, model)
    envelope = _json.loads(payload)
    df = pd.DataFrame(envelope["flows"])
    if not df.empty:
        df["date"] = pd.to_datetime(df["date"])
        if "reset_date" in df.columns:
            df["reset_date"] = pd.to_datetime(df["reset_date"])
    return envelope, df


__all__: list[str] = [
    "ValuationResult",
    "callable_range_accrual_accrued",
    "cms_spread_option_intrinsic",
    "composite",
    "credit_derivatives",
    "instrument_cashflows",
    "instruments",
    "inverse_floater_coupon_profile",
    "market",
    "schema",
    "snowball_coupon_profile",
    "tarn_coupon_profile",
]

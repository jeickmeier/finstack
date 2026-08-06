"""
Cashflow schedule JSON construction and validation.

JSON-first bindings for ``finstack-quant-cashflows``. Build schedules from a
``CashflowScheduleBuildSpec``, validate canonical payloads, extract dated flows,
and compute accrued interest.

Examples
--------
>>> import datetime
>>> from finstack_quant.cashflows.primitives import CFKind, CashFlow
>>> from finstack_quant.core.money import Money
>>> CashFlow(datetime.date(2025, 6, 15), Money(100.0, "USD"), CFKind.FIXED).amount.amount
100.0

"""

from __future__ import annotations

from finstack_quant.cashflows import accrual as accrual
from finstack_quant.cashflows import aggregation as aggregation
from finstack_quant.cashflows import builder as builder
from finstack_quant.cashflows import primitives as primitives
from finstack_quant.cashflows import schema as schema

__all__ = [
    "accrual",
    "accrued_interest_json",
    "aggregation",
    "build_cashflow_schedule_json",
    "builder",
    "dated_flows_json",
    "primitives",
    "schema",
    "validate_cashflow_schedule_json",
]

def build_cashflow_schedule_json(spec_json: str, market_json: str | None = None) -> str:
    """
    Build a cashflow schedule from a JSON spec and return canonical schedule JSON.

    Parameters
    ----------
    spec_json : str
        JSON-encoded ``CashflowScheduleBuildSpec`` with canonical
        ``coupon_program`` and ``payment_program`` instructions, principal,
        fees, and schedule rules.
    market_json : str, optional
        JSON-encoded ``MarketContext`` for floating-rate index lookups. Omit
        when the schedule uses fixed coupons only.

    Returns
    -------
    str
        Canonical JSON-encoded ``CashFlowSchedule``.

    Raises
    ------
    ValueError
        If ``spec_json`` (or ``market_json`` when supplied) fails schema or
        semantic validation.
    KeyError
        If required market data or a fixing series is missing.

    Examples
    --------
    >>> import json
    >>> spec = {
    ...     "notional": {"initial": {"amount": "1000000", "currency": "USD"}, "amort": "none"},
    ...     "issue": "2024-08-31",
    ...     "maturity": "2025-08-31",
    ...     "coupon_program": [
    ...         {
    ...             "kind": "fixed",
    ...             "spec": {
    ...                 "coupon_type": "cash",
    ...                 "rate": "0.06",
    ...                 "frequency": {"count": 12, "unit": "months"},
    ...                 "day_count": "30_360",
    ...                 "business_day_convention": "following",
    ...                 "calendar_id": "weekends_only",
    ...                 "stub": "none",
    ...                 "end_of_month": False,
    ...                 "payment_lag_days": 0,
    ...             },
    ...         }
    ...     ],
    ... }
    >>> from finstack_quant.cashflows import build_cashflow_schedule_json
    >>> schedule_json = build_cashflow_schedule_json(json.dumps(spec))
    >>> json.loads(schedule_json)["meta"]["issue_date"]
    '2024-08-31'

    """

def validate_cashflow_schedule_json(schedule_json: str) -> str:
    """
    Validate and canonicalize a ``CashFlowSchedule`` JSON payload.

    Parameters
    ----------
    schedule_json : str
        JSON-encoded ``CashFlowSchedule``.

    Returns
    -------
    str
        Canonical re-serialized schedule JSON.

    Raises
    ------
    ValueError
        If ``schedule_json`` is malformed or fails validation.

    Examples
    --------
    >>> import datetime
    >>> from decimal import Decimal
    >>> from finstack_quant.cashflows.builder import CashFlowSchedule, FixedCouponSpec, ScheduleParams
    >>> from finstack_quant.core.money import Money
    >>> schedule = (
    ...     CashFlowSchedule
    ...     .builder()
    ...     .principal(Money(1_000_000.0, "USD"), datetime.date(2025, 1, 15), datetime.date(2026, 1, 15))
    ...     .fixed_cf(FixedCouponSpec(rate=Decimal("0.05"), schedule=ScheduleParams.semiannual_30360()))
    ...     .build()
    ... )
    >>> import json
    >>> from finstack_quant.cashflows import validate_cashflow_schedule_json
    >>> json.loads(validate_cashflow_schedule_json(schedule.to_json()))["meta"]["issue_date"]
    '2025-01-15'

    """

def dated_flows_json(schedule_json: str) -> str:
    """
    Extract settlement-dated cashflows from a schedule as a compact JSON array.

    Parameters
    ----------
    schedule_json : str
        JSON-encoded ``CashFlowSchedule``.

    Returns
    -------
    str
        JSON array of settlement cash entries. ``PIK`` and
        ``DefaultedNotional`` state rows are omitted; parse the full schedule
        JSON when flow classification is required.

    Raises
    ------
    ValueError
        If ``schedule_json`` is invalid.

    Examples
    --------
    >>> import datetime
    >>> from decimal import Decimal
    >>> from finstack_quant.cashflows.builder import CashFlowSchedule, FixedCouponSpec, ScheduleParams
    >>> from finstack_quant.core.money import Money
    >>> schedule = (
    ...     CashFlowSchedule
    ...     .builder()
    ...     .principal(Money(1_000_000.0, "USD"), datetime.date(2025, 1, 15), datetime.date(2026, 1, 15))
    ...     .fixed_cf(FixedCouponSpec(rate=Decimal("0.05"), schedule=ScheduleParams.semiannual_30360()))
    ...     .build()
    ... )
    >>> import json
    >>> from finstack_quant.cashflows import dated_flows_json
    >>> len(json.loads(dated_flows_json(schedule.to_json()))) == len(schedule.get_flows())
    True

    """

def accrued_interest_json(schedule_json: str, as_of: str, config_json: str | None = None) -> float:
    """
    Compute accrued interest for a schedule as of a valuation date.

    Parameters
    ----------
    schedule_json : str
        JSON-encoded ``CashFlowSchedule``.
    as_of : str
        Accrual snapshot date in ISO 8601 ``YYYY-MM-DD`` form.
    config_json : str, optional
        JSON-encoded ``AccrualConfig`` overriding default accrual conventions.

    Returns
    -------
    float
        Accrued interest in the schedule settlement currency. The Rust engine
        computes from the canonical schedule and crosses the binding boundary as
        ``f64``; for large notionals, compare with an absolute tolerance scaled
        to the schedule notional rather than expecting decimal-string equality.
        Returns ``0.0`` when ``as_of`` is outside all coupon periods.

    Raises
    ------
    ValueError
        If the schedule JSON or accrual configuration is invalid.
    KeyError
        If an ex-coupon calendar is unknown.

    Examples
    --------
    >>> import datetime
    >>> from decimal import Decimal
    >>> from finstack_quant.cashflows.builder import CashFlowSchedule, FixedCouponSpec, ScheduleParams
    >>> from finstack_quant.core.money import Money
    >>> schedule = (
    ...     CashFlowSchedule
    ...     .builder()
    ...     .principal(Money(1_000_000.0, "USD"), datetime.date(2025, 1, 15), datetime.date(2026, 1, 15))
    ...     .fixed_cf(FixedCouponSpec(rate=Decimal("0.05"), schedule=ScheduleParams.semiannual_30360()))
    ...     .build()
    ... )
    >>> from finstack_quant.cashflows import accrued_interest_json
    >>> accrued_interest_json(schedule.to_json(), "2025-04-15") > 0.0
    True

    """

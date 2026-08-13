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

import datetime

from finstack_quant.cashflows import accrual as accrual
from finstack_quant.cashflows import aggregation as aggregation
from finstack_quant.cashflows import builder as builder
from finstack_quant.cashflows import primitives as primitives
from finstack_quant.cashflows import schema as schema

__all__ = [
    "accrual",
    "accrued_interest",
    "aggregation",
    "build_cashflow_schedule_json",
    "builder",
    "cdr_to_mdr",
    "cpr_to_smm",
    "dated_flows_json",
    "mdr_to_cdr",
    "primitives",
    "schema",
    "smm_to_cpr",
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

def accrued_interest(schedule_json: str, as_of: datetime.date | str, config_json: str | None = None) -> float:
    """
    Compute accrued interest for a schedule as of a valuation date.

    Parameters
    ----------
    schedule_json : str
        JSON-encoded ``CashFlowSchedule``.
    as_of : datetime.date | str
        Accrual snapshot date, either a date-like object or an ISO 8601 string.
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
    >>> from finstack_quant.cashflows import accrued_interest
    >>> accrued_interest(schedule.to_json(), "2025-04-15") > 0.0
    True

    """

def cpr_to_smm(cpr: float) -> float:
    """
    Convert an annual CPR (constant prepayment rate) to a monthly SMM.

    Flat re-export of :func:`finstack_quant.cashflows.builder.cpr_to_smm`,
    mirroring the Rust crate-root re-export. Uses
    ``SMM = 1 - (1 - CPR)^(1/12)``.

    Parameters
    ----------
    cpr : float
        Annualized CPR as a decimal in ``[0, 1]`` (``0.06`` means 6%).

    Returns
    -------
    float
        Monthly SMM as a decimal fraction.

    Raises
    ------
    ValueError
        If ``cpr`` is negative, non-finite, or above ``1.0``.

    Examples
    --------
    >>> from finstack_quant.cashflows import cpr_to_smm
    >>> round(cpr_to_smm(0.06), 6)
    0.005143
    """
    ...

def smm_to_cpr(smm: float) -> float:
    """
    Convert a monthly SMM (single monthly mortality) to an annual CPR.

    Flat re-export of :func:`finstack_quant.cashflows.builder.smm_to_cpr`,
    mirroring the Rust crate-root re-export. Uses ``CPR = 1 - (1 - SMM)^12``.

    Parameters
    ----------
    smm : float
        Monthly SMM as a decimal in ``[0, 1]``.

    Returns
    -------
    float
        Annualized CPR as a decimal fraction.

    Raises
    ------
    ValueError
        If ``smm`` is negative, non-finite, or above ``1.0``.

    Examples
    --------
    >>> from finstack_quant.cashflows import cpr_to_smm, smm_to_cpr
    >>> round(smm_to_cpr(cpr_to_smm(0.06)), 10)
    0.06
    """
    ...

def cdr_to_mdr(cdr: float) -> float:
    """
    Convert an annual CDR (constant default rate) to a monthly MDR.

    Flat re-export of :func:`finstack_quant.cashflows.builder.cdr_to_mdr`,
    mirroring the Rust crate-root re-export. Default and prepayment mortality
    rates share the same kernel: ``MDR = 1 - (1 - CDR)^(1/12)``.

    Parameters
    ----------
    cdr : float
        Constant annual default rate as a decimal in ``[0, 1]``.

    Returns
    -------
    float
        Monthly MDR as a decimal fraction.

    Raises
    ------
    ValueError
        If ``cdr`` is negative, non-finite, or above ``1.0``.

    Examples
    --------
    >>> from finstack_quant.cashflows import cdr_to_mdr
    >>> round(cdr_to_mdr(0.02), 6)
    0.001682
    """
    ...

def mdr_to_cdr(mdr: float) -> float:
    """
    Convert a monthly MDR (monthly default rate) to an annual CDR.

    Flat re-export of :func:`finstack_quant.cashflows.builder.mdr_to_cdr`,
    mirroring the Rust crate-root re-export. Uses ``CDR = 1 - (1 - MDR)^12``.

    Parameters
    ----------
    mdr : float
        Monthly default rate as a decimal in ``[0, 1]``.

    Returns
    -------
    float
        Annualized CDR as a decimal fraction.

    Raises
    ------
    ValueError
        If ``mdr`` is negative, non-finite, or above ``1.0``.

    Examples
    --------
    >>> from finstack_quant.cashflows import cdr_to_mdr, mdr_to_cdr
    >>> round(mdr_to_cdr(cdr_to_mdr(0.02)), 10)
    0.02
    """
    ...

"""
Currency-preserving aggregation of dated cashflows into periods and totals.

Typed bindings for ``finstack_quant_cashflows::aggregation``.
:func:`aggregate_by_period` groups dated flows into reporting periods while
preserving currency separation; :func:`aggregate_cashflows_checked` sums flows
into a single currency, rejecting mismatches; and :func:`calendar_year_ladder`
rolls coupon / principal / PV totals by calendar year.

Examples
--------
>>> import datetime
>>> from finstack_quant.cashflows.aggregation import aggregate_by_period
>>> from finstack_quant.core.dates import build_periods
>>> from finstack_quant.core.money import Money
>>> flows = [(datetime.date(2025, 1, 1), Money(10.0, "USD")), (datetime.date(2025, 1, 2), Money(5.0, "USD"))]
>>> aggregate_by_period(flows, build_periods("2025M01..M01").periods)["2025M01"]["USD"].amount
15.0

"""

from __future__ import annotations

import datetime

from finstack_quant.core.currency import Currency
from finstack_quant.core.dates import Period
from finstack_quant.core.money import Money

__all__ = [
    "aggregate_by_period",
    "aggregate_cashflows_checked",
    "calendar_year_ladder",
]

def aggregate_by_period(
    flows: list[tuple[datetime.date, Money]],
    periods: list[Period],
) -> dict[str, dict[str, Money]]:
    """
    Aggregate dated flows into reporting periods (half-open ``[start, end)``).

    Parameters
    ----------
    flows : list[tuple[datetime.date, Money]]
        Dated cashflows; unsorted input is sorted internally.
    periods : list[Period]
        Sorted, disjoint reporting periods.

    Returns
    -------
    dict[str, dict[str, Money]]
        ``{period_id_label: {currency_code: nominal_sum}}``; periods with no
        flows are omitted from the result.

    Raises
    ------
    ValueError
        If periods are unsorted, overlapping, or contain duplicate ids, or
        if a per-currency total is non-finite or exceeds the Decimal range.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.cashflows.aggregation import aggregate_by_period
    >>> from finstack_quant.core.dates import build_periods
    >>> from finstack_quant.core.money import Money
    >>> flows = [(datetime.date(2025, 3, 15), Money(100.0, "USD"))]
    >>> periods = build_periods("2025Q1..Q4").periods
    >>> aggregate_by_period(flows, periods)["2025Q1"]["USD"].amount
    100.0
    """
    ...

def aggregate_cashflows_checked(
    flows: list[tuple[datetime.date, Money]],
    target: Currency | str,
) -> Money:
    """
    Currency-checked single-currency aggregation with an explicit target currency.

    Parameters
    ----------
    flows : list[tuple[datetime.date, Money]]
        Dated cashflows; every flow must be in ``target`` currency.
    target : Currency or str
        Required currency for every flow and the returned total.

    Returns
    -------
    Money
        Single total in ``target`` currency. Empty ``flows`` returns a zero
        total.

    Raises
    ------
    ValueError
        If any flow currency differs from ``target``; currency mismatches
        are rejected, never silently converted.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.cashflows.aggregation import aggregate_cashflows_checked
    >>> from finstack_quant.core.money import Money
    >>> flows = [(datetime.date(2025, 1, 15), Money(50_000.0, "USD"))]
    >>> aggregate_cashflows_checked(flows, "USD").amount
    50000.0
    """
    ...

def calendar_year_ladder(
    dates: list[datetime.date],
    kinds: list[str],
    amounts: list[float],
    pvs: list[float],
) -> list[tuple[int, float, float, float]]:
    """
    Group dated cashflows into a calendar-year coupon / principal / PV ladder.

    Parameters
    ----------
    dates : list[datetime.date]
        Payment dates; the Gregorian year of each date is the bucket.
    kinds : list[str]
        Cashflow kind labels (``"fixed"``, ``"notional"``, ``"coupon"``,
        ``"principal"``, …). ASCII case is ignored. Unknown labels are treated
        as coupon (non-principal).
    amounts : list[float]
        Signed cashflow amounts, one per date, in native currency units.
    pvs : list[float]
        Present values, one per date, in the same units as ``amounts``.

    Returns
    -------
    list[tuple[int, float, float, float]]
        One ``(year, coupon, principal, pv)`` row per calendar year, sorted
        by year.

    Raises
    ------
    ValueError
        If the four lists have different lengths.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.cashflows.aggregation import calendar_year_ladder
    >>> calendar_year_ladder(
    ...     [datetime.date(2027, 3, 15), datetime.date(2034, 3, 15)],
    ...     ["coupon", "principal"],
    ...     [100.0, 1000.0],
    ...     [90.0, 700.0],
    ... )
    [(2027, 100.0, 0.0, 90.0), (2034, 0.0, 1000.0, 700.0)]
    """
    ...

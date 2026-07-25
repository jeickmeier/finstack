"""
Currency-preserving aggregation of dated cashflows into periods and totals.

Typed bindings for ``finstack_quant_cashflows::aggregation``. Both functions
operate on ``[(date, Money), ...]`` dated flows: :func:`aggregate_by_period`
groups flows into reporting periods while preserving currency separation,
and :func:`aggregate_cashflows_checked` sums flows into a single currency,
rejecting any flow whose currency does not match the target.

Examples
--------
>>> import finstack_quant.cashflows.aggregation as aggregation
>>> aggregation.__name__
'finstack_quant.cashflows.aggregation'
"""

from __future__ import annotations

import datetime

from finstack_quant.core.currency import Currency
from finstack_quant.core.dates import Period
from finstack_quant.core.money import Money

__all__ = [
    "aggregate_by_period",
    "aggregate_cashflows_checked",
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

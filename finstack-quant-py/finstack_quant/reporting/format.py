"""Deterministic value formatters for rendered reports.

All helpers return display strings; non-finite / missing inputs render as the
em-interpunct placeholder ``"·"`` so tables never show ``nan``.

Examples:
--------
>>> from finstack_quant.reporting.format import money, pct
>>> pct(12.34), money(1250, "USD")
('12.3%', '1,250.00 USD')
"""

from __future__ import annotations

import html
import math
from typing import Any

__all__ = [
    "fmt_date",
    "money",
    "pct",
    "ratio",
    "sign_class",
]

_PLACEHOLDER = "·"


def _escape_html(x: Any) -> str:
    return html.escape(str(x))


def _dates_of(df: Any) -> list[Any]:
    return [ix.date() if hasattr(ix, "date") else ix for ix in df.index]


def _missing(x: Any) -> bool:
    if x is None:
        return True
    try:
        return isinstance(x, float) and math.isnan(x)
    except TypeError:
        return False


def pct(x: Any, dp: int = 1, signed: bool = False) -> str:
    """Format a value already expressed in percentage points.

    Parameters
    ----------
    x : Any
        Percentage-point value, such as ``13.2`` for ``13.2%``; missing values
        render as the report placeholder.
    dp : int
        Number of digits after the decimal point; defaults to one.
    signed : bool
        Whether non-negative values receive an explicit ``+`` sign.

    Returns:
    -------
    str
        Percentage-point value with the requested precision and sign convention.

    Notes:
    -----
    This helper does not raise; missing or non-finite inputs render as ``·``.

    Examples:
    --------
    >>> from finstack_quant.reporting.format import pct
    >>> pct(13.25, dp=2, signed=True)
    '+13.25%'
    """
    if _missing(x):
        return _PLACEHOLDER
    sign = "+" if (signed and x >= 0) else ""
    return f"{sign}{x:.{dp}f}%"


def ratio(x: Any, dp: int = 2) -> str:
    """Format a unitless ratio such as a Sharpe ratio.

    Parameters
    ----------
    x : Any
        Unitless numeric ratio; missing values render as the report placeholder.
    dp : int
        Number of digits after the decimal point; defaults to two.

    Returns:
    -------
    str
        Unitless ratio rounded to the requested decimal precision.

    Notes:
    -----
    This helper does not raise; missing or non-finite inputs render as ``·``.

    Examples:
    --------
    >>> from finstack_quant.reporting.format import ratio
    >>> ratio(1.234)
    '1.23'
    """
    if _missing(x):
        return _PLACEHOLDER
    return f"{x:.{dp}f}"


def money(amount: Any, currency: str | None = None, dp: int = 2) -> str:
    """Format a monetary amount with grouping and an optional currency code.

    Parameters
    ----------
    amount : Any
        Monetary amount in the display currency; missing values use the placeholder.
    currency : str or None
        Optional ISO currency code appended after the formatted amount.
    dp : int
        Number of digits after the decimal point; defaults to two.

    Returns:
    -------
    str
        Grouped amount followed by the currency code when one is supplied.

    Notes:
    -----
    This helper does not raise; missing or non-finite inputs render as ``·``.

    Examples:
    --------
    >>> from finstack_quant.reporting.format import money
    >>> money(1250, "USD")
    '1,250.00 USD'
    """
    if _missing(amount):
        return _PLACEHOLDER
    body = f"{amount:,.{dp}f}"
    return f"{body} {currency}" if currency else body


def sign_class(x: Any) -> str:
    """Return a sign CSS class for a numeric value.

    Parameters
    ----------
    x : Any
        Numeric value whose sign selects ``"pos"``, ``"neg"``, or no class.

    Returns:
    -------
    str
        ``"pos"`` for positive values, ``"neg"`` for negative values, otherwise ``""``.

    Notes:
    -----
    This helper does not raise; missing or zero values yield an empty class.

    Examples:
    --------
    >>> from finstack_quant.reporting.format import sign_class
    >>> sign_class(-0.5)
    'neg'
    """
    if _missing(x) or x == 0:
        return ""
    return "pos" if x > 0 else "neg"


def fmt_date(d: Any) -> str:
    """Format a date-like object as ``"19 Jun 2026"``.

    Parameters
    ----------
    d : Any
        Date-like object with ``strftime`` or a value convertible to string.

    Returns:
    -------
    str
        Human-readable day, abbreviated month, and four-digit year.

    Notes:
    -----
    This helper does not raise; values without ``strftime`` are converted with ``str``.

    Examples:
    --------
    >>> from datetime import date
    >>> from finstack_quant.reporting.format import fmt_date
    >>> fmt_date(date(2026, 6, 19))
    '19 Jun 2026'
    """
    if _missing(d):
        return _PLACEHOLDER
    if hasattr(d, "strftime"):
        return d.strftime("%d %b %Y").lstrip("0").replace(" 0", " ")
    return str(d)

"""
Volatility surface arbitrage detection.

Model-free static checks for butterfly arbitrage (Durrleman density),
calendar-spread arbitrage (total-variance monotonicity), and Dupire local-vol
density positivity on an implied-volatility grid. Inputs are flat arrays so
callers can pass numpy-friendly grids without first constructing a
``VolSurface`` wrapper.

Examples
--------
>>> from finstack_quant.core.market_data.arbitrage import check_surface_grid
>>> strikes, expiries = [90.0, 100.0, 110.0], [1.0, 2.0]
>>> vols, forwards = [[0.2, 0.2, 0.2], [0.2, 0.2, 0.2]], [100.0, 100.0]
>>> check_surface_grid(strikes, expiries, vols, forwards)["passed"]
True

"""

from __future__ import annotations

from typing import Literal, Optional, TypedDict

class _ArbitrageViolation(TypedDict):
    type: Literal[
        "butterfly",
        "calendar_spread",
        "local_vol_density",
        "svi_moment_bound",
        "svi_butterfly_condition",
        "svi_calendar_spread",
    ]
    severity: Literal["negligible", "minor", "major", "critical"]
    strike: float
    expiry: float
    adjacent_expiry: float | None
    magnitude: float
    value: float
    message: str
    description: str

class _ArbitrageReport(TypedDict):
    total_violations: int
    passed: bool
    by_severity: dict[Literal["negligible", "minor", "major", "critical"], int]
    by_type: dict[Literal["butterfly", "calendar_spread", "local_vol_density"], int]
    violations: list[_ArbitrageViolation]
    elapsed_us: int

__all__ = [
    "check_butterfly_grid",
    "check_calendar_spread_grid",
    "check_local_vol_density_grid",
    "check_surface_grid",
]

def check_butterfly_grid(
    strikes: list[float],
    expiries: list[float],
    vols: list[list[float]],
    forward_prices: list[float],
    tolerance: float = 1e-6,
) -> list[_ArbitrageViolation]:
    """
    Check butterfly arbitrage via Durrleman's ``g(k)`` density condition.

    Parameters
    ----------
    strikes : list[float]
        Monotonically increasing strike grid.
    expiries : list[float]
        Monotonically increasing expiry grid in years.
    vols : list[list[float]]
        Implied volatilities shaped ``[n_expiries][n_strikes]`` (decimal, e.g.
        ``0.20`` for 20%).
    forward_prices : list[float]
        Forward prices per expiry, or a single value broadcast across expiries.
    tolerance : float, optional
        Tolerance in total-variance units. Default ``1e-6``.

    Returns
    -------
    list[_ArbitrageViolation]
        One dict per violation with keys ``type``, ``severity``, ``strike``,
        ``expiry``, ``adjacent_expiry``, ``magnitude``, ``value``, ``message``,
        and ``description``.

    Raises
    ------
    ValueError
        If grid dimensions are inconsistent or inputs are non-finite.

    Sources
    -------
    See ``docs/REFERENCES.md#dupire-1994`` for local-volatility density context.

    Examples
    --------
    >>> from finstack_quant.core.market_data.arbitrage import check_butterfly_grid
    >>> strikes, expiries = [90.0, 100.0, 110.0], [1.0, 2.0]
    >>> vols, forwards = [[0.2, 0.2, 0.2], [0.2, 0.2, 0.2]], [100.0, 100.0]
    >>> check_butterfly_grid(strikes, expiries, vols, forwards)
    []

    """
    ...

def check_calendar_spread_grid(
    strikes: list[float],
    expiries: list[float],
    vols: list[list[float]],
    forward_prices: list[float],
    tolerance: float = 1e-6,
) -> list[_ArbitrageViolation]:
    """
    Check calendar-spread arbitrage (total-variance monotonicity in log-moneyness).

    Parameters
    ----------
    strikes : list[float]
        Monotonically increasing strike grid.
    expiries : list[float]
        Monotonically increasing expiry grid in years.
    vols : list[list[float]]
        Implied vols shaped ``[n_expiries][n_strikes]``.
    forward_prices : list[float]
        Forward prices per expiry or one broadcast value.
    tolerance : float, optional
        Tolerance in total-variance units. Default ``1e-6``.

    Returns
    -------
    list[_ArbitrageViolation]
        Violation dicts with the same schema as :func:`check_butterfly_grid`.

    Raises
    ------
    ValueError
        If grid dimensions are inconsistent or inputs are non-finite.

    Examples
    --------
    >>> from finstack_quant.core.market_data.arbitrage import check_calendar_spread_grid
    >>> strikes, expiries = [90.0, 100.0, 110.0], [1.0, 2.0]
    >>> vols, forwards = [[0.2, 0.2, 0.2], [0.2, 0.2, 0.2]], [100.0, 100.0]
    >>> check_calendar_spread_grid(strikes, expiries, vols, forwards)
    []

    """
    ...

def check_local_vol_density_grid(
    strikes: list[float],
    expiries: list[float],
    vols: list[list[float]],
    forward_prices: list[float],
) -> list[_ArbitrageViolation]:
    """
    Check Dupire local-volatility density positivity on the implied-vol grid.

    Parameters
    ----------
    strikes : list[float]
        Monotonically increasing strike grid.
    expiries : list[float]
        Monotonically increasing expiry grid in years.
    vols : list[list[float]]
        Implied vols shaped ``[n_expiries][n_strikes]``.
    forward_prices : list[float]
        Forward prices per expiry or one broadcast value.

    Returns
    -------
    list[_ArbitrageViolation]
        Violation dicts with the same schema as :func:`check_butterfly_grid`.

    Raises
    ------
    ValueError
        If grid dimensions are inconsistent or inputs are non-finite.

    Sources
    -------
    See ``docs/REFERENCES.md#dupire-1994``.

    Examples
    --------
    >>> from finstack_quant.core.market_data.arbitrage import check_local_vol_density_grid
    >>> strikes, expiries = [90.0, 100.0, 110.0], [1.0, 2.0]
    >>> vols, forwards = [[0.2, 0.2, 0.2], [0.2, 0.2, 0.2]], [100.0, 100.0]
    >>> check_local_vol_density_grid(strikes, expiries, vols, forwards)
    []

    """
    ...

def check_surface_grid(
    strikes: list[float],
    expiries: list[float],
    vols: list[list[float]],
    forward_prices: list[float],
    tolerance: float = 1e-6,
) -> _ArbitrageReport:
    """
    Run butterfly, calendar-spread, and local-vol density checks together.

    Parameters
    ----------
    strikes : list[float]
        Monotonically increasing strike grid.
    expiries : list[float]
        Monotonically increasing expiry grid in years.
    vols : list[list[float]]
        Implied vols shaped ``[n_expiries][n_strikes]``.
    forward_prices : list[float]
        One forward price to broadcast or one price per expiry.
    tolerance : float, optional
        Tolerance in total-variance units. Default ``1e-6``.

    Returns
    -------
    _ArbitrageReport
        Aggregate report with ``total_violations``, ``passed``,
        ``by_severity``, ``by_type``, ``violations``, and ``elapsed_us``.

    Raises
    ------
    ValueError
        If the forward-price shape or grid inputs are invalid.

    Examples
    --------
    >>> from finstack_quant.core.market_data.arbitrage import check_surface_grid
    >>> strikes, expiries = [90.0, 100.0, 110.0], [1.0, 2.0]
    >>> vols, forwards = [[0.2, 0.2, 0.2], [0.2, 0.2, 0.2]], [100.0, 100.0]
    >>> check_surface_grid(strikes, expiries, vols, forwards)["passed"]
    True

    """
    ...

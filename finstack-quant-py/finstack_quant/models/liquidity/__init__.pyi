"""Product-independent liquidity risk and market-impact models.

Examples
--------
>>> from finstack_quant.models.liquidity import days_to_liquidate
>>> days_to_liquidate(1_000_000, 250_000, 0.20)
20.0
"""

from __future__ import annotations

__all__ = [
    "almgren_chriss_impact",
    "amihud_illiquidity",
    "days_to_liquidate",
    "kyle_lambda",
    "liquidity_tier",
    "lvar_bangia",
    "roll_effective_spread",
]

def roll_effective_spread(returns: list[float]) -> float | None:
    """Estimate Roll effective spread from an ordered return series.

    Parameters
    ----------
    returns : list[float]
        Log or arithmetic decimal returns in time order; at least two values
        are required.

    Returns
    -------
    float | None
        Effective spread in return units, or ``None`` when the sample is too
        short or its serial covariance is non-negative.

    Notes
    -----
    This helper does not raise; unavailable estimates return ``None``.

    Examples
    --------
    >>> from finstack_quant.models.liquidity import roll_effective_spread
    >>> roll_effective_spread([0.01, -0.01, 0.01, -0.01])
    0.02
    """
    ...

def amihud_illiquidity(returns: list[float], volumes: list[float]) -> float | None:
    """Compute Amihud illiquidity from aligned returns and volumes.

    Parameters
    ----------
    returns : list[float]
        Ordered decimal period returns.
    volumes : list[float]
        Strictly positive traded volumes aligned with ``returns``.

    Returns
    -------
    float | None
        Mean ``abs(return) / volume``, or ``None`` for empty, mismatched,
        non-finite, or non-positive-volume inputs.

    Notes
    -----
    This helper does not raise; unavailable estimates return ``None``.

    Examples
    --------
    >>> from finstack_quant.models.liquidity import amihud_illiquidity
    >>> amihud_illiquidity([0.01, 0.02], [100.0, 200.0])
    0.0001
    """
    ...

def days_to_liquidate(
    position_quantity: float,
    adv: float,
    participation_rate: float,
) -> float:
    """Estimate a position's liquidation horizon in trading days.

    Parameters
    ----------
    position_quantity : float
        Shares or contracts to liquidate; the absolute value is used.
    adv : float
        Average daily volume in the same share or contract units.
    participation_rate : float
        Fraction of ADV available per trading day.

    Returns
    -------
    float
        ``abs(position_quantity) / (adv * participation_rate)``, or infinity
        when ADV or participation is non-positive.

    Notes
    -----
    This helper does not raise; invalid capacity produces infinity.

    Examples
    --------
    >>> from finstack_quant.models.liquidity import days_to_liquidate
    >>> days_to_liquidate(1_000_000, 250_000, 0.20)
    20.0
    """
    ...

def liquidity_tier(days_to_liquidate: float) -> str:
    """Classify a liquidation horizon using the default model thresholds.

    Parameters
    ----------
    days_to_liquidate : float
        Estimated unwind horizon in trading days.

    Returns
    -------
    str
        One of ``"tier1"`` through ``"tier5"`` using thresholds 1, 5, 20,
        and 60 trading days, with Tier 1 the most liquid.

    Notes
    -----
    This helper does not raise; every horizon maps to a tier label.

    Examples
    --------
    >>> from finstack_quant.models.liquidity import liquidity_tier
    >>> liquidity_tier(3.0)
    'tier2'
    """
    ...

def lvar_bangia(
    var: float,
    spread_mean: float,
    spread_vol: float,
    confidence: float,
    position_value: float,
) -> dict[str, float]:
    """Compute Bangia liquidity-adjusted VaR under the loss-sign convention.

    Parameters
    ----------
    var : float
        Finite non-positive market VaR; ``-100`` denotes a loss of 100.
    spread_mean : float
        Finite non-negative mean relative bid-ask spread as a decimal.
    spread_vol : float
        Finite non-negative volatility of the relative spread.
    confidence : float
        Confidence level strictly between 0.5 and 1.
    position_value : float
        Finite position market value; only its magnitude is used.

    Returns
    -------
    dict[str, float]
        ``var``, non-negative ``spread_cost``, adjusted ``lvar``, and
        ``lvar_ratio`` values.

    Raises
    ------
    ValueError
        If an input violates the stated finiteness, sign, or range contract.

    Examples
    --------
    >>> from finstack_quant.models.liquidity import lvar_bangia
    >>> result = lvar_bangia(-100.0, 0.01, 0.005, 0.99, 1_000_000)
    >>> round(result["lvar"], 2)
    -10915.87
    """
    ...

def almgren_chriss_impact(
    position_size: float,
    avg_daily_volume: float,
    volatility: float,
    execution_horizon_days: float,
    permanent_impact_coef: float,
    temporary_impact_coef: float,
    reference_price: float | None = None,
) -> dict[str, float]:
    """Estimate uniform Almgren-Chriss execution-impact components.

    Parameters
    ----------
    position_size : float
        Finite signed quantity in shares or contracts.
    avg_daily_volume : float
        Positive finite ADV in matching quantity units.
    volatility : float
        Positive finite daily volatility as a decimal.
    execution_horizon_days : float
        Positive finite execution horizon in trading days.
    permanent_impact_coef : float
        Non-negative finite multiplier on permanent impact.
    temporary_impact_coef : float
        Positive finite multiplier on temporary impact.
    reference_price : float | None, default None
        Optional positive finite price for notional and basis-point scaling;
        ``None`` uses unit price.

    Returns
    -------
    dict[str, float]
        Canonical ``ImpactEstimate`` fields: ``permanent_impact``,
        ``temporary_impact``, ``total_cost``, ``cost_bp``, and
        ``execution_risk``.

    Raises
    ------
    ValueError
        If an input violates the stated finiteness, sign, or range contract.

    Examples
    --------
    >>> from finstack_quant.models.liquidity import almgren_chriss_impact
    >>> result = almgren_chriss_impact(100_000, 1_000_000, 0.20, 5.0, 0.10, 0.20)
    >>> round(result["cost_bp"], 2)
    56.62
    """
    ...

def kyle_lambda(
    volumes: list[float],
    returns: list[float],
    reference_price: float,
) -> float | None:
    """Estimate price-space Kyle lambda from volume and return observations.

    Parameters
    ----------
    volumes : list[float]
        Strictly positive finite volume observations in consistent units.
    returns : list[float]
        Finite decimal returns aligned one-for-one with ``volumes``.
    reference_price : float
        Positive finite price per share or contract.

    Returns
    -------
    float | None
        Estimated price-space impact coefficient, or ``None`` for invalid
        samples or reference price.

    Notes
    -----
    This helper does not raise; unavailable estimates return ``None``.

    Examples
    --------
    >>> from finstack_quant.models.liquidity import kyle_lambda
    >>> kyle_lambda([100.0, 200.0], [0.01, -0.02], 50.0)
    0.005
    """
    ...

"""Reusable analytical, numerical, volatility, Fourier, and stochastic models.

The root namespace contains model-family functions and classes. Correlation,
credit, Monte Carlo, and rates APIs are grouped under the matching submodules.

Examples
--------
>>> from finstack_quant.models import bs_price
>>> round(bs_price(100.0, 100.0, 0.05, 0.0, 0.2, 1.0, True), 4)
10.4506
"""

def bs_price(
    spot: float,
    strike: float,
    r: float,
    q: float,
    sigma: float,
    t: float,
    is_call: bool,
) -> float:
    """
    Per-unit Black-Scholes / Garman-Kohlhagen price of a European option.

    All rates are continuously compounded decimals; ``sigma`` is annualized
    vol; ``t`` is years to expiry. Pass ``is_call=False`` for puts.

    Parameters
    ----------
    spot : float
        Spot price of the underlying.
    strike : float
        Option strike.
    r : float
        Continuously compounded risk-free rate (decimal).
    q : float
        Continuous dividend/borrow yield (decimal).
    sigma : float
        Annualized volatility (decimal).
    t : float
        Time to expiry in years.
    is_call : bool
        ``True`` for a call, ``False`` for a put.

    Returns
    -------
    float
        Per-unit option price.

    Raises
    ------
    ValueError
        If the supplied inputs produce a non-finite Black-Scholes price.

    Examples
    --------
    >>> from finstack_quant.models import bs_price
    >>> round(bs_price(100.0, 100.0, 0.05, 0.0, 0.2, 1.0, True), 4)
    10.4506

    Sources
    -------
    - Black-Scholes (1973): see docs/REFERENCES.md#black-scholes-1973
    - Merton (1973): see docs/REFERENCES.md#merton-1973
    - Garman-Kohlhagen (1983): see docs/REFERENCES.md#garman-kohlhagen-1983

    """
    ...

def vanilla_expiry_payoff(spot: float, strike: float, is_call: bool) -> float:
    """
    Vanilla option payoff at expiry: ``max(±(spot - strike), 0)``.

    Parameters
    ----------
    spot : float
        Underlying level at expiry, in the same price units as ``strike``.
        Must be finite and non-negative; zero spot is allowed.
    strike : float
        Exercise price; must be finite and strictly positive.
    is_call : bool
        ``True`` for a call (``max(spot - strike, 0)``), ``False`` for a put
        (``max(strike - spot, 0)``).

    Returns
    -------
    float
        Undiscounted expiry payoff in the same units as ``spot`` and ``strike``.

    Raises
    ------
    ValueError
        If ``spot`` is non-finite or negative, or ``strike`` is non-finite or
        not strictly positive.

    Examples
    --------
    >>> from finstack_quant.models import vanilla_expiry_payoff
    >>> vanilla_expiry_payoff(110.0, 100.0, True)
    10.0
    """
    ...

def bs_greeks(
    spot: float,
    strike: float,
    r: float,
    q: float,
    sigma: float,
    t: float,
    is_call: bool,
    theta_days: float = 365.0,
) -> dict[str, float]:
    """
    Black-Scholes / Garman-Kohlhagen Greeks as a dict.

    Returns ``{"delta", "gamma", "vega", "theta", "rho", "rho_q"}``. ``vega``
    and both rho values are per 1% move; ``theta`` is per-day using the
    ``theta_days`` day-count denominator (ACT/365 by default).

    Parameters
    ----------
    spot : float
        Spot price of the underlying.
    strike : float
        Option strike.
    r : float
        Continuously compounded risk-free rate (decimal).
    q : float
        Continuous dividend/borrow yield (decimal).
    sigma : float
        Annualized volatility (decimal).
    t : float
        Time to expiry in years.
    is_call : bool
        ``True`` for a call, ``False`` for a put.
    theta_days : float, default 365.0
        Day-count denominator for theta scaling.

    Returns
    -------
    dict[str, float]
        Greeks dict with keys ``delta``, ``gamma``, ``vega``, ``theta``,
        ``rho``, ``rho_q``.

    Raises
    ------
    ValueError
        If any numeric input is non-finite; ``spot`` or ``strike`` is
        non-positive; ``sigma``, ``t``, or ``theta_days`` is non-positive; or
        a computed Greek is non-finite.

    Examples
    --------
    >>> from finstack_quant.models import bs_greeks
    >>> greeks = bs_greeks(100.0, 100.0, 0.05, 0.0, 0.2, 1.0, True)
    >>> (round(greeks["delta"], 4), sorted(greeks))
    (0.6368, ['delta', 'gamma', 'rho', 'rho_q', 'theta', 'vega'])

    Sources
    -------
    - Black-Scholes (1973): see docs/REFERENCES.md#black-scholes-1973
    - Merton (1973): see docs/REFERENCES.md#merton-1973
    - Garman-Kohlhagen (1983): see docs/REFERENCES.md#garman-kohlhagen-1983

    """
    ...

def bs_implied_vol(
    spot: float,
    strike: float,
    r: float,
    q: float,
    t: float,
    price: float,
    is_call: bool,
) -> float:
    """
    Solve for Black-Scholes implied volatility given a target price.

    Parameters
    ----------
    spot : float
        Spot price of the underlying.
    strike : float
        Option strike.
    r : float
        Continuously compounded risk-free rate (decimal).
    q : float
        Continuous dividend/borrow yield (decimal).
    t : float
        Time to expiry in years.
    price : float
        Observed option price in the same units as spot.
    is_call : bool
        ``True`` for a call, ``False`` for a put.

    Returns
    -------
    float
        Implied volatility as a decimal.

    Raises
    ------
    ValueError
        If inputs are invalid or no root exists in the search bracket.

    Examples
    --------
    >>> from finstack_quant.models import bs_implied_vol, bs_price
    >>> price = bs_price(100.0, 100.0, 0.05, 0.0, 0.2, 1.0, True)
    >>> round(bs_implied_vol(100.0, 100.0, 0.05, 0.0, 1.0, price, True), 6)
    0.2

    Sources
    -------
    - Black-Scholes (1973): see docs/REFERENCES.md#black-scholes-1973
    - Merton (1973): see docs/REFERENCES.md#merton-1973

    """
    ...

def black76_implied_vol(
    forward: float,
    strike: float,
    df: float,
    t: float,
    price: float,
    is_call: bool,
) -> float:
    """
    Solve for Black-76 (forward-based) implied volatility given a target price.

    Parameters
    ----------
    forward : float
        Forward price at expiry.
    strike : float
        Option strike.
    df : float
        Discount factor from valuation date to expiry.
    t : float
        Time to expiry in years.
    price : float
        Observed option price (same units as forward).
    is_call : bool
        ``True`` for a call, ``False`` for a put.

    Returns
    -------
    float
        Implied volatility as a decimal.

    Raises
    ------
    ValueError
        If inputs are invalid or no root exists in the search bracket.

    Examples
    --------
    >>> from finstack_quant.models import black76_implied_vol
    >>> round(black76_implied_vol(100.0, 100.0, 0.95, 1.0, 7.5673, True), 6)
    0.2

    Sources
    -------
    - Black (1976): see docs/REFERENCES.md#black-1976

    """
    ...

# Closed-form exotics

def barrier_call(
    spot: float,
    strike: float,
    barrier: float,
    r: float,
    q: float,
    sigma: float,
    t: float,
    direction: str,
    knock: str,
) -> float:
    """
    Reiner-Rubinstein continuous-monitoring barrier call price.

    ``direction`` is ``"up"`` or ``"down"``; ``knock`` is ``"in"`` or ``"out"``.

    Parameters
    ----------
    spot : float
        Spot price of the underlying.
    strike : float
        Option strike.
    barrier : float
        Barrier level.
    r : float
        Continuously compounded risk-free rate (decimal).
    q : float
        Continuous dividend/borrow yield (decimal).
    sigma : float
        Annualized volatility (decimal).
    t : float
        Time to expiry in years.
    direction : str
        ``"up"`` or ``"down"``.
    knock : str
        ``"in"`` or ``"out"``.

    Returns
    -------
    float
        Per-unit barrier call price.

    Raises
    ------
    ValueError
        If ``direction`` is not ``"up"`` or ``"down"``, ``knock`` is not
        ``"in"`` or ``"out"``, or the formula produces a non-finite price.

    Examples
    --------
    >>> from finstack_quant.models import barrier_call
    >>> round(barrier_call(100.0, 100.0, 120.0, 0.05, 0.0, 0.2, 1.0, "up", "out"), 4)
    1.1761

    Sources
    -------
    - Reiner-Rubinstein (1991): see docs/REFERENCES.md#reiner-rubinstein-1991

    """
    ...

def asian_option_price(
    spot: float,
    strike: float,
    r: float,
    q: float,
    sigma: float,
    t: float,
    num_fixings: int,
    averaging: str = "arithmetic",
    is_call: bool = True,
) -> float:
    """
    Arithmetic (Turnbull-Wakeman) or geometric (Kemna-Vorst) Asian option price.

    Parameters
    ----------
    spot : float
        Spot price of the underlying.
    strike : float
        Option strike.
    r : float
        Continuously compounded risk-free rate (decimal).
    q : float
        Continuous dividend/borrow yield (decimal).
    sigma : float
        Annualized volatility (decimal).
    t : float
        Time to expiry in years.
    num_fixings : int
        Number of averaging fixings.
    averaging : str, default "arithmetic"
        ``"arithmetic"`` (Turnbull-Wakeman) or ``"geometric"`` (Kemna-Vorst).
    is_call : bool, default True
        ``True`` for a call, ``False`` for a put.

    Returns
    -------
    float
        Per-unit Asian option price.

    Raises
    ------
    ValueError
        If ``averaging`` is not ``"arithmetic"`` or ``"geometric"``, or the
        formula produces a non-finite price.

    Examples
    --------
    >>> from finstack_quant.models import asian_option_price
    >>> round(asian_option_price(100.0, 100.0, 0.05, 0.0, 0.2, 1.0, 12), 4)
    6.1742

    Sources
    -------
    - Kemna-Vorst (1990): see docs/REFERENCES.md#kemna-vorst-1990
    - Turnbull-Wakeman (1991): see docs/REFERENCES.md#turnbull-wakeman-1991

    """
    ...

def lookback_option_price(
    spot: float,
    strike: float,
    r: float,
    q: float,
    sigma: float,
    t: float,
    extremum: float,
    strike_type: str = "fixed",
    is_call: bool = True,
) -> float:
    """
    Conze-Viswanathan lookback option price.

    For ``strike_type="floating"``, ``strike`` is ignored and ``extremum``
    is the observed min (call) / max (put) to date.

    Parameters
    ----------
    spot : float
        Spot price of the underlying.
    strike : float
        Option strike (ignored for floating strike).
    r : float
        Continuously compounded risk-free rate (decimal).
    q : float
        Continuous dividend/borrow yield (decimal).
    sigma : float
        Annualized volatility (decimal).
    t : float
        Time to expiry in years.
    extremum : float
        Observed extremum (min for call, max for put) to date.
    strike_type : str, default "fixed"
        ``"fixed"`` or ``"floating"``.
    is_call : bool, default True
        ``True`` for a call, ``False`` for a put.

    Returns
    -------
    float
        Per-unit lookback option price.

    Raises
    ------
    ValueError
        If ``strike_type`` is not ``"fixed"`` or ``"floating"``, or the
        formula produces a non-finite price.

    Examples
    --------
    >>> from finstack_quant.models import lookback_option_price
    >>> round(lookback_option_price(100.0, 100.0, 0.05, 0.0, 0.2, 1.0, 90.0), 4)
    17.2168

    Sources
    -------
    - Conze-Viswanathan (1991): see docs/REFERENCES.md#conze-viswanathan-1991

    """
    ...

def quanto_option_price(
    spot: float,
    strike: float,
    t: float,
    rate_domestic: float,
    rate_foreign: float,
    div_yield: float,
    vol_asset: float,
    vol_fx: float,
    correlation: float,
    is_call: bool = True,
) -> float:
    """
    Quanto option (FX-adjusted cross-currency) price in domestic currency.

    Parameters
    ----------
    spot : float
        Spot price of the underlying in foreign currency.
    strike : float
        Option strike in foreign currency.
    t : float
        Time to expiry in years.
    rate_domestic : float
        Domestic risk-free rate (decimal, continuously compounded).
    rate_foreign : float
        Foreign risk-free rate (decimal, continuously compounded).
    div_yield : float
        Dividend yield on the underlying (decimal).
    vol_asset : float
        Volatility of the underlying asset (decimal).
    vol_fx : float
        Volatility of the FX rate (decimal).
    correlation : float
        Correlation between asset and FX returns.
    is_call : bool, default True
        ``True`` for a call, ``False`` for a put.

    Returns
    -------
    float
        Per-unit quanto option price in domestic currency.

    Raises
    ------
    ValueError
        If the supplied inputs produce a non-finite quanto price.

    Examples
    --------
    >>> from finstack_quant.models import quanto_option_price
    >>> round(quanto_option_price(100.0, 100.0, 1.0, 0.05, 0.02, 0.01, 0.2, 0.1, 0.3), 4)
    7.7844

    Sources
    -------
    - Garman-Kohlhagen (1983): see docs/REFERENCES.md#garman-kohlhagen-1983

    """
    ...

# Fourier option pricing helpers

def bs_cos_price(
    spot: float,
    strike: float,
    rate: float,
    dividend: float,
    vol: float,
    maturity: float,
    is_call: bool,
    n_terms: int | None = None,
) -> float:
    """
    Price a European option under Black-Scholes with the COS method.

    Parameters
    ----------
    spot : float
        Spot price of the underlying.
    strike : float
        Option strike.
    rate : float
        Continuously compounded risk-free rate (decimal).
    dividend : float
        Continuous dividend yield (decimal).
    vol : float
        Annualized volatility (decimal).
    maturity : float
        Time to expiry in years.
    is_call : bool
        ``True`` for a call, ``False`` for a put.
    n_terms : int, optional
        Number of COS terms. Uses a default when ``None``.

    Returns
    -------
    float
        Per-unit option price.

    Raises
    ------
    ValueError
        If the inputs produce an invalid COS truncation range, a non-finite
        characteristic-function value, or a non-finite option price.

    Examples
    --------
    >>> from finstack_quant.models import bs_cos_price
    >>> round(bs_cos_price(100.0, 100.0, 0.05, 0.0, 0.2, 1.0, True), 4)
    10.4506

    Sources
    -------
    - Fang-Oosterlee (2008): see docs/REFERENCES.md#fang-oosterlee-2008
    - Black-Scholes (1973): see docs/REFERENCES.md#black-scholes-1973

    """
    ...

def vg_cos_price(
    spot: float,
    strike: float,
    rate: float,
    dividend: float,
    sigma: float,
    theta: float,
    nu: float,
    maturity: float,
    is_call: bool,
    n_terms: int | None = None,
) -> float:
    """
    Price a European option under Variance Gamma with the COS method.

    Parameters
    ----------
    spot : float
        Spot price of the underlying.
    strike : float
        Option strike.
    rate : float
        Continuously compounded risk-free rate (decimal).
    dividend : float
        Continuous dividend yield (decimal).
    sigma : float
        VG diffusion parameter (volatility).
    theta : float
        VG drift parameter.
    nu : float
        VG variance rate parameter.
    maturity : float
        Time to expiry in years.
    is_call : bool
        ``True`` for a call, ``False`` for a put.
    n_terms : int, optional
        Number of COS terms. Uses a default when ``None``.

    Returns
    -------
    float
        Per-unit option price.

    Raises
    ------
    ValueError
        If the Variance Gamma parameters produce an invalid COS truncation
        range, a non-finite characteristic-function value, or a non-finite
        option price.

    Examples
    --------
    >>> from finstack_quant.models import vg_cos_price
    >>> round(vg_cos_price(100.0, 100.0, 0.05, 0.0, 0.2, -0.1, 0.2, 1.0, True), 4)
    10.4445

    Sources
    -------
    - Fang-Oosterlee (2008): see docs/REFERENCES.md#fang-oosterlee-2008
    - Madan-Carr-Chang (1998): see docs/REFERENCES.md#madan-carr-chang-1998

    """
    ...

def merton_jump_cos_price(
    spot: float,
    strike: float,
    rate: float,
    dividend: float,
    sigma: float,
    mu_jump: float,
    sigma_jump: float,
    lambda_: float,
    maturity: float,
    is_call: bool,
    n_terms: int | None = None,
) -> float:
    """
    Price a European option under Merton jump-diffusion with the COS method.

    Parameters
    ----------
    spot : float
        Spot price of the underlying.
    strike : float
        Option strike.
    rate : float
        Continuously compounded risk-free rate (decimal).
    dividend : float
        Continuous dividend yield (decimal).
    sigma : float
        Diffusion volatility (decimal).
    mu_jump : float
        Mean of the jump size distribution.
    sigma_jump : float
        Standard deviation of the jump size.
    lambda_ : float
        Jump intensity (expected jumps per year).
    maturity : float
        Time to expiry in years.
    is_call : bool
        ``True`` for a call, ``False`` for a put.
    n_terms : int, optional
        Number of COS terms. Uses a default when ``None``.

    Returns
    -------
    float
        Per-unit option price.

    Raises
    ------
    ValueError
        If the jump-diffusion parameters produce an invalid COS truncation
        range, a non-finite characteristic-function value, or a non-finite
        option price.

    Examples
    --------
    >>> from finstack_quant.models import merton_jump_cos_price
    >>> round(merton_jump_cos_price(100.0, 100.0, 0.05, 0.0, 0.2, -0.1, 0.2, 0.5, 1.0, True), 4)
    12.1642

    Sources
    -------
    - Fang-Oosterlee (2008): see docs/REFERENCES.md#fang-oosterlee-2008
    - Merton jump-diffusion (1976): see docs/REFERENCES.md#merton-1976-jump

    """
    ...

# Exotic rate products — deterministic coupon / payoff helpers
"""Reusable analytical, Fourier, credit, correlation, and stochastic models.

Examples
--------
>>> from finstack_quant.models import bs_price
>>> round(bs_price(100.0, 100.0, 0.05, 0.0, 0.2, 1.0, True), 4)
10.4506
"""

from __future__ import annotations

from typing import Any, Optional

from finstack_quant.models import correlation as correlation
from finstack_quant.models import credit as credit
from finstack_quant.models import monte_carlo as monte_carlo
from finstack_quant.models import rates as rates
from finstack_quant.models import volatility as volatility

__all__ = [
    "asian_option_price",
    "barrier_call",
    "black76_implied_vol",
    "bs_cos_price",
    "bs_greeks",
    "bs_implied_vol",
    "bs_price",
    "correlation",
    "credit",
    "lookback_option_price",
    "merton_jump_cos_price",
    "monte_carlo",
    "quanto_option_price",
    "rates",
    "vanilla_expiry_payoff",
    "vg_cos_price",
    "volatility",
]

# Closed-form analytical primitives.

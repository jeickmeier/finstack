"""Reusable analytical, numerical, volatility, Fourier, and stochastic models.

The root namespace contains model-family functions and classes. Correlation,
credit, and Monte Carlo APIs are grouped under the matching submodules.

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

# SABR volatility smile

class SabrParameters:
    """
    SABR parameters ``(alpha, beta, nu, rho)`` with optional ``shift``.

    Enforces ``alpha > 0``, ``beta in [0, 1]``, ``nu >= 0``, ``rho in
    [-1, 1]``, and ``shift > 0`` when supplied.

    Sources
    -------
    - Hagan SABR (2002): see docs/REFERENCES.md#hagan-2002-sabr

    Examples
    --------
    >>> from finstack_quant.models import SabrParameters
    >>> params = SabrParameters(0.2, 0.5, 0.3, -0.2)
    >>> (params.alpha, params.beta, params.nu, params.rho, params.is_shifted())
    (0.2, 0.5, 0.3, -0.2, False)

    """

    def __init__(
        self,
        alpha: float,
        beta: float,
        nu: float,
        rho: float,
        shift: float | None = None,
    ) -> None:
        """
        Create a validated SABR volatility-smile parameter set.

        Parameters
        ----------
        alpha : float
            Positive SABR level parameter in the rate or price unit convention.
        beta : float
            CEV elasticity constrained to the closed interval ``[0, 1]``.
        nu : float
            Non-negative volatility-of-volatility parameter per square-root year.
        rho : float
            Instantaneous forward/volatility correlation in ``[-1, 1]``.
        shift : float or None, default None
            Optional positive shifted-lognormal displacement for negative-rate
            strikes and forwards; ``None`` uses the unshifted formula.

        Raises
        ------
        ValueError
            If any parameter is non-finite; ``alpha`` is non-positive;
            ``beta`` is outside ``[0, 1]``; ``nu`` is negative; ``rho`` is
            outside ``[-1, 1]``; or a supplied ``shift`` is non-positive.
        """
        ...
    @staticmethod
    def equity_default() -> SabrParameters:
        """
        Equity-standard defaults ``(alpha=0.20, beta=1.0, nu=0.30, rho=-0.20)``.

        Returns
        -------
        SabrParameters
            Default equity SABR parameters.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.models import SabrParameters
        >>> params = SabrParameters.equity_default()
        >>> (params.alpha, params.beta, params.nu, params.rho)
        (0.2, 1.0, 0.3, -0.2)
        """
        ...

    @staticmethod
    def rates_default() -> SabrParameters:
        """
        Rates-standard defaults ``(alpha=0.02, beta=0.5, nu=0.30, rho=0.0)``.

        Returns
        -------
        SabrParameters
            Default rates SABR parameters.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.models import SabrParameters
        >>> params = SabrParameters.rates_default()
        >>> (params.alpha, params.beta, params.nu, params.rho)
        (0.02, 0.5, 0.3, 0.0)
        """
        ...

    @property
    def alpha(self) -> float:
        """
        SABR alpha level parameter.

        Returns
        -------
        float
            Alpha parameter value.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def beta(self) -> float:
        """
        SABR beta elasticity parameter in ``[0, 1]``.

        Returns
        -------
        float
            Beta parameter value.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def nu(self) -> float:
        """
        Volatility-of-volatility parameter.

        Returns
        -------
        float
            Nu parameter value.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def rho(self) -> float:
        """
        Forward/volatility correlation parameter in ``[-1, 1]``.

        Returns
        -------
        float
            Rho parameter value.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def shift(self) -> float | None:
        """
        Optional positive shift used for negative-rate smiles.

        Returns
        -------
        float or None
            Shift value, or ``None`` if not set.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def is_shifted(self) -> bool:
        """
        ``True`` when parameters include a non-zero shift (negative-rate support).

        Returns
        -------
        bool
            ``True`` if a non-zero shift is present.

        Notes
        -----
        This method does not raise; it returns ``True`` or ``False``.
        """
        ...

class SabrModel:
    """
    Hagan-2002 SABR stochastic-volatility smile model.

    Sources
    -------
    - Hagan SABR (2002): see docs/REFERENCES.md#hagan-2002-sabr

    Examples
    --------
    >>> from finstack_quant.models import SabrModel, SabrParameters
    >>> model = SabrModel(SabrParameters.rates_default())
    >>> (round(model.implied_vol(0.02, 0.02, 1.0), 6), model.supports_negative_rates())
    (0.142511, False)

    """

    def __init__(self, params: SabrParameters) -> None:
        """
        Create a SABR model from validated parameters.

        Parameters
        ----------
        params : SabrParameters
            SABR parameter set (alpha, beta, nu, rho, optional shift).

        Raises
        ------
        ValueError
            If parameters violate SABR constraints.
        """
        ...

    def implied_vol(self, forward: float, strike: float, t: float) -> float:
        """
        Black-style implied volatility under the Hagan-2002 expansion.

        Parameters
        ----------
        forward : float
            Forward price at expiry.
        strike : float
            Strike price.
        t : float
            Time to expiry in years.

        Returns
        -------
        float
            Implied volatility as a decimal.

        Raises
        ------
        ValueError
            If an input is non-finite, ``t`` is non-positive, unshifted
            non-normal SABR receives a non-positive forward or strike, shifted
            SABR has a non-positive effective forward or strike, or the
            calculation produces an invalid volatility.
        """
        ...

    @property
    def params(self) -> SabrParameters:
        """
        Parameters used by this model.

        Returns
        -------
        SabrParameters
            The SABR parameter set.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def supports_negative_rates(self) -> bool:
        """
        Return ``True`` when the model has a positive shift.

        Returns
        -------
        bool
            ``True`` if the shift is non-zero, enabling negative-rate smiles.

        Notes
        -----
        This method does not raise; it returns ``True`` or ``False``.
        """
        ...

class SabrSmile:
    """
    Volatility smile generator for a fixed ``(forward, t)`` pair.

    Sources
    -------
    - Hagan SABR (2002): see docs/REFERENCES.md#hagan-2002-sabr

    Examples
    --------
    >>> from finstack_quant.models import SabrParameters, SabrSmile
    >>> smile = SabrSmile(SabrParameters.equity_default(), 100.0, 1.0)
    >>> (round(smile.atm_vol(), 6), len(smile.generate_smile([90.0, 100.0, 110.0])))
    (0.20081, 3)

    """

    def __init__(
        self,
        params: SabrParameters,
        forward: float,
        t: float,
    ) -> None:
        """
        Create a smile helper for one forward and expiry.

        Parameters
        ----------
        params : SabrParameters
            SABR parameter set.
        forward : float
            Forward price at expiry.
        t : float
            Time to expiry in years.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...

    def atm_vol(self) -> float:
        """
        Return the ATM implied volatility.

        Returns
        -------
        float
            ATM implied vol as a decimal.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    def implied_vol(self, strike: float) -> float:
        """
        Return implied volatility at ``strike``.

        Parameters
        ----------
        strike : float
            Strike price.

        Returns
        -------
        float
            Implied vol as a decimal.

        Raises
        ------
        ValueError
            If the stored forward, ``strike``, or expiry is outside the model's
            valid domain, or the calculation produces an invalid volatility.
        RuntimeError
            If the native smile calculation returns no value for ``strike``.
        """
        ...

    def generate_smile(self, strikes: list[float]) -> list[float]:
        """
        Return implied volatilities for all supplied strikes.

        Parameters
        ----------
        strikes : list[float]
            Strike grid.

        Returns
        -------
        list[float]
            Implied vols aligned with ``strikes``.

        Raises
        ------
        ValueError
            If the stored forward, any supplied strike, or expiry is outside
            the model's valid domain, or a volatility calculation is invalid.
        """
        ...

    def arbitrage_diagnostics(
        self,
        strikes: list[float],
        r: float = 0.0,
        q: float = 0.0,
    ) -> dict[str, Any]:
        """
        Butterfly + monotonicity arbitrage diagnostics on ``strikes``.

        Returns a dict with ``arbitrage_free``, ``butterfly_violations``,
        and ``monotonicity_violations``.

        Parameters
        ----------
        strikes : list[float]
            Strike grid to test.
        r : float, default 0.0
            Risk-free rate (decimal).
        q : float, default 0.0
            Dividend yield (decimal).

        Returns
        -------
        dict[str, Any]
            Diagnostics dict with ``arbitrage_free``, ``butterfly_violations``,
            and ``monotonicity_violations``.

        Raises
        ------
        ValueError
            If the stored forward, a strike, or expiry is outside the model's
            valid domain, or smile generation produces an invalid volatility.
        """
        ...

class SabrCalibrator:
    """
    SABR calibrator (Levenberg-Marquardt with beta fixed).

    Sources
    -------
    - Hagan SABR (2002): see docs/REFERENCES.md#hagan-2002-sabr

    Examples
    --------
    >>> from finstack_quant.models import SabrCalibrator
    >>> calibrator = SabrCalibrator()
    >>> calibrator.with_tolerance(1e-6) is calibrator
    False

    """

    def __init__(self) -> None:
        """
        Create a SABR calibrator with the library default tolerance and iteration cap.

        Use :meth:`high_precision` for a tighter production fit, or
        :meth:`with_tolerance` to override the residual tolerance.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...

    @staticmethod
    def high_precision() -> SabrCalibrator:
        """
        Return a calibrator with tighter tolerance for production fits.

        Returns
        -------
        SabrCalibrator
            Calibrator with high-precision tolerance.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.models import SabrCalibrator
        >>> calibrator = SabrCalibrator.high_precision()
        >>> calibrator.with_tolerance(1e-6) is calibrator
        False
        """
        ...

    def with_tolerance(self, tolerance: float) -> SabrCalibrator:
        """
        Return a copy with an overridden convergence tolerance.

        Parameters
        ----------
        tolerance : float
            Relative RMSE target for the fit.

        Returns
        -------
        SabrCalibrator
            New calibrator instance sharing other settings.

        Notes
        -----
        This builder returns a copy with the field set and does not raise.

        """
        ...

    def calibrate(
        self,
        forward: float,
        strikes: list[float],
        market_vols: list[float],
        t: float,
        beta: float,
    ) -> SabrParameters:
        """
        Fit ``(alpha, nu, rho)`` to market vols with ``beta`` fixed.

        Parameters
        ----------
        forward : float
            Forward at expiry.
        strikes : list[float]
            Strike grid aligned with ``market_vols``.
        market_vols : list[float]
            Market implied vols as decimals.
        t : float
            Expiry in years.
        beta : float
            Fixed SABR beta in ``[0, 1]``.

        Returns
        -------
        SabrParameters
            Calibrated :class:`SabrParameters`.

        Raises
        ------
        ValueError
            If lengths mismatch or fit fails to converge.
        """
        ...

    def calibrate_auto_shift(
        self,
        forward: float,
        strikes: list[float],
        market_vols: list[float],
        t: float,
        beta: float,
    ) -> SabrParameters:
        """
        Calibrate with automatic shift selection for negative-rate smiles.

        Parameters
        ----------
        forward : float
            Forward at expiry.
        strikes : list[float]
            Strike grid aligned with ``market_vols``.
        market_vols : list[float]
            Market implied vols as decimals.
        t : float
            Expiry in years.
        beta : float
            Fixed SABR beta in ``[0, 1]``.

        Returns
        -------
        SabrParameters
            Calibrated :class:`SabrParameters` with auto-selected shift.

        Raises
        ------
        ValueError
            If lengths mismatch or fit fails to converge.
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

__all__ = [
    "SabrCalibrator",
    "SabrModel",
    "SabrParameters",
    "SabrSmile",
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
    "vanilla_expiry_payoff",
    "vg_cos_price",
]

# Closed-form analytical primitives.

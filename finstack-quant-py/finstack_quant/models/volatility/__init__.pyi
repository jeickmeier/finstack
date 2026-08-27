"""Product-independent volatility models, evaluators, fitting, and convention conversion.

Examples
--------
>>> from finstack_quant.models.volatility import SabrParameters
>>> SabrParameters.rates_default().beta
0.5
"""

from __future__ import annotations

from typing import Any, Literal, Optional, TypedDict

from finstack_quant.core.market_data import FxDeltaVolSurface, VolCube, VolSurface

__all__ = [
    "SabrCalibrator",
    "SabrModel",
    "SabrParameters",
    "SabrSmile",
    "check_butterfly_grid",
    "check_calendar_spread_grid",
    "check_local_vol_density_grid",
    "check_surface_grid",
    "delta_to_strike",
    "get_cube_normal_vol",
    "get_cube_normal_vol_clamped",
    "get_cube_vol",
    "get_cube_vol_clamped",
    "get_fx_delta_pillar_vols",
    "get_fx_delta_vol",
    "get_surface_vol",
    "get_surface_vol_clamped",
    "materialize_cube_expiry_slice",
    "materialize_cube_expiry_slice_normal",
    "materialize_cube_tenor_slice",
    "materialize_cube_tenor_slice_normal",
    "materialize_fx_delta_surface",
    "strike_to_delta",
]

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
    >>> from finstack_quant.models.volatility import SabrParameters
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
        >>> from finstack_quant.models.volatility import SabrParameters
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
        >>> from finstack_quant.models.volatility import SabrParameters
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
    >>> from finstack_quant.models.volatility import SabrModel, SabrParameters
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
    >>> from finstack_quant.models.volatility import SabrParameters, SabrSmile
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
    >>> from finstack_quant.models.volatility import SabrCalibrator
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
        >>> from finstack_quant.models.volatility import SabrCalibrator
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

def get_surface_vol(surface: VolSurface, expiry: float, strike: float) -> float:
    """Evaluate a stored surface with checked grid bounds.

    Parameters
    ----------
    surface : VolSurface
        Data-only core surface artifact.
    expiry : float
        Positive option expiry in years inside the grid.
    strike : float
        Secondary-axis coordinate in stored units.

    Returns
    -------
    float
        Interpolated annualized volatility as a decimal.

    Raises
    ------
    ValueError
        If a coordinate is non-finite or outside the checked grid.

    Examples
    --------
    >>> from finstack_quant.core.market_data import VolSurface
    >>> from finstack_quant.models.volatility import get_surface_vol
    >>> get_surface_vol(VolSurface("V", [1.0], [100.0], [0.2]), 1.0, 100.0)
    0.2
    """
    ...

def get_surface_vol_clamped(surface: VolSurface, expiry: float, strike: float) -> float:
    """Evaluate a stored surface with flat coordinate extrapolation.

    Parameters
    ----------
    surface : VolSurface
        Data-only core surface artifact.
    expiry : float
        Finite expiry in years, clamped to the stored axis.
    strike : float
        Finite secondary coordinate, clamped to the stored axis.

    Returns
    -------
    float
        Interpolated volatility, or nan for a non-finite input.

    This function does not raise; invalid finite model outputs are returned as nan.

    Examples
    --------
    >>> from finstack_quant.core.market_data import VolSurface
    >>> from finstack_quant.models.volatility import get_surface_vol_clamped
    >>> get_surface_vol_clamped(VolSurface("V", [1.0], [100.0], [0.2]), 2.0, 120.0)
    0.2
    """
    ...

def get_cube_vol(cube: VolCube, expiry: float, tenor: float, strike: float) -> float:
    """Evaluate Black volatility from a stored SABR cube.

    Parameters
    ----------
    cube : VolCube
        Data-only SABR parameter and forward grid.
    expiry : float
        Positive expiry in years inside the grid.
    tenor : float
        Positive underlying tenor in years inside the grid.
    strike : float
        Finite strike in stored forward units.

    Returns
    -------
    float
        Annualized Black volatility as a decimal.

    Raises
    ------
    ValueError
        If coordinates or SABR evaluation are invalid.

    Examples
    --------
    >>> from finstack_quant.core.market_data import VolCube
    >>> from finstack_quant.models.volatility import get_cube_vol
    >>> c = VolCube("C", [1.0], [5.0], [{"alpha": 0.03, "beta": 0.5, "rho": -0.2, "nu": 0.4}], [0.03])
    >>> get_cube_vol(c, 1.0, 5.0, 0.03) > 0
    True
    """
    ...

def get_cube_vol_clamped(cube: VolCube, expiry: float, tenor: float, strike: float) -> float:
    """Evaluate Black cube volatility with coordinate clamping.

    Parameters
    ----------
    cube : VolCube
        Data-only SABR parameter and forward grid.
    expiry : float
        Finite expiry in years, clamped to the grid.
    tenor : float
        Finite tenor in years, clamped to the grid.
    strike : float
        Finite strike in stored forward units.

    Returns
    -------
    float
        Annualized volatility, or nan when undefined.

    This function does not raise; invalid coordinates or model outputs return nan.

    Examples
    --------
    >>> from finstack_quant.core.market_data import VolCube
    >>> from finstack_quant.models.volatility import get_cube_vol_clamped
    >>> c = VolCube("C", [1.0], [5.0], [{"alpha": 0.03, "beta": 0.5, "rho": -0.2, "nu": 0.4}], [0.03])
    >>> get_cube_vol_clamped(c, 2.0, 10.0, 0.03) > 0
    True
    """
    ...

def get_cube_normal_vol(cube: VolCube, expiry: float, tenor: float, strike: float) -> float:
    """Evaluate normal volatility from a stored SABR cube.

    Parameters
    ----------
    cube : VolCube
        Data-only SABR parameter and forward grid.
    expiry : float
        Positive expiry in years inside the grid.
    tenor : float
        Positive tenor in years inside the grid.
    strike : float
        Finite strike in absolute rate units.

    Returns
    -------
    float
        Annualized normal volatility in absolute rate units.

    Raises
    ------
    ValueError
        If coordinates or the shifted-SABR domain are invalid.

    Examples
    --------
    >>> from finstack_quant.core.market_data import VolCube
    >>> from finstack_quant.models.volatility import get_cube_normal_vol
    >>> c = VolCube("C", [1.0], [5.0], [{"alpha": 0.03, "beta": 0.5, "rho": -0.2, "nu": 0.4}], [0.03])
    >>> get_cube_normal_vol(c, 1.0, 5.0, 0.03) > 0
    True
    """
    ...

def get_cube_normal_vol_clamped(cube: VolCube, expiry: float, tenor: float, strike: float) -> float:
    """Evaluate normal cube volatility with coordinate clamping.

    Parameters
    ----------
    cube : VolCube
        Data-only SABR parameter and forward grid.
    expiry : float
        Finite expiry in years, clamped to the grid.
    tenor : float
        Finite tenor in years, clamped to the grid.
    strike : float
        Finite strike in absolute rate units.

    Returns
    -------
    float
        Normal volatility, or nan when the domain is invalid.

    This function does not raise; invalid coordinates or model outputs return nan.

    Examples
    --------
    >>> from finstack_quant.core.market_data import VolCube
    >>> from finstack_quant.models.volatility import get_cube_normal_vol_clamped
    >>> c = VolCube("C", [1.0], [5.0], [{"alpha": 0.03, "beta": 0.5, "rho": -0.2, "nu": 0.4}], [0.03])
    >>> get_cube_normal_vol_clamped(c, 2.0, 10.0, 0.03) > 0
    True
    """
    ...

def materialize_cube_tenor_slice(cube: VolCube, tenor: float, strikes: list[float]) -> VolSurface:
    """Materialize a lognormal cube tenor slice.

    Parameters
    ----------
    cube : VolCube
        Source SABR cube.
    tenor : float
        Finite tenor in years, clamped to the grid.
    strikes : list[float]
        Non-empty finite strike grid.

    Returns
    -------
    VolSurface
        Data-only expiry-by-strike Black surface.

    Raises
    ------
    ValueError
        If inputs or evaluation are invalid.

    Examples
    --------
    >>> from finstack_quant.core.market_data import VolCube
    >>> from finstack_quant.models.volatility import materialize_cube_tenor_slice
    >>> c = VolCube("C", [1.0], [5.0], [{"alpha": 0.03, "beta": 0.5, "rho": -0.2, "nu": 0.4}], [0.03])
    >>> materialize_cube_tenor_slice(c, 5.0, [0.02, 0.03]).grid_shape
    (1, 2)
    """
    ...

def materialize_cube_tenor_slice_normal(cube: VolCube, tenor: float, strikes: list[float]) -> VolSurface:
    """Materialize a normal-volatility cube tenor slice.

    Parameters
    ----------
    cube : VolCube
        Source SABR cube.
    tenor : float
        Finite tenor in years, clamped to the grid.
    strikes : list[float]
        Non-empty finite strike grid.

    Returns
    -------
    VolSurface
        Data-only expiry-by-strike normal surface.

    Raises
    ------
    ValueError
        If inputs or evaluation are invalid.

    Examples
    --------
    >>> from finstack_quant.core.market_data import VolCube
    >>> from finstack_quant.models.volatility import materialize_cube_tenor_slice_normal
    >>> c = VolCube("C", [1.0], [5.0], [{"alpha": 0.03, "beta": 0.5, "rho": -0.2, "nu": 0.4}], [0.03])
    >>> materialize_cube_tenor_slice_normal(c, 5.0, [0.02, 0.03]).quote_type
    'normal'
    """
    ...

def materialize_cube_expiry_slice(cube: VolCube, expiry: float, strikes: list[float]) -> VolSurface:
    """Materialize a lognormal cube expiry slice.

    Parameters
    ----------
    cube : VolCube
        Source SABR cube.
    expiry : float
        Finite expiry in years, clamped to the grid.
    strikes : list[float]
        Non-empty finite strike grid.

    Returns
    -------
    VolSurface
        Data-only tenor-axis Black surface.

    Raises
    ------
    ValueError
        If inputs or evaluation are invalid.

    Examples
    --------
    >>> from finstack_quant.core.market_data import VolCube
    >>> from finstack_quant.models.volatility import materialize_cube_expiry_slice
    >>> c = VolCube("C", [1.0], [5.0], [{"alpha": 0.03, "beta": 0.5, "rho": -0.2, "nu": 0.4}], [0.03])
    >>> materialize_cube_expiry_slice(c, 1.0, [0.02, 0.03]).grid_shape
    (1, 2)
    """
    ...

def materialize_cube_expiry_slice_normal(cube: VolCube, expiry: float, strikes: list[float]) -> VolSurface:
    """Materialize a normal-volatility cube expiry slice.

    Parameters
    ----------
    cube : VolCube
        Source SABR cube.
    expiry : float
        Finite expiry in years, clamped to the grid.
    strikes : list[float]
        Non-empty finite strike grid.

    Returns
    -------
    VolSurface
        Data-only tenor-axis normal surface.

    Raises
    ------
    ValueError
        If inputs or evaluation are invalid.

    Examples
    --------
    >>> from finstack_quant.core.market_data import VolCube
    >>> from finstack_quant.models.volatility import materialize_cube_expiry_slice_normal
    >>> c = VolCube("C", [1.0], [5.0], [{"alpha": 0.03, "beta": 0.5, "rho": -0.2, "nu": 0.4}], [0.03])
    >>> materialize_cube_expiry_slice_normal(c, 1.0, [0.02, 0.03]).quote_type
    'normal'
    """
    ...

def get_fx_delta_pillar_vols(surface: FxDeltaVolSurface, expiry_index: int) -> tuple[float, float, float]:
    """Recover ATM, 25-delta put, and 25-delta call vols.

    Parameters
    ----------
    surface : FxDeltaVolSurface
        Data-only FX delta quote artifact.
    expiry_index : int
        Zero-based stored expiry index.

    Returns
    -------
    tuple[float, float, float]
        ATM, put, and call annualized decimal volatilities.

    Raises
    ------
    ValueError
        If the index is outside the surface.

    Examples
    --------
    >>> from finstack_quant.core.market_data import FxDeltaVolSurface
    >>> from finstack_quant.models.volatility import get_fx_delta_pillar_vols
    >>> get_fx_delta_pillar_vols(FxDeltaVolSurface("FX", [1.0], [0.12], [0.01], [0.002]), 0)
    (0.12, 0.117, 0.127)
    """
    ...

def get_fx_delta_vol(surface: FxDeltaVolSurface, expiry: float, strike: float, forward: float) -> float:
    """Evaluate an FX delta-quoted surface.

    Parameters
    ----------
    surface : FxDeltaVolSurface
        Data-only FX delta quote artifact.
    expiry : float
        Positive option expiry in years.
    strike : float
        Positive strike in FX quote units.
    forward : float
        Positive FX forward in quote units.

    Returns
    -------
    float
        Interpolated annualized Black volatility.

    Raises
    ------
    ValueError
        If inputs or reconstructed wing vols are invalid.

    Examples
    --------
    >>> from finstack_quant.core.market_data import FxDeltaVolSurface
    >>> from finstack_quant.models.volatility import get_fx_delta_vol
    >>> s = FxDeltaVolSurface("FX", [1.0], [0.12], [0.01], [0.002])
    >>> get_fx_delta_vol(s, 1.0, 1.1, 1.1) > 0
    True
    """
    ...

def materialize_fx_delta_surface(
    surface: FxDeltaVolSurface,
    spot: float,
    domestic_rate: float,
    foreign_rate: float,
) -> VolSurface:
    """Materialize FX delta quotes as a strike-axis surface.

    Parameters
    ----------
    surface : FxDeltaVolSurface
        Source FX delta quote artifact.
    spot : float
        Positive spot FX rate.
    domestic_rate : float
        Continuously compounded domestic decimal rate.
    foreign_rate : float
        Continuously compounded foreign decimal rate.

    Returns
    -------
    VolSurface
        Data-only expiry-by-strike surface.

    Raises
    ------
    ValueError
        If market inputs or smile nodes are invalid.

    Examples
    --------
    >>> from finstack_quant.core.market_data import FxDeltaVolSurface
    >>> from finstack_quant.models.volatility import materialize_fx_delta_surface
    >>> s = FxDeltaVolSurface("FX", [1.0], [0.12], [0.01], [0.002])
    >>> materialize_fx_delta_surface(s, 1.1, 0.03, 0.02).grid_shape
    (1, 3)
    """
    ...

def delta_to_strike(delta: float, forward: float, vol: float, expiry: float) -> float:
    """Convert premium-unadjusted forward call delta to strike.

    Parameters
    ----------
    delta : float
        Forward call delta in the open interval from zero to one.
    forward : float
        Positive forward in strike units.
    vol : float
        Positive annualized Black decimal volatility.
    expiry : float
        Positive option expiry in years.

    Returns
    -------
    float
        Strike in forward units.

    This function does not raise; IEEE non-finite results propagate for invalid inputs.

    Examples
    --------
    >>> from finstack_quant.models.volatility import delta_to_strike
    >>> round(delta_to_strike(0.25, 1.1, 0.12, 1.0), 6)
    1.206685
    """
    ...

def strike_to_delta(strike: float, forward: float, vol: float, expiry: float) -> float:
    """Convert strike to premium-unadjusted forward call delta.

    Parameters
    ----------
    strike : float
        Positive strike in forward units.
    forward : float
        Positive forward in strike units.
    vol : float
        Positive annualized Black decimal volatility.
    expiry : float
        Positive option expiry in years.

    Returns
    -------
    float
        Forward call delta as a decimal probability.

    This function does not raise; IEEE non-finite results propagate for invalid inputs.

    Examples
    --------
    >>> from finstack_quant.models.volatility import strike_to_delta
    >>> round(strike_to_delta(1.1, 1.1, 0.12, 1.0), 6)
    0.523922
    """
    ...

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
    >>> from finstack_quant.models.volatility import check_butterfly_grid
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
    >>> from finstack_quant.models.volatility import check_calendar_spread_grid
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
    >>> from finstack_quant.models.volatility import check_local_vol_density_grid
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
    >>> from finstack_quant.models.volatility import check_surface_grid
    >>> strikes, expiries = [90.0, 100.0, 110.0], [1.0, 2.0]
    >>> vols, forwards = [[0.2, 0.2, 0.2], [0.2, 0.2, 0.2]], [100.0, 100.0]
    >>> check_surface_grid(strikes, expiries, vols, forwards)["passed"]
    True

    """
    ...

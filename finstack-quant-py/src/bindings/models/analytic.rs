//! Closed-form analytic option primitives (Black-Scholes, Black-76, implied vol).
//!
//! Thin wrappers around `finstack_quant_models::closed_form`
//! that expose the per-unit pricing and Greek formulas to Python without
//! requiring a full `MarketContext` / `Instrument` round trip.
//!
//! Conventions mirror the underlying Rust crate:
//!
//! - `r`, `q` are continuously-compounded annualized rates (decimal).
//! - `sigma` is annualized volatility (decimal).
//! - `t` is time to expiry in years.
//! - Greeks use the canonical Rust scaling: `vega` and `rho_*` are per-1% move,
//!   `theta` is per day under ACT/365 (use 252 day-count via `theta_days` if you
//!   want a business-day convention).

use crate::errors::display_to_py;
use finstack_quant_models::closed_form::implied_vol::{black76_implied_vol, bs_implied_vol};
use finstack_quant_models::closed_form::{
    asian_option_price_str, barrier_call_str, bs_greeks, bs_price, lookback_option_price_str,
    quanto_option_price, vanilla_expiry_payoff, BsGreeks,
};
use finstack_quant_models::OptionType;
use pyo3::prelude::*;
use pyo3::types::PyDict;

const DEFAULT_THETA_DAYS_PER_YEAR: f64 = 365.0;

// bs_price

/// Black-Scholes / Garman-Kohlhagen per-unit price of a European option.
///
/// Parameters
/// ----------
/// spot : float
///     Current spot price `S`.
/// strike : float
///     Strike price `K`.
/// r : float
///     Domestic / risk-free rate (continuously compounded, decimal).
/// q : float
///     Dividend yield or foreign rate (continuously compounded, decimal).
/// sigma : float
///     Annualized volatility (decimal, e.g. ``0.20`` for 20%).
/// t : float
///     Time to expiry in years.
/// is_call : bool
///     ``True`` for a call, ``False`` for a put.
///
/// Returns
/// -------
/// float
///     Present-value option price (per unit; multiply by contract size to scale).
///
/// Raises
/// ------
/// ValueError
///     If the inputs produce a non-finite price (e.g. negative volatility).
///
/// Sources
/// -------
/// - Black-Scholes (1973): see docs/REFERENCES.md#black-scholes-1973
/// - Merton (1973): see docs/REFERENCES.md#merton-1973
/// - Garman-Kohlhagen (1983): see docs/REFERENCES.md#garman-kohlhagen-1983
#[pyfunction(name = "bs_price")]
#[pyo3(signature = (spot, strike, r, q, sigma, t, is_call))]
fn bs_price_wrapper(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    is_call: bool,
) -> PyResult<f64> {
    bs_price(spot, strike, r, q, sigma, t, OptionType::from(is_call)).map_err(display_to_py)
}

/// Vanilla option payoff at expiry: ``max(±(spot - strike), 0)``.
///
/// Parameters
/// ----------
/// spot : float
///     Underlying level at expiry, in the same price units as ``strike``.
/// strike : float
///     Exercise price; must be finite and strictly positive.
/// is_call : bool
///     ``True`` for a call (``max(spot - strike, 0)``), ``False`` for a put
///     (``max(strike - spot, 0)``).
///
/// Returns
/// -------
/// float
///     Undiscounted expiry payoff in the same units as ``spot`` and ``strike``.
///
/// Raises
/// ------
/// ValueError
///     If ``spot`` is non-finite or ``strike`` is non-finite or not strictly
///     positive.
///
/// Examples
/// --------
/// >>> from finstack_quant.models import vanilla_expiry_payoff
/// >>> vanilla_expiry_payoff(110.0, 100.0, True)
/// 10.0
#[pyfunction(name = "vanilla_expiry_payoff")]
#[pyo3(signature = (spot, strike, is_call))]
fn vanilla_expiry_payoff_wrapper(spot: f64, strike: f64, is_call: bool) -> PyResult<f64> {
    vanilla_expiry_payoff(spot, strike, OptionType::from(is_call)).map_err(display_to_py)
}

// bs_greeks

/// Black-Scholes / Garman-Kohlhagen Greeks for a European option.
///
/// Returns a dict with ``delta``, ``gamma``, ``vega``, ``theta``, ``rho`` (=rho_r),
/// and ``rho_q``. ``vega`` and both rho values are per 1% move; ``theta`` is
/// per day using the `theta_days` day-count (ACT/365 by default).
///
/// Parameters
/// ----------
/// spot : float
///     Current spot price `S`.
/// strike : float
///     Strike price `K`.
/// r : float
///     Domestic / risk-free rate (continuously compounded, decimal).
/// q : float
///     Dividend yield or foreign rate (continuously compounded, decimal).
/// sigma : float
///     Annualized volatility (decimal, e.g. ``0.20`` for 20%).
/// t : float
///     Time to expiry in years.
/// is_call : bool
///     ``True`` for a call, ``False`` for a put.
/// theta_days : float, optional
///     Day-count denominator for per-day theta (default ``365.0``). Pass
///     ``252.0`` for business-day-scaled theta, ``360.0`` for ACT/360.
///
/// Returns
/// -------
/// dict
///     ``{"delta": ..., "gamma": ..., "vega": ..., "theta": ..., "rho": ..., "rho_q": ...}``.
///
/// Sources
/// -------
/// - Black-Scholes (1973): see docs/REFERENCES.md#black-scholes-1973
/// - Merton (1973): see docs/REFERENCES.md#merton-1973
/// - Garman-Kohlhagen (1983): see docs/REFERENCES.md#garman-kohlhagen-1983
#[pyfunction(name = "bs_greeks")]
#[pyo3(
    signature = (spot, strike, r, q, sigma, t, is_call, theta_days=DEFAULT_THETA_DAYS_PER_YEAR),
    text_signature = "(spot, strike, r, q, sigma, t, is_call, theta_days=365.0)"
)]
#[allow(clippy::too_many_arguments)]
fn bs_greeks_wrapper<'py>(
    py: Python<'py>,
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    is_call: bool,
    theta_days: f64,
) -> PyResult<Bound<'py, PyDict>> {
    // theta_days validation (finite, > 0) lives in `bs_greeks`.
    let greeks: BsGreeks = bs_greeks(
        spot,
        strike,
        r,
        q,
        sigma,
        t,
        OptionType::from(is_call),
        theta_days,
    )
    .map_err(display_to_py)?;
    let out = PyDict::new(py);
    out.set_item("delta", greeks.delta)?;
    out.set_item("gamma", greeks.gamma)?;
    out.set_item("vega", greeks.vega)?;
    out.set_item("theta", greeks.theta)?;
    out.set_item("rho", greeks.rho_r)?;
    out.set_item("rho_q", greeks.rho_q)?;
    Ok(out)
}

// bs_implied_vol

/// Solve for Black-Scholes / Garman-Kohlhagen implied volatility.
///
/// Uses a Newton-in-vega hybrid with bisection fallback. Returns ``0.0`` when
/// ``t <= 0`` (expired — volatility is undefined); raises on non-finite inputs
/// or target prices outside the no-arbitrage bracket.
///
/// Parameters
/// ----------
/// spot : float
///     Current spot price `S`.
/// strike : float
///     Strike price `K`.
/// r : float
///     Domestic / risk-free rate (continuously compounded, decimal).
/// q : float
///     Dividend yield or foreign rate (continuously compounded, decimal).
/// t : float
///     Time to expiry in years.
/// price : float
///     Target per-unit option price.
/// is_call : bool
///     ``True`` for a call, ``False`` for a put.
///
/// Returns
/// -------
/// float
///     Implied volatility (annualized, decimal).
///
/// Sources
/// -------
/// - Black-Scholes (1973): see docs/REFERENCES.md#black-scholes-1973
/// - Merton (1973): see docs/REFERENCES.md#merton-1973
#[pyfunction(name = "bs_implied_vol")]
#[pyo3(signature = (spot, strike, r, q, t, price, is_call))]
fn bs_implied_vol_wrapper(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    t: f64,
    price: f64,
    is_call: bool,
) -> PyResult<f64> {
    bs_implied_vol(spot, strike, r, q, t, OptionType::from(is_call), price).map_err(display_to_py)
}

// black76_implied_vol

/// Solve for Black-76 (forward-based) implied volatility.
///
/// Takes a forward price, strike, discount factor, time to expiry, and target
/// price; returns the lognormal implied vol consistent with the Black-76
/// pricing formula.
///
/// Parameters
/// ----------
/// forward : float
///     Forward price `F`.
/// strike : float
///     Strike `K`.
/// df : float
///     Discount factor from expiry to settlement (``exp(-r * t)`` for
///     continuously-compounded rate ``r``).
/// t : float
///     Time to expiry in years.
/// price : float
///     Target per-unit option price.
/// is_call : bool
///     ``True`` for a call, ``False`` for a put.
///
/// Returns
/// -------
/// float
///     Implied volatility (annualized, decimal).
///
/// Sources
/// -------
/// - Black (1976): see docs/REFERENCES.md#black-1976
#[pyfunction(name = "black76_implied_vol")]
#[pyo3(signature = (forward, strike, df, t, price, is_call))]
fn black76_implied_vol_wrapper(
    forward: f64,
    strike: f64,
    df: f64,
    t: f64,
    price: f64,
    is_call: bool,
) -> PyResult<f64> {
    black76_implied_vol(forward, strike, df, t, OptionType::from(is_call), price)
        .map_err(display_to_py)
}

/// Reiner-Rubinstein continuous-monitoring barrier call price.
///
/// Parameters
/// ----------
/// spot : float
///     Current spot price `S`.
/// strike : float
///     Strike price `K`.
/// barrier : float
///     Barrier level.
/// r : float
///     Domestic / risk-free rate (continuously compounded, decimal).
/// q : float
///     Dividend yield or foreign rate (continuously compounded, decimal).
/// sigma : float
///     Annualized volatility (decimal, e.g. ``0.20`` for 20%).
/// t : float
///     Time to expiry in years.
/// direction : str
///     ``"up"`` or ``"down"`` (relative to spot / barrier).
/// knock : str
///     ``"in"`` (knock-in) or ``"out"`` (knock-out).
///
/// Returns
/// -------
/// float
///     Per-unit option price.
///
/// Sources
/// -------
/// - Reiner-Rubinstein (1991): see docs/REFERENCES.md#reiner-rubinstein-1991
#[pyfunction(name = "barrier_call")]
#[pyo3(signature = (spot, strike, barrier, r, q, sigma, t, direction, knock))]
#[allow(clippy::too_many_arguments)]
fn barrier_call_wrapper(
    spot: f64,
    strike: f64,
    barrier: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    direction: &str,
    knock: &str,
) -> PyResult<f64> {
    barrier_call_str(spot, strike, barrier, t, r, q, sigma, direction, knock)
        .map_err(crate::errors::core_to_py)
}

/// Arithmetic (Turnbull-Wakeman) or geometric (Kemna-Vorst) Asian option call.
///
/// Parameters
/// ----------
/// spot : float
///     Current spot price `S`.
/// strike : float
///     Strike price `K`.
/// r : float
///     Domestic / risk-free rate (continuously compounded, decimal).
/// q : float
///     Dividend yield or foreign rate (continuously compounded, decimal).
/// sigma : float
///     Annualized volatility (decimal, e.g. ``0.20`` for 20%).
/// t : float
///     Time to expiry in years.
/// num_fixings : int
///     Number of averaging fixings.
/// averaging : str, optional
///     ``"arithmetic"`` (Turnbull-Wakeman, default) or ``"geometric"``
///     (Kemna-Vorst exact).
/// is_call : bool, optional
///     ``True`` for call (default), ``False`` for put.
///
/// Sources
/// -------
/// - Kemna-Vorst (1990): see docs/REFERENCES.md#kemna-vorst-1990
/// - Turnbull-Wakeman (1991): see docs/REFERENCES.md#turnbull-wakeman-1991
#[pyfunction(name = "asian_option_price")]
#[pyo3(signature = (spot, strike, r, q, sigma, t, num_fixings, averaging="arithmetic", is_call=true))]
#[allow(clippy::too_many_arguments)]
fn asian_option_wrapper(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    num_fixings: usize,
    averaging: &str,
    is_call: bool,
) -> PyResult<f64> {
    asian_option_price_str(
        spot,
        strike,
        t,
        r,
        q,
        sigma,
        num_fixings,
        averaging,
        OptionType::from(is_call),
    )
    .map_err(crate::errors::core_to_py)
}

/// Conze-Viswanathan lookback option price.
///
/// Parameters
/// ----------
/// spot : float
///     Current spot price `S`.
/// strike : float
///     Strike price `K`. Ignored when ``strike_type`` is ``"floating"``.
/// r : float
///     Domestic / risk-free rate (continuously compounded, decimal).
/// q : float
///     Dividend yield or foreign rate (continuously compounded, decimal).
/// sigma : float
///     Annualized volatility (decimal, e.g. ``0.20`` for 20%).
/// t : float
///     Time to expiry in years.
/// extremum : float
///     Observed historical extremum — max for fixed-strike call / floating-
///     strike put, min for fixed-strike put / floating-strike call. For a
///     fresh option with no observation, use ``spot``.
/// strike_type : str, optional
///     ``"fixed"`` (default) or ``"floating"``.
/// is_call : bool, optional
///     ``True`` for call (default), ``False`` for put.
///
/// Sources
/// -------
/// - Conze-Viswanathan (1991): see docs/REFERENCES.md#conze-viswanathan-1991
#[pyfunction(name = "lookback_option_price")]
#[pyo3(signature = (spot, strike, r, q, sigma, t, extremum, strike_type="fixed", is_call=true))]
#[allow(clippy::too_many_arguments)]
fn lookback_option_wrapper(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    extremum: f64,
    strike_type: &str,
    is_call: bool,
) -> PyResult<f64> {
    lookback_option_price_str(
        spot,
        strike,
        t,
        r,
        q,
        sigma,
        extremum,
        strike_type,
        OptionType::from(is_call),
    )
    .map_err(crate::errors::core_to_py)
}

/// Quanto option (cross-currency, FX-adjusted) price in domestic currency.
///
/// Parameters
/// ----------
/// spot : float
///     Spot price of the foreign asset in foreign currency.
/// strike : float
///     Strike in foreign currency.
/// t : float
///     Time to expiry in years.
/// rate_domestic, rate_foreign : float
///     Continuously-compounded domestic and foreign rates.
/// div_yield : float
///     Foreign asset dividend yield.
/// vol_asset : float
///     Foreign asset volatility.
/// vol_fx : float
///     Domestic/foreign FX volatility.
/// correlation : float
///     Correlation between asset and FX returns (``[-1, 1]``).
/// is_call : bool, optional
///     ``True`` for call (default), ``False`` for put.
///
/// Raises
/// ------
/// ValueError
///     If the inputs produce a non-finite price.
///
/// Sources
/// -------
/// - Garman-Kohlhagen (1983): see docs/REFERENCES.md#garman-kohlhagen-1983
#[pyfunction(name = "quanto_option_price")]
#[pyo3(signature = (spot, strike, t, rate_domestic, rate_foreign, div_yield, vol_asset, vol_fx, correlation, is_call=true))]
#[allow(clippy::too_many_arguments)]
fn quanto_option_wrapper(
    spot: f64,
    strike: f64,
    t: f64,
    rate_domestic: f64,
    rate_foreign: f64,
    div_yield: f64,
    vol_asset: f64,
    vol_fx: f64,
    correlation: f64,
    is_call: bool,
) -> PyResult<f64> {
    quanto_option_price(
        spot,
        strike,
        t,
        rate_domestic,
        rate_foreign,
        div_yield,
        vol_asset,
        vol_fx,
        correlation,
        OptionType::from(is_call),
    )
    .map_err(crate::errors::core_to_py)
}

/// Register the analytic option primitives on the models submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(bs_price_wrapper, m)?)?;
    m.add_function(wrap_pyfunction!(vanilla_expiry_payoff_wrapper, m)?)?;
    m.add_function(wrap_pyfunction!(bs_greeks_wrapper, m)?)?;
    m.add_function(wrap_pyfunction!(bs_implied_vol_wrapper, m)?)?;
    m.add_function(wrap_pyfunction!(black76_implied_vol_wrapper, m)?)?;
    m.add_function(wrap_pyfunction!(barrier_call_wrapper, m)?)?;
    m.add_function(wrap_pyfunction!(asian_option_wrapper, m)?)?;
    m.add_function(wrap_pyfunction!(lookback_option_wrapper, m)?)?;
    m.add_function(wrap_pyfunction!(quanto_option_wrapper, m)?)?;
    Ok(())
}

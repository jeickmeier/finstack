//! Python bindings for Monte Carlo Greek estimators.
//!
//! Thin wrappers around the canonical GBM European finite-difference
//! convenience entry points in `finstack_quant_monte_carlo::greeks::gbm_european`.

use super::engine::resolve_currency;
use crate::errors::core_to_py;
use finstack_quant_monte_carlo::greeks::gbm_european::{
    finite_diff_delta_crn_gbm, finite_diff_delta_gbm, finite_diff_gamma_crn_gbm,
    finite_diff_gamma_gbm, GbmEuropeanFdSpec,
};
use pyo3::prelude::*;

#[allow(clippy::too_many_arguments)]
fn spec_from_args(
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
    num_paths: Option<usize>,
    seed: Option<u64>,
    num_steps: Option<usize>,
    bump_size: Option<f64>,
    option_type: Option<&str>,
    currency: Option<&Bound<'_, PyAny>>,
) -> PyResult<GbmEuropeanFdSpec> {
    Ok(GbmEuropeanFdSpec {
        spot,
        strike,
        rate,
        dividend_yield: div_yield,
        volatility: vol,
        expiry,
        num_paths,
        seed,
        num_steps,
        bump_size,
        option_type: option_type.map(str::to_owned),
        currency: Some(resolve_currency(currency)?),
    })
}

/// Finite-difference delta for a vanilla European option under GBM.
///
/// Reports the conservative independence-bound stderr. Use [`finite_diff_delta_crn`]
/// for paired common-random-number stderr.
///
/// `spot` and the relative `bump_size` must be finite and positive. The
/// absolute bump is `max(abs(spot) * bump_size, 1e-8)`, and the symmetric
/// down-bumped state must remain at least `1e-12`.
///
/// Returns `(delta, stderr)`.
///
/// Raises `ValueError` when the inputs cannot form that symmetric central
/// stencil or another pricing input is invalid.
#[pyfunction]
#[allow(clippy::too_many_arguments)]
#[pyo3(signature = (
    spot, strike, rate, div_yield, vol, expiry,
    num_paths=None, seed=None, num_steps=None,
    bump_size=None, option_type=None, currency=None,
))]
fn finite_diff_delta(
    py: Python<'_>,
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
    num_paths: Option<usize>,
    seed: Option<u64>,
    num_steps: Option<usize>,
    bump_size: Option<f64>,
    option_type: Option<&str>,
    currency: Option<&Bound<'_, PyAny>>,
) -> PyResult<(f64, f64)> {
    let spec = spec_from_args(
        spot,
        strike,
        rate,
        div_yield,
        vol,
        expiry,
        num_paths,
        seed,
        num_steps,
        bump_size,
        option_type,
        currency,
    )?;
    py.detach(|| finite_diff_delta_gbm(spec)).map_err(core_to_py)
}

/// Finite-difference delta with paired common-random-number stderr.
///
/// `spot` and the relative `bump_size` must be finite and positive. The
/// absolute bump is `max(abs(spot) * bump_size, 1e-8)`, and the symmetric
/// down-bumped state must remain at least `1e-12`.
///
/// Returns `(delta, stderr)`.
///
/// Raises `ValueError` when the inputs cannot form that symmetric central
/// stencil or another pricing input is invalid.
#[pyfunction]
#[allow(clippy::too_many_arguments)]
#[pyo3(signature = (
    spot, strike, rate, div_yield, vol, expiry,
    num_paths=None, seed=None, num_steps=None,
    bump_size=None, option_type=None, currency=None,
))]
fn finite_diff_delta_crn(
    py: Python<'_>,
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
    num_paths: Option<usize>,
    seed: Option<u64>,
    num_steps: Option<usize>,
    bump_size: Option<f64>,
    option_type: Option<&str>,
    currency: Option<&Bound<'_, PyAny>>,
) -> PyResult<(f64, f64)> {
    let spec = spec_from_args(
        spot,
        strike,
        rate,
        div_yield,
        vol,
        expiry,
        num_paths,
        seed,
        num_steps,
        bump_size,
        option_type,
        currency,
    )?;
    py.detach(|| finite_diff_delta_crn_gbm(spec))
        .map_err(core_to_py)
}

/// Finite-difference gamma (independence-bound stderr).
///
/// `spot` and the relative `bump_size` must be finite and positive. The
/// absolute bump is `max(abs(spot) * bump_size, 1e-8)`, and the symmetric
/// down-bumped state must remain at least `1e-12`.
///
/// Returns `(gamma, stderr)`.
///
/// Raises `ValueError` when the inputs cannot form that symmetric central
/// stencil or another pricing input is invalid.
#[pyfunction]
#[allow(clippy::too_many_arguments)]
#[pyo3(signature = (
    spot, strike, rate, div_yield, vol, expiry,
    num_paths=None, seed=None, num_steps=None,
    bump_size=None, option_type=None, currency=None,
))]
fn finite_diff_gamma(
    py: Python<'_>,
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
    num_paths: Option<usize>,
    seed: Option<u64>,
    num_steps: Option<usize>,
    bump_size: Option<f64>,
    option_type: Option<&str>,
    currency: Option<&Bound<'_, PyAny>>,
) -> PyResult<(f64, f64)> {
    let spec = spec_from_args(
        spot,
        strike,
        rate,
        div_yield,
        vol,
        expiry,
        num_paths,
        seed,
        num_steps,
        bump_size,
        option_type,
        currency,
    )?;
    py.detach(|| finite_diff_gamma_gbm(spec)).map_err(core_to_py)
}

/// Finite-difference gamma with paired common-random-number stderr.
///
/// `spot` and the relative `bump_size` must be finite and positive. The
/// absolute bump is `max(abs(spot) * bump_size, 1e-8)`, and the symmetric
/// down-bumped state must remain at least `1e-12`.
///
/// Returns `(gamma, stderr)`.
///
/// Raises `ValueError` when the inputs cannot form that symmetric central
/// stencil or another pricing input is invalid.
#[pyfunction]
#[allow(clippy::too_many_arguments)]
#[pyo3(signature = (
    spot, strike, rate, div_yield, vol, expiry,
    num_paths=None, seed=None, num_steps=None,
    bump_size=None, option_type=None, currency=None,
))]
fn finite_diff_gamma_crn(
    py: Python<'_>,
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
    num_paths: Option<usize>,
    seed: Option<u64>,
    num_steps: Option<usize>,
    bump_size: Option<f64>,
    option_type: Option<&str>,
    currency: Option<&Bound<'_, PyAny>>,
) -> PyResult<(f64, f64)> {
    let spec = spec_from_args(
        spot,
        strike,
        rate,
        div_yield,
        vol,
        expiry,
        num_paths,
        seed,
        num_steps,
        bump_size,
        option_type,
        currency,
    )?;
    py.detach(|| finite_diff_gamma_crn_gbm(spec))
        .map_err(core_to_py)
}

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(finite_diff_delta, m)?)?;
    m.add_function(wrap_pyfunction!(finite_diff_delta_crn, m)?)?;
    m.add_function(wrap_pyfunction!(finite_diff_gamma, m)?)?;
    m.add_function(wrap_pyfunction!(finite_diff_gamma_crn, m)?)?;
    Ok(())
}

//! Thin adapter over the workspace-canonical Heston characteristic function.
//!
//! The "Little Heston Trap" algebra (Albrecher et al. 2007) lives once, in
//! [`crate::volatility::heston`]. This module only maps
//! this crate's [`HestonPricingParams`] (which carries `r`/`q` alongside the model
//! parameters) onto the canonical parameter type and re-exports the status enum, so
//! the Fourier drivers here keep their own quadrature while sharing the
//! algebra.

use super::HestonPricingParams;
use num_complex::Complex;

pub(super) use crate::volatility::heston::HestonCfStatus;

/// Heston probability characteristic function ψ_j(φ) for j ∈ {1, 2}.
///
/// Forwards to
/// [`crate::volatility::heston::heston_pj_characteristic_function`];
/// see that function for the formulation and the meaning of the returned
/// [`HestonCfStatus`].
///
/// # Arguments
///
/// * `j` - Probability index (1 or 2)
/// * `phi` - Fourier variable
/// * `time` - Time to maturity
/// * `log_spot` - Natural log of spot price
/// * `params` - Heston model parameters (including `r` and `q`)
///
/// # Returns
///
/// `(ψ_j(φ), status)`: the complex value (zeroed on overflow/underflow) and a
/// [`HestonCfStatus`] telling the caller whether a zero is legitimate
/// underflow or corruption.
pub(super) fn heston_pj_characteristic_function(
    j: u8,
    phi: f64,
    time: f64,
    log_spot: f64,
    params: &HestonPricingParams,
) -> (Complex<f64>, HestonCfStatus) {
    crate::volatility::heston::heston_pj_characteristic_function(
        j,
        phi,
        log_spot,
        params.r,
        params.q,
        time,
        &params.model,
    )
}

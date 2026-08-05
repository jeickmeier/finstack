//! Thin adapter over the workspace-canonical Heston characteristic function.
//!
//! The "Little Heston Trap" algebra (Albrecher et al. 2007) lives once, in
//! [`finstack_quant_core::math::volatility::heston`]. This module only maps
//! this crate's [`HestonParams`] (which carries `r`/`q` alongside the model
//! parameters) onto core's parameter type and re-exports the status enum, so
//! the Fourier drivers here keep their own quadrature while sharing the
//! algebra.

use super::HestonParams;
use num_complex::Complex;

pub(super) use finstack_quant_core::math::volatility::heston::HestonCfStatus;

/// Heston probability characteristic function ψ_j(φ) for j ∈ {1, 2}.
///
/// Forwards to
/// [`finstack_quant_core::math::volatility::heston::heston_pj_characteristic_function`];
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
    params: &HestonParams,
) -> (Complex<f64>, HestonCfStatus) {
    // `HestonParams::new` in this crate already validates the model
    // parameters, so the core constructor cannot fail here; report a corrupt
    // node rather than panicking if that ever changes.
    let Ok(core_params) = finstack_quant_core::math::volatility::heston::HestonParams::new(
        params.v0,
        params.kappa,
        params.theta,
        params.sigma_v,
        params.rho,
    ) else {
        return (Complex::new(0.0, 0.0), HestonCfStatus::Overflow);
    };

    finstack_quant_core::math::volatility::heston::heston_pj_characteristic_function(
        j,
        phi,
        log_spot,
        params.r,
        params.q,
        time,
        &core_params,
    )
}

//! WASM bindings for equality-constrained least-squares regression.
//!
//! Binds `finstack_quant_analytics::regression::constrained_least_squares`
//! (Jeet & Partani 2023, Appendix A). Unlike the JSON-shim pattern used
//! elsewhere in this crate's bindings, this is a plain numeric API: no
//! portfolio, sector, or attribution concepts are involved, so numeric
//! vectors travel as `Float64Array` rather than through a JSON envelope.

use js_sys::Float64Array;
use wasm_bindgen::prelude::*;

use crate::utils::to_js_err;

use super::support::parse_f64_vec;

/// Fit factor returns satisfying the equality constraint `w'Xf = w'r`.
///
/// Binds Rust `constrained_least_squares`: adds the minimal Lagrangian
/// correction to an unconstrained OLS fit so the corrected factor returns
/// exactly reproduce the weighted realized return `w'r`. Typically used to
/// fit the benchmark factor returns consumed by
/// `portfolio.factorBrinsonAttribution`, which requires factor returns
/// satisfying that same completeness condition.
/// @param exposures - Row-major factor exposure matrix, `n_assets x n_factors`: asset i's exposure to factor j is `exposures[i * n_factors + j]`.
/// @param nFactors - Number of factor columns in `exposures`; must be positive.
/// @param returns - Realized asset returns, length `n_assets` (defines `n_assets`).
/// @param weights - Holding weights whose weighted return `w'r` must be fully reproduced by `w'Xf` (e.g. benchmark weights for a benchmark-return attribution).
#[wasm_bindgen(js_name = constrainedLeastSquares)]
pub fn constrained_least_squares(
    exposures: JsValue,
    n_factors: usize,
    returns: JsValue,
    weights: JsValue,
) -> Result<Float64Array, JsValue> {
    let exposures = parse_f64_vec(exposures)?;
    let returns = parse_f64_vec(returns)?;
    let weights = parse_f64_vec(weights)?;
    let f = finstack_quant_analytics::regression::constrained_least_squares(
        &exposures, n_factors, &returns, &weights,
    )
    .map_err(to_js_err)?;
    Ok(Float64Array::from(f.as_slice()))
}

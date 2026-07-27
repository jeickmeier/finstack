//! Python bindings for equality-constrained least-squares regression.
//!
//! Binds `finstack_quant_analytics::regression::constrained_least_squares`
//! (Jeet & Partani 2023, Appendix A). Unlike the JSON-shim pattern used
//! elsewhere in the bindings, this is a plain numeric API: no portfolio,
//! sector, or attribution concepts are involved, so lists of floats travel
//! directly rather than through a JSON envelope.

use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::errors::analytics_to_py;

/// Fit factor returns satisfying the equality constraint `w'Xf = w'r`.
///
/// Binds Rust
/// `finstack_quant_analytics::regression::constrained_least_squares`: adds
/// the minimal Lagrangian correction to an unconstrained OLS fit so the
/// corrected factor returns exactly reproduce the weighted realized return
/// `w'r`. Typically used to fit the benchmark factor returns consumed by
/// :func:`finstack_quant.portfolio.factor_brinson_attribution`, which
/// requires factor returns satisfying that same completeness condition.
///
/// Parameters
/// ----------
/// exposures : list[float]
///     Row-major factor exposure matrix, ``n_assets x n_factors``: asset
///     ``i``'s exposure to factor ``j`` is ``exposures[i * n_factors + j]``.
/// n_factors : int
///     Number of factor columns in ``exposures``; must be positive.
/// returns : list[float]
///     Realized asset returns, length ``n_assets`` (defines ``n_assets``).
/// weights : list[float]
///     Holding weights whose weighted return ``w'r`` must be fully
///     reproduced by ``w'Xf`` (e.g. benchmark weights for a
///     benchmark-return attribution); length ``n_assets``.
///
/// Returns
/// -------
/// list[float]
///     Constrained factor returns ``f``, one per factor, satisfying
///     ``w'Xf = w'r`` to numerical precision.
///
/// Raises
/// ------
/// AnalyticsError
///     If ``n_factors`` is zero, ``returns`` is empty, ``exposures`` or
///     ``weights`` has the wrong length, any input value is non-finite, the
///     design matrix is rank-deficient, or ``w`` and the constraint
///     direction are numerically orthogonal so the constraint cannot be
///     restored by any scalar correction.
///
/// Sources
/// -------
/// See ``docs/REFERENCES.md#jeet-partani-2023``.
///
/// Examples
/// --------
/// >>> from finstack_quant.analytics import constrained_least_squares
/// >>> exposures = [0.0, 1.0, 1.0, 0.0, 0.0, 1.0]
/// >>> returns = [0.05, 0.02, 0.01]
/// >>> weights = [0.6, 0.3, 0.1]
/// >>> f = constrained_least_squares(exposures, 2, returns, weights)
/// >>> len(f)
/// 2
#[pyfunction]
#[pyo3(text_signature = "(exposures, n_factors, returns, weights)")]
fn constrained_least_squares(
    py: Python<'_>,
    exposures: Vec<f64>,
    n_factors: usize,
    returns: Vec<f64>,
    weights: Vec<f64>,
) -> PyResult<Vec<f64>> {
    py.detach(move || {
        finstack_quant_analytics::regression::constrained_least_squares(
            &exposures, n_factors, &returns, &weights,
        )
        .map_err(analytics_to_py)
    })
}

/// Register regression functions on the analytics submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(constrained_least_squares, m)?)?;
    Ok(())
}

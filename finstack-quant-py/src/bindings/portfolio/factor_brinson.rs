//! Python bindings for Factor-Brinson unified attribution (Jeet & Partani 2023).
//!
//! Binds `finstack_quant_portfolio::factor_brinson_attribution`. The input
//! payload and result are JSON strings matching the Rust `serde` shapes;
//! `factor_returns` is a plain numeric list rather than embedded in the JSON
//! payload because it is typically produced by
//! `finstack_quant.analytics.constrained_least_squares`.

use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::errors::{portfolio_to_py, serde_json_to_py};

/// Compute Jeet-Partani (2023) factor-Brinson unified attribution.
///
/// Binds Rust `finstack_quant_portfolio::factor_brinson_attribution`:
/// generalizes classical Brinson-Fachler allocation/selection to continuous
/// factor exposures by replacing the sector partition with a factor-exposure
/// matrix and a caller-supplied benchmark factor-return vector.
///
/// Parameters
/// ----------
/// input_json : str
///     JSON ``FactorBrinsonInput`` with ``asset_ids``, ``asset_returns``,
///     ``exposures`` (row-major ``n_assets x n_factors``), ``factor_names``,
///     ``portfolio_weights`` and ``benchmark_weights``. Each weight vector
///     must sum to ``1.0`` within ``1e-6``; weights may be negative (short
///     positions).
/// factor_returns : list[float]
///     Caller-supplied benchmark factor returns ``f_b``, length
///     ``input.factor_names``. Typically fit with
///     :func:`finstack_quant.analytics.constrained_least_squares` (using
///     benchmark weights) so the completeness condition below holds.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``FactorBrinsonResult`` with ``allocation``,
///     ``selection``, and their per-factor / per-asset breakdowns.
///
/// Raises
/// ------
/// PortfolioError
///     If any array has the wrong length relative to ``n_assets`` /
///     ``n_factors``, either ``n_assets`` or ``n_factors`` is zero, any
///     input value is non-finite, portfolio or benchmark weights don't sum
///     to ``1.0`` within tolerance, or the Jeet-Partani completeness
///     residual ``|h_b'eps_b|`` exceeds tolerance — meaning the supplied
///     ``factor_returns`` do not fully explain the benchmark return, so
///     ``allocation``/``selection`` would not be valid Brinson effects. The
///     error directs callers to
///     :func:`finstack_quant.analytics.constrained_least_squares`.
/// ValueError
///     If ``input_json`` is malformed or carries unknown fields.
///
/// Sources
/// -------
/// See ``docs/REFERENCES.md#jeet-partani-2023``.
///
/// Examples
/// --------
/// >>> from finstack_quant.portfolio import factor_brinson_attribution
/// >>> result_json = factor_brinson_attribution(input_json, [0.02, 0.31 / 7.0])  # doctest: +SKIP
#[pyfunction]
#[pyo3(text_signature = "(input_json, factor_returns)")]
fn factor_brinson_attribution(
    py: Python<'_>,
    input_json: &str,
    factor_returns: Vec<f64>,
) -> PyResult<String> {
    let input_json = input_json.to_owned();
    py.detach(move || {
        let input: finstack_quant_portfolio::FactorBrinsonInput = serde_json::from_str(&input_json)
            .map_err(|err| serde_json_to_py(err, "invalid factor-Brinson input JSON"))?;
        let result = finstack_quant_portfolio::factor_brinson_attribution(&input, &factor_returns)
            .map_err(portfolio_to_py)?;
        serde_json::to_string(&result)
            .map_err(|err| serde_json_to_py(err, "serialize FactorBrinsonResult"))
    })
}

/// Register factor-Brinson attribution functions on the portfolio submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(factor_brinson_attribution, m)?)?;
    Ok(())
}

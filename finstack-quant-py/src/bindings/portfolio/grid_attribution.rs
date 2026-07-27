//! Python bindings for hierarchical duration-cell x sector grid attribution.
//!
//! Binds `finstack_quant_portfolio::grid_attribution` (Dynkin, Hyman &
//! Vankudre 1998, Appendix A). Inputs and outputs are JSON strings matching
//! the Rust `serde` shapes, same pattern as the Brinson bindings in
//! `crate::bindings::portfolio::brinson`.

use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::errors::{portfolio_to_py, serde_json_to_py};

/// Compute a single-period hierarchical duration-cell x sector grid attribution.
///
/// Binds Rust `finstack_quant_portfolio::grid_attribution` (Dynkin, Hyman &
/// Vankudre 1998, Appendix A): decomposes active return into a per-cell
/// curve (positioning) effect, a within-cell sector allocation effect, and a
/// security-selection residual per (cell, sector).
///
/// Parameters
/// ----------
/// portfolio_json : str
///     JSON array of ``GridPosition`` objects (``cell``, ``sector``,
///     ``weight``, ``total_return``) for the portfolio side; weights must
///     sum to ``1.0`` within ``1e-6``.
/// benchmark_json : str
///     JSON array of ``GridPosition`` objects for the benchmark side; same
///     weight-sum requirement.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``GridAttributionResult`` whose ``total_curve``,
///     ``total_sector`` and ``total_selection`` sum to ``active_return`` to
///     floating-point precision for well-conditioned inputs; among *accepted*
///     inputs (those that clear the near-zero-net-weight check below), the
///     reconciliation residual grows the closer any bucket's net weight sits
///     to that check's own rejection boundary — see the Rust module docs'
///     measured residuals for magnitudes.
///
/// Raises
/// ------
/// PortfolioError
///     If any weight or return is non-finite, either side's weights don't
///     sum to ``1.0`` within tolerance, or a (cell) or (cell, sector)
///     bucket has positions on a side but nets to a weight that is zero, or
///     numerically near zero (within ``1e-6`` relative to its own gross
///     weight), which is undefined-or-explosive to attribute (the error
///     names the bucket and the side).
/// ValueError
///     If either JSON argument is malformed or carries unknown fields.
///
/// Sources
/// -------
/// See ``docs/REFERENCES.md#dynkin-hyman-vankudre-1998``.
///
/// Examples
/// --------
/// >>> from finstack_quant.portfolio import grid_attribution
/// >>> result_json = grid_attribution(portfolio_json, benchmark_json)  # doctest: +SKIP
#[pyfunction]
#[pyo3(text_signature = "(portfolio_json, benchmark_json)")]
fn grid_attribution(
    py: Python<'_>,
    portfolio_json: &str,
    benchmark_json: &str,
) -> PyResult<String> {
    let portfolio_json = portfolio_json.to_owned();
    let benchmark_json = benchmark_json.to_owned();
    py.detach(move || {
        let portfolio: Vec<finstack_quant_portfolio::GridPosition> =
            serde_json::from_str(&portfolio_json)
                .map_err(|err| serde_json_to_py(err, "invalid grid portfolio JSON"))?;
        let benchmark: Vec<finstack_quant_portfolio::GridPosition> =
            serde_json::from_str(&benchmark_json)
                .map_err(|err| serde_json_to_py(err, "invalid grid benchmark JSON"))?;
        let result = finstack_quant_portfolio::grid_attribution(&portfolio, &benchmark)
            .map_err(portfolio_to_py)?;
        serde_json::to_string(&result)
            .map_err(|err| serde_json_to_py(err, "serialize GridAttributionResult"))
    })
}

/// Carino-link multi-period hierarchical grid attribution results.
///
/// Binds Rust `finstack_quant_portfolio::grid_carino_link` (Carino 1999):
/// applies Carino smoothing to a chronological sequence of single-period
/// `grid_attribution` results so the three top-level effects
/// (``linked_curve``, ``linked_sector``, ``linked_selection``) sum exactly
/// to the geometrically compounded active return. Only the three top-level
/// effects are linked; per-cell / per-(cell, sector) multi-period linking is
/// out of scope.
///
/// Parameters
/// ----------
/// periods_json : str
///     JSON array of ``GridAttributionResult`` objects, in chronological
///     order, each the parsed output of :func:`grid_attribution`.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``GridCarinoLinkedResult``.
///
/// Raises
/// ------
/// PortfolioError
///     If ``periods_json`` is an empty array, any period's portfolio or
///     benchmark return is non-finite, or any per-period or compounded
///     return is at or below -100% (outside the Carino domain).
/// ValueError
///     If ``periods_json`` is malformed or does not match the
///     ``GridAttributionResult`` schema.
///
/// Sources
/// -------
/// See ``docs/REFERENCES.md#carino-1999`` and
/// ``docs/REFERENCES.md#dynkin-hyman-vankudre-1998``.
///
/// Examples
/// --------
/// >>> import json
/// >>> from finstack_quant.portfolio import grid_attribution, grid_carino_link
/// >>> jan = grid_attribution(jan_portfolio_json, jan_benchmark_json)  # doctest: +SKIP
/// >>> feb = grid_attribution(feb_portfolio_json, feb_benchmark_json)  # doctest: +SKIP
/// >>> linked_json = grid_carino_link(json.dumps([json.loads(jan), json.loads(feb)]))  # doctest: +SKIP
#[pyfunction]
#[pyo3(text_signature = "(periods_json)")]
fn grid_carino_link(py: Python<'_>, periods_json: &str) -> PyResult<String> {
    let periods_json = periods_json.to_owned();
    py.detach(move || {
        let periods: Vec<finstack_quant_portfolio::GridAttributionResult> =
            serde_json::from_str(&periods_json)
                .map_err(|err| serde_json_to_py(err, "invalid grid periods JSON"))?;
        let result =
            finstack_quant_portfolio::grid_carino_link(&periods).map_err(portfolio_to_py)?;
        serde_json::to_string(&result)
            .map_err(|err| serde_json_to_py(err, "serialize GridCarinoLinkedResult"))
    })
}

/// Register grid attribution functions on the portfolio submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(grid_attribution, m)?)?;
    m.add_function(wrap_pyfunction!(grid_carino_link, m)?)?;
    Ok(())
}

//! Python bindings for Campisi fixed-income benchmark attribution.
//!
//! Inputs and outputs are JSON strings matching the Rust `serde` shapes so
//! the binding layer stays a conversion shim around the canonical Rust
//! analytics (same pattern as the Brinson bindings in [`super::brinson`]).

use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::errors::{portfolio_to_py, serde_json_to_py};

/// Compute a single-period Campisi fixed-income attribution from JSON.
///
/// Parameters
/// ----------
/// portfolio_json : str
///     JSON array of ``FiPositionSnapshot`` objects (``sector``, ``weight``,
///     ``total_return``, ``yield_annual``, ``modified_duration``,
///     ``spread_duration``, ``spread``, ``delta_treasury_yield``,
///     ``delta_spread``).
/// benchmark_json : str
///     JSON array of ``FiPositionSnapshot`` objects for the benchmark.
/// config_json : str
///     JSON ``FiAttributionConfig`` with ``period_years`` and ``spread_mode``
///     (``"spread_duration"`` or ``"dts"``); both fields are required.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``FiAttributionResult``.
#[pyfunction]
#[pyo3(text_signature = "(portfolio_json, benchmark_json, config_json)")]
fn campisi_attribution(
    py: Python<'_>,
    portfolio_json: &str,
    benchmark_json: &str,
    config_json: &str,
) -> PyResult<String> {
    let portfolio_json = portfolio_json.to_owned();
    let benchmark_json = benchmark_json.to_owned();
    let config_json = config_json.to_owned();
    py.detach(move || {
        let portfolio: Vec<finstack_quant_portfolio::FiPositionSnapshot> =
            serde_json::from_str(&portfolio_json)
                .map_err(|err| serde_json_to_py(err, "invalid Campisi portfolio JSON"))?;
        let benchmark: Vec<finstack_quant_portfolio::FiPositionSnapshot> =
            serde_json::from_str(&benchmark_json)
                .map_err(|err| serde_json_to_py(err, "invalid Campisi benchmark JSON"))?;
        let config: finstack_quant_portfolio::FiAttributionConfig =
            serde_json::from_str(&config_json)
                .map_err(|err| serde_json_to_py(err, "invalid Campisi config JSON"))?;
        let result = finstack_quant_portfolio::campisi_attribution(&portfolio, &benchmark, &config)
            .map_err(portfolio_to_py)?;
        serde_json::to_string(&result)
            .map_err(|err| serde_json_to_py(err, "serialize Campisi result"))
    })
}

/// Carino-link already-computed single-period Campisi attribution results.
///
/// Binds Rust `finstack_quant_portfolio::campisi_carino_link`. Because each
/// period carries its own already-applied `period_years`, periods of
/// *different* lengths (e.g. act/365 calendar months) link correctly here;
/// use this entry point whenever the periods are not all the same length.
///
/// Parameters
/// ----------
/// periods_json : str
///     JSON array of ``FiAttributionResult`` objects, in chronological order,
///     as returned by :func:`campisi_attribution`.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``FiCarinoLinkedResult``.
#[pyfunction]
#[pyo3(text_signature = "(periods_json)")]
fn campisi_carino_link(py: Python<'_>, periods_json: &str) -> PyResult<String> {
    let periods_json = periods_json.to_owned();
    py.detach(move || {
        let periods: Vec<finstack_quant_portfolio::FiAttributionResult> =
            serde_json::from_str(&periods_json)
                .map_err(|err| serde_json_to_py(err, "invalid Campisi period results JSON"))?;
        let result =
            finstack_quant_portfolio::campisi_carino_link(&periods).map_err(portfolio_to_py)?;
        serde_json::to_string(&result)
            .map_err(|err| serde_json_to_py(err, "serialize Campisi linked result"))
    })
}

/// Compute Carino-linked multi-period Campisi attribution from period JSON.
///
/// Binds Rust `finstack_quant_portfolio::campisi_carino_link_from_snapshots`.
/// One shared config — hence one shared ``period_years`` — is applied to every
/// period, so this entry point is only correct for equal-length periods.
///
/// Parameters
/// ----------
/// periods_json : str
///     JSON array of ``FiPeriodInput`` objects, each with ``portfolio`` and
///     ``benchmark`` arrays of ``FiPositionSnapshot``.
/// config_json : str
///     JSON ``FiAttributionConfig`` shared across periods.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``FiCarinoLinkedResult``.
#[pyfunction]
#[pyo3(text_signature = "(periods_json, config_json)")]
fn campisi_carino_link_from_snapshots(
    py: Python<'_>,
    periods_json: &str,
    config_json: &str,
) -> PyResult<String> {
    let periods_json = periods_json.to_owned();
    let config_json = config_json.to_owned();
    py.detach(move || {
        let periods: Vec<finstack_quant_portfolio::FiPeriodInput> =
            serde_json::from_str(&periods_json)
                .map_err(|err| serde_json_to_py(err, "invalid Campisi periods JSON"))?;
        let config: finstack_quant_portfolio::FiAttributionConfig =
            serde_json::from_str(&config_json)
                .map_err(|err| serde_json_to_py(err, "invalid Campisi config JSON"))?;
        let result =
            finstack_quant_portfolio::campisi_carino_link_from_snapshots(&periods, &config)
                .map_err(portfolio_to_py)?;
        serde_json::to_string(&result)
            .map_err(|err| serde_json_to_py(err, "serialize Campisi linked result"))
    })
}

/// Reconcile the five Campisi effect totals against the active return.
///
/// Binds the Rust method
/// `finstack_quant_portfolio::FiAttributionResult::reconciliation_check`.
/// The decomposition reconciles by construction (selection is the residual),
/// so this is a floating-point sanity gate rather than a model check; without
/// it Python and JavaScript callers must re-sum the five totals by hand.
///
/// Parameters
/// ----------
/// result_json : str
///     JSON ``FiAttributionResult``, as returned by :func:`campisi_attribution`.
/// tolerance : float
///     Absolute tolerance in return units (``1e-10`` is appropriate for
///     return-space values).
///
/// Returns
/// -------
/// str
///     JSON-serialized ``FiReconciliationReport`` with ``total_residual``,
///     ``is_reconciled`` and ``tolerance``.
#[pyfunction]
#[pyo3(text_signature = "(result_json, tolerance)")]
fn campisi_reconciliation_check(
    py: Python<'_>,
    result_json: &str,
    tolerance: f64,
) -> PyResult<String> {
    let result_json = result_json.to_owned();
    py.detach(move || {
        let result: finstack_quant_portfolio::FiAttributionResult =
            serde_json::from_str(&result_json)
                .map_err(|err| serde_json_to_py(err, "invalid Campisi result JSON"))?;
        serde_json::to_string(&result.reconciliation_check(tolerance))
            .map_err(|err| serde_json_to_py(err, "serialize Campisi reconciliation report"))
    })
}

/// Register Campisi attribution functions on the portfolio submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(campisi_attribution, m)?)?;
    m.add_function(wrap_pyfunction!(campisi_carino_link, m)?)?;
    m.add_function(wrap_pyfunction!(campisi_carino_link_from_snapshots, m)?)?;
    m.add_function(wrap_pyfunction!(campisi_reconciliation_check, m)?)?;
    Ok(())
}

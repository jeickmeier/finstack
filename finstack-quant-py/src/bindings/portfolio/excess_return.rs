//! Python bindings for duration-matched credit excess returns.
//!
//! Binds `finstack_quant_portfolio::excess_return`: duration-cell base-return
//! tables built either from a reference universe (Dynkin, Hyman & Vankudre
//! 1998, Appendix B) or from discount-curve snapshots, plus position-level
//! duration-matched excess returns measured against either table. Inputs and
//! outputs are JSON strings matching the Rust `serde` shapes, except the
//! curve-snapshot path, which takes the typed `DiscountCurve` objects
//! directly (same pattern as the XVA bindings in
//! `crate::bindings::margin::xva`).

use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::bindings::core::market_data::curves::PyDiscountCurve;
use crate::errors::{portfolio_to_py, serde_json_to_py};

/// Build a duration-cell base-return table from a reference universe.
///
/// Binds Rust `finstack_quant_portfolio::cell_returns_from_reference`
/// (Dynkin, Hyman & Vankudre 1998, Appendix B): buckets `reference_json` into
/// fixed-width duration cells and averages each cell's member total returns,
/// interpolating interior gaps and flat-extrapolating leading/trailing gaps.
///
/// Parameters
/// ----------
/// reference_json : str
///     JSON array of ``ReferenceReturn`` objects (``duration``,
///     ``total_return``, both decimals with duration in years); must be
///     non-empty. Unknown fields are rejected.
/// base_label : str
///     Label identifying the resulting curve (e.g. ``"UST"``), carried
///     through to the output's ``base_label`` for policy visibility.
/// config_json : str
///     JSON ``CellConfig``; ``width`` is its only field (cell width in
///     years, finite and positive) and is required — there is no default.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``DurationCellTable``.
///
/// Raises
/// ------
/// PortfolioError
///     If ``reference_json`` is empty, ``config.width`` is not finite and
///     positive, any reference entry has a non-finite ``total_return`` or a
///     non-finite/negative ``duration``, the width produces two numerically
///     distinct cells that collide on their one-decimal label, or the
///     largest reference ``duration`` divided by ``config.width`` would
///     require more than an internal sanity bound of cells (100,000; a units
///     mistake, e.g. days instead of years, rather than a legitimate
///     duration grid).
/// ValueError
///     If any JSON argument is malformed or carries unknown fields.
///
/// Sources
/// -------
/// See ``docs/REFERENCES.md#dynkin-hyman-vankudre-1998``.
///
/// Examples
/// --------
/// >>> import json
/// >>> from finstack_quant.portfolio import cell_returns_from_reference
/// >>> reference = [{"duration": 1.0, "total_return": 0.02}]
/// >>> table = json.loads(cell_returns_from_reference(json.dumps(reference), "UST", '{"width": 2.0}'))
/// >>> table["cells"][0]["base_return"]
/// 0.02
#[pyfunction]
#[pyo3(text_signature = "(reference_json, base_label, config_json)")]
fn cell_returns_from_reference(
    py: Python<'_>,
    reference_json: &str,
    base_label: &str,
    config_json: &str,
) -> PyResult<String> {
    let reference_json = reference_json.to_owned();
    let base_label = base_label.to_owned();
    let config_json = config_json.to_owned();
    py.detach(move || {
        let reference: Vec<finstack_quant_portfolio::ReferenceReturn> =
            serde_json::from_str(&reference_json)
                .map_err(|err| serde_json_to_py(err, "invalid reference JSON"))?;
        let config: finstack_quant_portfolio::CellConfig = serde_json::from_str(&config_json)
            .map_err(|err| serde_json_to_py(err, "invalid config JSON"))?;
        let table =
            finstack_quant_portfolio::cell_returns_from_reference(&reference, &base_label, &config)
                .map_err(portfolio_to_py)?;
        serde_json::to_string(&table)
            .map_err(|err| serde_json_to_py(err, "serialize DurationCellTable"))
    })
}

/// Build a duration-cell base-return table from start/end discount curves.
///
/// Binds Rust `finstack_quant_portfolio::cell_returns_from_curves`: each
/// cell's base return is the holding-period return of a hypothetical
/// zero-coupon position bought at the cell midpoint off ``start`` and
/// revalued off ``end`` after ``horizon_years`` have elapsed. Every
/// resulting cell is observed (a curve has a discount factor at every
/// queried point), unlike the reference-universe path in
/// :func:`cell_returns_from_reference`.
///
/// Parameters
/// ----------
/// start : DiscountCurve
///     Discount curve observed at the start of the holding period.
/// end : DiscountCurve
///     Discount curve observed ``horizon_years`` later, at period end.
/// horizon_years : float
///     Length of the holding period, in years; must be finite and positive.
/// max_duration : float
///     Upper bound of the duration grid, in years; must be finite and
///     strictly greater than ``horizon_years``.
/// base_label : str
///     Label identifying the base curve (e.g. ``"UST"``, ``"USD-SOFR"``),
///     stamped into the result purely for policy visibility.
/// config_json : str
///     JSON ``CellConfig``; ``width`` is its only field and is required.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``DurationCellTable``.
///
/// Raises
/// ------
/// PortfolioError
///     If ``config.width`` or ``horizon_years`` is not finite and positive,
///     ``max_duration`` does not strictly exceed ``horizon_years``,
///     ``max_duration`` divided by ``config.width`` would require more than
///     an internal sanity bound of cells (100,000; a units mistake rather
///     than a legitimate duration grid), a cell's midpoint does not exceed
///     ``horizon_years`` (this is unavoidable whenever ``config.width`` is
///     not strictly greater than ``2 * horizon_years``, since the first
///     cell's midpoint is ``config.width / 2``), either curve produces a
///     non-positive/non-finite discount factor at a queried time, or the
///     width produces colliding one-decimal cell labels.
/// ValueError
///     If ``config_json`` is malformed.
///
/// Sources
/// -------
/// See ``docs/REFERENCES.md#dynkin-hyman-vankudre-1998``.
///
/// Examples
/// --------
/// >>> from datetime import date
/// >>> import json
/// >>> from finstack_quant.core.market_data import DiscountCurve
/// >>> from finstack_quant.portfolio import cell_returns_from_curves
/// >>> start = DiscountCurve.flat("start", date(2025, 1, 1), 0.02)
/// >>> end = DiscountCurve.flat("end", date(2025, 4, 1), 0.03)
/// >>> table = json.loads(cell_returns_from_curves(start, end, 0.25, 2.0, "UST", '{"width": 1.0}'))
/// >>> len(table["cells"])
/// 2
#[pyfunction]
#[pyo3(text_signature = "(start, end, horizon_years, max_duration, base_label, config_json)")]
fn cell_returns_from_curves(
    py: Python<'_>,
    start: &PyDiscountCurve,
    end: &PyDiscountCurve,
    horizon_years: f64,
    max_duration: f64,
    base_label: &str,
    config_json: &str,
) -> PyResult<String> {
    let start_curve = std::sync::Arc::clone(&start.inner);
    let end_curve = std::sync::Arc::clone(&end.inner);
    let base_label = base_label.to_owned();
    let config_json = config_json.to_owned();
    py.detach(move || {
        let config: finstack_quant_portfolio::CellConfig = serde_json::from_str(&config_json)
            .map_err(|err| serde_json_to_py(err, "invalid config JSON"))?;
        let table = finstack_quant_portfolio::cell_returns_from_curves(
            &start_curve,
            &end_curve,
            horizon_years,
            max_duration,
            &base_label,
            &config,
        )
        .map_err(portfolio_to_py)?;
        serde_json::to_string(&table)
            .map_err(|err| serde_json_to_py(err, "serialize DurationCellTable"))
    })
}

/// Compute duration-matched credit excess returns against a base-return table.
///
/// Binds Rust `finstack_quant_portfolio::excess_returns` (Dynkin, Hyman &
/// Vankudre 1998, Appendix B): each position's ``duration`` is matched to its
/// duration cell in ``table_json`` and the position's excess return is
/// ``total_return - cell.base_return``, the credit-specific component of
/// performance isolated from the general level/shape move of the base curve.
///
/// Parameters
/// ----------
/// positions_json : str
///     JSON array of ``ExcessReturnPosition`` objects (``id``, ``weight``,
///     ``duration``, ``total_return``); weights must sum to ``1.0`` within
///     ``1e-6``.
/// table_json : str
///     JSON ``DurationCellTable``, as returned by
///     :func:`cell_returns_from_reference` or :func:`cell_returns_from_curves`.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``ExcessReturnResult`` with per-position and
///     portfolio-level total/base/excess returns.
///
/// Raises
/// ------
/// PortfolioError
///     If ``table_json`` has no cells; a cell has an empty or duplicate
///     label, non-finite bounds or base return, a negative lower bound, a
///     non-positive width, non-ascending lower bounds, or overlaps its
///     predecessor; a position has a non-finite
///     weight/duration/total_return; a position's duration falls outside
///     every cell, including a valid gap (the error names the position); or
///     the position weights do not sum to ``1.0`` within tolerance.
/// ValueError
///     If any JSON argument is malformed.
///
/// Sources
/// -------
/// See ``docs/REFERENCES.md#dynkin-hyman-vankudre-1998``.
///
/// Examples
/// --------
/// >>> import json
/// >>> from finstack_quant.portfolio import cell_returns_from_reference, excess_returns
/// >>> reference = [{"duration": 1.0, "total_return": 0.02}]
/// >>> table = cell_returns_from_reference(json.dumps(reference), "UST", '{"width": 2.0}')
/// >>> positions = [{"id": "B1", "weight": 1.0, "duration": 1.0, "total_return": 0.03}]
/// >>> result = json.loads(excess_returns(json.dumps(positions), table))
/// >>> round(result["portfolio_excess_return"], 4)
/// 0.01
#[pyfunction]
#[pyo3(text_signature = "(positions_json, table_json)")]
fn excess_returns(py: Python<'_>, positions_json: &str, table_json: &str) -> PyResult<String> {
    let positions_json = positions_json.to_owned();
    let table_json = table_json.to_owned();
    py.detach(move || {
        let positions: Vec<finstack_quant_portfolio::ExcessReturnPosition> =
            serde_json::from_str(&positions_json)
                .map_err(|err| serde_json_to_py(err, "invalid positions JSON"))?;
        let table: finstack_quant_portfolio::DurationCellTable = serde_json::from_str(&table_json)
            .map_err(|err| serde_json_to_py(err, "invalid table JSON"))?;
        let result = finstack_quant_portfolio::excess_returns(&positions, &table)
            .map_err(portfolio_to_py)?;
        serde_json::to_string(&result)
            .map_err(|err| serde_json_to_py(err, "serialize ExcessReturnResult"))
    })
}

/// Register duration-matched credit excess return functions on the portfolio submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(cell_returns_from_reference, m)?)?;
    m.add_function(wrap_pyfunction!(cell_returns_from_curves, m)?)?;
    m.add_function(wrap_pyfunction!(excess_returns, m)?)?;
    Ok(())
}

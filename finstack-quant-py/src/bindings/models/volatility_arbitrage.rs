//! Python bindings for `finstack_quant_models::volatility::arbitrage`.
//!
//! Exposes a function-based API for model-free volatility surface arbitrage
//! detection. Vol surfaces are constructed internally from flat arrays so
//! callers can work directly with numpy-friendly inputs without first
//! building a `VolSurface` wrapper.

use finstack_quant_models::volatility::arbitrage::{
    self as model_arbitrage, ArbitrageSeverity, ArbitrageType, ArbitrageViolation,
};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};

use crate::bindings::pandas_utils::serde_to_py;
use crate::errors::core_to_py;

/// Serde name of an arbitrage enum variant (the `snake_case` wire form).
fn label<T: serde::Serialize>(value: &T) -> PyResult<String> {
    finstack_quant_core::wire::serde_label(value).map_err(core_to_py)
}

/// Convert a slice of violations into a Python list of serde dicts.
///
/// Each entry is the wire form of the Rust ``ArbitrageViolation``:
/// ``violation_type``, ``severity``, ``location`` (``strike``, ``expiry``,
/// ``adjacent_expiry``), ``magnitude``, ``description``,
/// ``suggested_adjustment``.
fn violations_to_pylist<'py>(
    py: Python<'py>,
    violations: &[ArbitrageViolation],
) -> PyResult<Bound<'py, PyAny>> {
    serde_to_py(py, &violations)
}

/// Check butterfly arbitrage via Durrleman's g(k) density condition.
///
/// Parameters
/// ----------
/// strikes : list[float]
///     Monotonically increasing strike grid.
/// expiries : list[float]
///     Monotonically increasing expiry grid (years).
/// vols : list[list[float]]
///     Implied vols shaped ``[n_expiries][n_strikes]``.
/// forward_prices : list[float]
///     Forward prices. Pass either one scalar-equivalent entry to broadcast
///     across expiries, or one value per expiry.
/// tolerance : float, optional
///     Tolerance in total-variance units. Default ``1e-6``.
///
/// Returns
/// -------
/// list[dict]
///     One dict per violation with keys ``type``, ``severity``, ``strike``,
///     ``expiry``, ``adjacent_expiry``, ``magnitude``, ``value``,
///     ``message``, ``description``.
#[pyfunction]
#[pyo3(signature = (strikes, expiries, vols, forward_prices, tolerance = 1e-6))]
fn check_butterfly_grid<'py>(
    py: Python<'py>,
    strikes: Vec<f64>,
    expiries: Vec<f64>,
    vols: Vec<Vec<f64>>,
    forward_prices: Vec<f64>,
    tolerance: f64,
) -> PyResult<Bound<'py, PyAny>> {
    let violations = model_arbitrage::check_butterfly_grid(
        &strikes,
        &expiries,
        &vols,
        forward_prices,
        tolerance,
    )
    .map_err(core_to_py)?;
    violations_to_pylist(py, &violations)
}

/// Check calendar spread arbitrage (total variance monotonicity in log-moneyness).
///
/// Parameters
/// ----------
/// strikes : list[float]
///     Monotonically increasing strike grid.
/// expiries : list[float]
///     Monotonically increasing expiry grid (years).
/// vols : list[list[float]]
///     Implied vols shaped ``[n_expiries][n_strikes]``.
/// forward_prices : list[float]
///     Forward prices. Pass either one scalar-equivalent entry to broadcast
///     across expiries, or one value per expiry.
/// tolerance : float, optional
///     Tolerance in total-variance units. Default ``1e-6``.
///
/// Returns
/// -------
/// list[dict]
///     One dict per violation.
#[pyfunction]
#[pyo3(signature = (strikes, expiries, vols, forward_prices, tolerance = 1e-6))]
fn check_calendar_spread_grid<'py>(
    py: Python<'py>,
    strikes: Vec<f64>,
    expiries: Vec<f64>,
    vols: Vec<Vec<f64>>,
    forward_prices: Vec<f64>,
    tolerance: f64,
) -> PyResult<Bound<'py, PyAny>> {
    let violations = model_arbitrage::check_calendar_spread_grid(
        &strikes,
        &expiries,
        &vols,
        forward_prices,
        tolerance,
    )
    .map_err(core_to_py)?;
    violations_to_pylist(py, &violations)
}

/// Check Dupire local-vol density positivity.
///
/// Parameters
/// ----------
/// strikes : list[float]
///     Monotonically increasing strike grid.
/// expiries : list[float]
///     Monotonically increasing expiry grid (years).
/// vols : list[list[float]]
///     Implied vols shaped ``[n_expiries][n_strikes]``.
/// forward_prices : list[float]
///     Forward price per expiry (length must equal ``len(expiries)``).
///
/// Notes
/// -----
/// The underlying Rust check takes a single forward. When per-expiry forwards
/// are supplied, the check is run once per expiry with that expiry's forward
/// and only the corresponding expiry's violations are kept. This is
/// equivalent to the scalar case when all forwards are identical.
#[pyfunction]
#[pyo3(signature = (strikes, expiries, vols, forward_prices))]
fn check_local_vol_density_grid<'py>(
    py: Python<'py>,
    strikes: Vec<f64>,
    expiries: Vec<f64>,
    vols: Vec<Vec<f64>>,
    forward_prices: Vec<f64>,
) -> PyResult<Bound<'py, PyAny>> {
    let violations =
        model_arbitrage::check_local_vol_density_grid(&strikes, &expiries, &vols, forward_prices)
            .map_err(core_to_py)?;
    violations_to_pylist(py, &violations)
}

/// Run butterfly, calendar-spread, and local-vol density checks together.
///
/// Parameters
/// ----------
/// strikes : list[float]
///     Monotonically increasing strike grid.
/// expiries : list[float]
///     Monotonically increasing expiry grid (years).
/// vols : list[list[float]]
///     Implied vols shaped ``[n_expiries][n_strikes]``.
/// forward_prices : list[float]
///     Forward prices for every check. Pass either one value to broadcast or
///     one value per expiry.
/// tolerance : float, optional
///     Shared tolerance for all checks. Default ``1e-6``.
///
/// Returns
/// -------
/// dict
///     Aggregated report with keys ``total_violations``, ``passed``,
///     ``by_severity`` (dict ``severity -> count``), ``by_type``
///     (dict ``type -> count``), and ``violations`` (list[dict]).
#[pyfunction]
#[pyo3(signature = (strikes, expiries, vols, forward_prices, tolerance = 1e-6))]
fn check_surface_grid<'py>(
    py: Python<'py>,
    strikes: Vec<f64>,
    expiries: Vec<f64>,
    vols: Vec<Vec<f64>>,
    forward_prices: Vec<f64>,
    tolerance: f64,
) -> PyResult<Bound<'py, PyDict>> {
    let report =
        model_arbitrage::check_surface_grid(&strikes, &expiries, &vols, forward_prices, tolerance)
            .map_err(core_to_py)?;

    let out = PyDict::new(py);
    out.set_item("total_violations", report.violations.len())?;
    out.set_item("passed", report.passed)?;

    let by_sev = PyDict::new(py);
    for sev in [
        ArbitrageSeverity::Negligible,
        ArbitrageSeverity::Minor,
        ArbitrageSeverity::Major,
        ArbitrageSeverity::Critical,
    ] {
        by_sev.set_item(
            label(&sev)?,
            report.counts_by_severity.get(&sev).copied().unwrap_or(0),
        )?;
    }
    out.set_item("by_severity", by_sev)?;

    // Every variant `ArbitrageReport::counts_by_type` can carry, so the SVI
    // checks are reported rather than silently dropped.
    let by_type = PyDict::new(py);
    for t in [
        ArbitrageType::Butterfly,
        ArbitrageType::CalendarSpread,
        ArbitrageType::LocalVolDensity,
        ArbitrageType::SviMomentBound,
        ArbitrageType::SviButterflyCondition,
        ArbitrageType::SviCalendarSpread,
    ] {
        by_type.set_item(
            label(&t)?,
            report.counts_by_type.get(&t).copied().unwrap_or(0),
        )?;
    }
    out.set_item("by_type", by_type)?;

    out.set_item("violations", violations_to_pylist(py, &report.violations)?)?;
    out.set_item("elapsed_us", report.elapsed_us)?;
    Ok(out)
}

/// Register volatility arbitrage functions on `finstack_quant.models.volatility`.
pub fn register(_py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_function(wrap_pyfunction!(check_butterfly_grid, parent)?)?;
    parent.add_function(wrap_pyfunction!(check_calendar_spread_grid, parent)?)?;
    parent.add_function(wrap_pyfunction!(check_local_vol_density_grid, parent)?)?;
    parent.add_function(wrap_pyfunction!(check_surface_grid, parent)?)?;

    Ok(())
}

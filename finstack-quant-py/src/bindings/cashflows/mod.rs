//! Python bindings for the `finstack-quant-cashflows` crate.

pub(crate) mod accrual;
pub(crate) mod aggregation;
pub(crate) mod builder;
pub(crate) mod primitives;
mod schema;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

/// Build a cashflow schedule from a JSON spec and return canonical schedule JSON.
///
/// Parameters
/// ----------
/// spec_json : str
///     JSON-encoded `CashflowScheduleBuildSpec`.
/// market_json : str, optional
///     JSON-encoded market context for floating-rate lookups.
///
/// Returns
/// -------
/// str
///     JSON-encoded `CashFlowSchedule`.
#[pyfunction]
#[pyo3(
    signature = (spec_json, market_json = None),
    text_signature = "(spec_json, market_json=None)"
)]
fn build_cashflow_schedule_json(
    py: Python<'_>,
    spec_json: &str,
    market_json: Option<&str>,
) -> PyResult<String> {
    py.detach(|| {
        finstack_quant_cashflows::build_cashflow_schedule_json(spec_json, market_json)
            .map_err(crate::errors::core_to_py)
    })
}

/// Validate a cashflow schedule JSON string and return it canonicalized.
///
/// Parameters
/// ----------
/// schedule_json : str
///     JSON-encoded `CashFlowSchedule`.
///
/// Returns
/// -------
/// str
///     Canonicalized JSON-encoded `CashFlowSchedule`.
#[pyfunction]
#[pyo3(text_signature = "(schedule_json)")]
fn validate_cashflow_schedule_json(py: Python<'_>, schedule_json: &str) -> PyResult<String> {
    py.detach(|| {
        finstack_quant_cashflows::validate_cashflow_schedule_json(schedule_json)
            .map_err(crate::errors::core_to_py)
    })
}

/// Extract dated flows from a cashflow schedule.
///
/// Parameters
/// ----------
/// schedule_json : str
///     JSON-encoded `CashFlowSchedule`.
///
/// Returns
/// -------
/// str
///     JSON array of settlement cash entries. Non-cash PIK and default-write-down
///     rows are omitted; parse the full schedule JSON when classifications are needed.
#[pyfunction]
#[pyo3(text_signature = "(schedule_json)")]
fn dated_flows_json(py: Python<'_>, schedule_json: &str) -> PyResult<String> {
    py.detach(|| {
        finstack_quant_cashflows::dated_flows_json(schedule_json).map_err(crate::errors::core_to_py)
    })
}

/// Compute accrued interest for a schedule as of a given date.
///
/// Parameters
/// ----------
/// schedule_json : str
///     JSON-encoded `CashFlowSchedule`.
/// as_of : datetime.date | str
///     Accrual snapshot date, either a date-like object (``datetime.date``,
///     ``pandas.Timestamp``) or an ISO 8601 string.
/// config_json : str, optional
///     JSON-encoded `AccrualConfig` overriding defaults.
///
/// Returns
/// -------
/// float
///     Accrued interest in the schedule's settlement currency, returned as a
///     host-language double. The Rust engine computes from the canonical
///     schedule and then crosses the binding boundary as `f64`; for large
///     notionals, compare results with an absolute tolerance scaled to the
///     schedule notional rather than expecting decimal-string equality.
#[pyfunction]
#[pyo3(
    signature = (schedule_json, as_of, config_json = None),
    text_signature = "(schedule_json, as_of, config_json=None)"
)]
fn accrued_interest_json(
    py: Python<'_>,
    schedule_json: &str,
    as_of: &Bound<'_, PyAny>,
    config_json: Option<&str>,
) -> PyResult<f64> {
    let as_of = crate::bindings::date_utils::extract_date_iso(as_of)?;
    py.detach(|| {
        finstack_quant_cashflows::accrued_interest_json(schedule_json, &as_of, config_json)
            .map_err(crate::errors::core_to_py)
    })
}

/// Register the `finstack_quant.cashflows` Python namespace.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "cashflows")?;
    m.setattr(
        "__doc__",
        "Cashflow schedule JSON construction and validation.",
    )?;
    m.setattr("__package__", "finstack_quant.cashflows")?;

    primitives::register(py, &m)?;
    builder::register(py, &m)?;
    accrual::register(py, &m)?;
    aggregation::register(py, &m)?;
    schema::register(py, &m)?;

    m.add_function(wrap_pyfunction!(accrued_interest_json, &m)?)?;
    m.add_function(wrap_pyfunction!(build_cashflow_schedule_json, &m)?)?;
    m.add_function(wrap_pyfunction!(dated_flows_json, &m)?)?;
    m.add_function(wrap_pyfunction!(validate_cashflow_schedule_json, &m)?)?;

    for name in [
        "accrued_interest_json",
        "build_cashflow_schedule_json",
        "dated_flows_json",
        "validate_cashflow_schedule_json",
    ] {
        m.getattr(name)?
            .setattr("__module__", "finstack_quant.cashflows")?;
    }

    let all = PyList::new(
        py,
        [
            "accrual",
            "accrued_interest_json",
            "aggregation",
            "build_cashflow_schedule_json",
            "builder",
            "dated_flows_json",
            "primitives",
            "schema",
            "validate_cashflow_schedule_json",
        ],
    )?;
    m.setattr("__all__", all)?;
    crate::bindings::module_utils::register_submodule(
        py,
        parent,
        &m,
        "cashflows",
        crate::bindings::module_utils::ROOT_PACKAGE,
        crate::bindings::module_utils::ParentNameSource::Name,
    )?;

    Ok(())
}

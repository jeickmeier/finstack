//! Python bindings for IMM, CDS-roll, and listed-option expiry dates.

use finstack_quant_core::dates::{
    imm_option_expiry, is_cds_date, is_imm_date, next_cds_date, next_equity_option_expiry,
    next_imm, next_imm_option_expiry, next_semiannual_cds_maturity, prev_cds_date,
    prev_cds_semiannual_roll, third_friday, third_wednesday,
};
use pyo3::prelude::*;
use pyo3::types::PyModule;

use super::utils::{date_to_py, py_to_date};

/// Public names registered by this module.
pub const EXPORTS: &[&str] = &[
    "third_wednesday",
    "third_friday",
    "next_imm",
    "is_imm_date",
    "is_cds_date",
    "next_cds_date",
    "prev_cds_date",
    "prev_cds_semiannual_roll",
    "next_semiannual_cds_maturity",
    "imm_option_expiry",
    "next_imm_option_expiry",
    "next_equity_option_expiry",
];

fn month(value: u8) -> PyResult<time::Month> {
    time::Month::try_from(value)
        .map_err(|_| crate::errors::value_error(format!("invalid month: {value}")))
}

/// Third Wednesday of the given month — the IMM date convention.
#[pyfunction(name = "third_wednesday")]
#[pyo3(text_signature = "(month, year)")]
fn py_third_wednesday<'py>(
    py: Python<'py>,
    month_number: u8,
    year: i32,
) -> PyResult<Bound<'py, PyAny>> {
    date_to_py(py, third_wednesday(month(month_number)?, year))
}

/// Third Friday of the given month — the listed-equity-option expiry
/// convention.
#[pyfunction(name = "third_friday")]
#[pyo3(text_signature = "(month, year)")]
fn py_third_friday<'py>(
    py: Python<'py>,
    month_number: u8,
    year: i32,
) -> PyResult<Bound<'py, PyAny>> {
    date_to_py(py, third_friday(month(month_number)?, year))
}

/// Next quarterly IMM date strictly after `date`.
#[pyfunction(name = "next_imm")]
#[pyo3(text_signature = "(date)")]
fn py_next_imm<'py>(py: Python<'py>, date: &Bound<'_, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    date_to_py(py, next_imm(py_to_date(date)?))
}

/// True when `date` is a quarterly IMM date.
#[pyfunction(name = "is_imm_date")]
#[pyo3(text_signature = "(date)")]
fn py_is_imm_date(date: &Bound<'_, PyAny>) -> PyResult<bool> {
    Ok(is_imm_date(py_to_date(date)?))
}

/// True when `date` is a standard CDS roll date.
#[pyfunction(name = "is_cds_date")]
#[pyo3(text_signature = "(date)")]
fn py_is_cds_date(date: &Bound<'_, PyAny>) -> PyResult<bool> {
    Ok(is_cds_date(py_to_date(date)?))
}

/// Next standard CDS roll date on or after `date`.
#[pyfunction(name = "next_cds_date")]
#[pyo3(text_signature = "(date)")]
fn py_next_cds_date<'py>(py: Python<'py>, date: &Bound<'_, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    date_to_py(py, next_cds_date(py_to_date(date)?))
}

/// Most recent standard CDS roll date on or before `date`.
#[pyfunction(name = "prev_cds_date")]
#[pyo3(text_signature = "(date)")]
fn py_prev_cds_date<'py>(py: Python<'py>, date: &Bound<'_, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    date_to_py(py, prev_cds_date(py_to_date(date)?))
}

/// Most recent semi-annual (March / September) CDS roll on or before `date`.
#[pyfunction(name = "prev_cds_semiannual_roll")]
#[pyo3(text_signature = "(date)")]
fn py_prev_cds_semiannual_roll<'py>(
    py: Python<'py>,
    date: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    date_to_py(py, prev_cds_semiannual_roll(py_to_date(date)?))
}

/// Next semi-annual CDS maturity date after `date`.
#[pyfunction(name = "next_semiannual_cds_maturity")]
#[pyo3(text_signature = "(date)")]
fn py_next_semiannual_cds_maturity<'py>(
    py: Python<'py>,
    date: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    date_to_py(py, next_semiannual_cds_maturity(py_to_date(date)?))
}

/// Expiry of the option on the IMM future for the given month.
#[pyfunction(name = "imm_option_expiry")]
#[pyo3(text_signature = "(month, year)")]
fn py_imm_option_expiry<'py>(
    py: Python<'py>,
    month_number: u8,
    year: i32,
) -> PyResult<Bound<'py, PyAny>> {
    date_to_py(py, imm_option_expiry(month(month_number)?, year))
}

/// Next quarterly IMM option expiry strictly after `date`.
#[pyfunction(name = "next_imm_option_expiry")]
#[pyo3(text_signature = "(date)")]
fn py_next_imm_option_expiry<'py>(
    py: Python<'py>,
    date: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    date_to_py(py, next_imm_option_expiry(py_to_date(date)?))
}

/// Next monthly listed-equity-option expiry strictly after `date`.
#[pyfunction(name = "next_equity_option_expiry")]
#[pyo3(text_signature = "(date)")]
fn py_next_equity_option_expiry<'py>(
    py: Python<'py>,
    date: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    date_to_py(py, next_equity_option_expiry(py_to_date(date)?))
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(py_third_wednesday, module)?)?;
    module.add_function(wrap_pyfunction!(py_third_friday, module)?)?;
    module.add_function(wrap_pyfunction!(py_next_imm, module)?)?;
    module.add_function(wrap_pyfunction!(py_is_imm_date, module)?)?;
    module.add_function(wrap_pyfunction!(py_is_cds_date, module)?)?;
    module.add_function(wrap_pyfunction!(py_next_cds_date, module)?)?;
    module.add_function(wrap_pyfunction!(py_prev_cds_date, module)?)?;
    module.add_function(wrap_pyfunction!(py_prev_cds_semiannual_roll, module)?)?;
    module.add_function(wrap_pyfunction!(py_next_semiannual_cds_maturity, module)?)?;
    module.add_function(wrap_pyfunction!(py_imm_option_expiry, module)?)?;
    module.add_function(wrap_pyfunction!(py_next_imm_option_expiry, module)?)?;
    module.add_function(wrap_pyfunction!(py_next_equity_option_expiry, module)?)?;
    Ok(())
}

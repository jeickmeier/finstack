//! Python bindings for published and estimated SIFMA settlement dates.

use finstack_quant_core::dates::{
    estimated_sifma_settlement_date_for_class, next_sifma_settlement, sifma_settlement_date,
    sifma_settlement_date_for_class, SifmaSettlementClass,
};
use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::bindings::date_utils::{date_to_py, py_to_date};

/// Public names registered by this module.
pub const EXPORTS: &[&str] = &[
    "SifmaSettlementClass",
    "sifma_settlement_date",
    "sifma_settlement_date_for_class",
    "estimated_sifma_settlement_date_for_class",
    "next_sifma_settlement",
];

#[pyclass(
    name = "SifmaSettlementClass",
    module = "finstack_quant.core.dates",
    frozen,
    eq,
    hash,
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PySifmaSettlementClass {
    inner: SifmaSettlementClass,
}

#[pymethods]
impl PySifmaSettlementClass {
    #[classattr]
    const A: Self = Self {
        inner: SifmaSettlementClass::A,
    };
    #[classattr]
    const B: Self = Self {
        inner: SifmaSettlementClass::B,
    };
    #[classattr]
    const C: Self = Self {
        inner: SifmaSettlementClass::C,
    };
    #[classattr]
    const D: Self = Self {
        inner: SifmaSettlementClass::D,
    };

    #[classmethod]
    #[pyo3(text_signature = "(cls, agency, term_years)")]
    fn from_agency_term(
        _cls: &Bound<'_, pyo3::types::PyType>,
        agency: &str,
        term_years: u32,
    ) -> Self {
        Self {
            inner: SifmaSettlementClass::from_agency_term(agency, term_years),
        }
    }

    fn __repr__(&self) -> String {
        format!("SifmaSettlementClass.{:?}", self.inner)
    }
}

/// Published SIFMA settlement date for a delivery month.
///
/// Parameters
/// ----------
/// month : int
///     Delivery month, 1-12.
/// year : int
///     Delivery year.
///
/// Returns
/// -------
/// datetime.date | None
///     The published date, or ``None`` when SIFMA has not published one for
///     that month. Use :func:`estimated_sifma_settlement_date_for_class` when
///     an estimate is acceptable.
///
/// Raises
/// ------
/// ValueError
///     If ``month`` is outside 1-12.
#[pyfunction(name = "sifma_settlement_date")]
#[pyo3(text_signature = "(month, year)")]
fn py_sifma_settlement_date<'py>(
    py: Python<'py>,
    month_number: u8,
    year: i32,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    sifma_settlement_date(
        crate::bindings::date_utils::month_from_u8(month_number)?,
        year,
    )
    .map(|date| date_to_py(py, date))
    .transpose()
}

/// Published SIFMA settlement date for one settlement class.
///
/// Parameters
/// ----------
/// month : int
///     Delivery month, 1-12.
/// year : int
///     Delivery year.
/// settlement_class : SifmaSettlementClass
///     Class whose calendar applies (classes settle on different days of the
///     same delivery month).
///
/// Returns
/// -------
/// datetime.date | None
///     The published date, or ``None`` when SIFMA has not published one for
///     that month and class.
///
/// Raises
/// ------
/// ValueError
///     If ``month`` is outside 1-12.
#[pyfunction(name = "sifma_settlement_date_for_class")]
#[pyo3(text_signature = "(month, year, settlement_class)")]
fn py_sifma_settlement_date_for_class<'py>(
    py: Python<'py>,
    month_number: u8,
    year: i32,
    settlement_class: PyRef<'_, PySifmaSettlementClass>,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    sifma_settlement_date_for_class(
        crate::bindings::date_utils::month_from_u8(month_number)?,
        year,
        settlement_class.inner,
    )
    .map(|date| date_to_py(py, date))
    .transpose()
}

/// Estimated SIFMA settlement date for one settlement class.
///
/// Unlike :func:`sifma_settlement_date_for_class` this always returns a date:
/// months SIFMA has not published are projected from the published rule, so
/// forward schedules can be built beyond the calendar.
///
/// Parameters
/// ----------
/// month : int
///     Delivery month, 1-12.
/// year : int
///     Delivery year.
/// settlement_class : SifmaSettlementClass
///     Class whose calendar applies.
///
/// Returns
/// -------
/// datetime.date
///     The published date when one exists, otherwise the projected date.
///
/// Raises
/// ------
/// ValueError
///     If ``month`` is outside 1-12.
#[pyfunction(name = "estimated_sifma_settlement_date_for_class")]
#[pyo3(text_signature = "(month, year, settlement_class)")]
fn py_estimated_sifma_settlement_date_for_class<'py>(
    py: Python<'py>,
    month_number: u8,
    year: i32,
    settlement_class: PyRef<'_, PySifmaSettlementClass>,
) -> PyResult<Bound<'py, PyAny>> {
    date_to_py(
        py,
        estimated_sifma_settlement_date_for_class(
            crate::bindings::date_utils::month_from_u8(month_number)?,
            year,
            settlement_class.inner,
        ),
    )
}

/// Next published SIFMA settlement date on or after `date`.
///
/// # Arguments
///
/// * `date` - Calendar date from which to search (`datetime.date` or date-like).
///
/// # Errors
///
/// Returns `TypeError` if `date` is not date-like, or `ValueError` if those
/// attributes do not form a valid calendar date.
#[pyfunction(name = "next_sifma_settlement")]
#[pyo3(text_signature = "(date)")]
fn py_next_sifma_settlement<'py>(
    py: Python<'py>,
    date: &Bound<'_, PyAny>,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    next_sifma_settlement(py_to_date(date)?)
        .map(|date| date_to_py(py, date))
        .transpose()
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PySifmaSettlementClass>()?;
    module.add_function(wrap_pyfunction!(py_sifma_settlement_date, module)?)?;
    module.add_function(wrap_pyfunction!(
        py_sifma_settlement_date_for_class,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        py_estimated_sifma_settlement_date_for_class,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(py_next_sifma_settlement, module)?)?;
    Ok(())
}

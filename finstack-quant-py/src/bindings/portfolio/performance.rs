//! Python bindings for portfolio performance measurement.
//!
//! The functions accept JSON inputs matching the Rust `serde` shapes and
//! delegate all calculations to `finstack_quant_portfolio::performance`.
//! `twrr_linked` returns a typed wrapper; `twrr_linked_json` keeps the exact
//! JSON wire string.

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule};

use crate::bindings::pandas_utils::serde_object_to_single_row_dataframe_with_schema;
use crate::errors::{core_to_py, display_to_py, serde_json_to_py};

/// Result of geometrically linking TWRR sub-period returns.
///
/// Returned by :func:`twrr_linked`.
#[pyclass(
    name = "LinkedReturn",
    module = "finstack_quant.portfolio",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyLinkedReturn {
    pub(crate) inner: finstack_quant_portfolio::LinkedReturn,
}

#[pymethods]
impl PyLinkedReturn {
    /// Cumulative return over the full horizon.
    #[getter]
    fn cumulative(&self) -> f64 {
        self.inner.cumulative
    }

    /// Annualised return; mirrors ``cumulative`` for horizons below one year.
    #[getter]
    fn annualised(&self) -> f64 {
        self.inner.annualised
    }

    /// Number of sub-periods linked.
    #[getter]
    fn num_periods(&self) -> usize {
        self.inner.num_periods
    }

    /// Single-row :class:`pandas.DataFrame` view of the linked return.
    ///
    /// Columns: ``cumulative``, ``annualised``, ``num_periods``.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_object_to_single_row_dataframe_with_schema(
            py,
            &self.inner,
            &["cumulative", "annualised", "num_periods"],
        )
    }

    /// Serialize to a compact JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Deserialize from a JSON string.
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: finstack_quant_portfolio::LinkedReturn =
            serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    fn __repr__(&self) -> String {
        format!(
            "LinkedReturn(cumulative={}, annualised={}, num_periods={})",
            self.inner.cumulative, self.inner.annualised, self.inner.num_periods,
        )
    }

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }
}

/// Compute a Modified-Dietz TWRR sub-period return.
///
/// Raises ``ValueError`` when the return is undefined (for example, a
/// non-positive adjusted denominator or an out-of-range cashflow weight).
#[pyfunction]
#[pyo3(text_signature = "(period_json)")]
fn twrr_modified_dietz(py: Python<'_>, period_json: &str) -> PyResult<f64> {
    let period_json = period_json.to_owned();
    py.detach(move || {
        let period: finstack_quant_portfolio::TwrrPeriod = serde_json::from_str(&period_json)
            .map_err(|err| serde_json_to_py(err, "invalid TWRR period JSON"))?;
        finstack_quant_portfolio::twrr_modified_dietz(&period).map_err(core_to_py)
    })
}

/// Parse the returns and run the canonical geometric linking.
fn run_twrr_linked(
    py: Python<'_>,
    returns_json: &str,
    horizon_years: f64,
) -> PyResult<finstack_quant_portfolio::LinkedReturn> {
    let returns_json = returns_json.to_owned();
    py.detach(move || {
        let returns: Vec<f64> = serde_json::from_str(&returns_json)
            .map_err(|err| serde_json_to_py(err, "invalid TWRR returns JSON"))?;
        finstack_quant_portfolio::twrr_linked(&returns, horizon_years).map_err(core_to_py)
    })
}

/// Geometrically link TWRR sub-period returns.
///
/// Parameters
/// ----------
/// returns_json : str
///     JSON array of sub-period returns as decimal fractions.
/// horizon_years : float
///     Full elapsed horizon in 365-day calendar years; values below one skip
///     annualization (``annualised`` then mirrors ``cumulative``).
///
/// Returns
/// -------
/// LinkedReturn
///     Typed result with ``cumulative``, ``annualised`` and ``num_periods``.
///     Use :func:`twrr_linked_json` for the raw wire string.
///
/// Raises
/// ------
/// ValueError
///     When any sub-period return is non-finite or the compounded growth
///     factor is non-positive.
#[pyfunction]
#[pyo3(text_signature = "(returns_json, horizon_years)")]
fn twrr_linked(py: Python<'_>, returns_json: &str, horizon_years: f64) -> PyResult<PyLinkedReturn> {
    Ok(PyLinkedReturn {
        inner: run_twrr_linked(py, returns_json, horizon_years)?,
    })
}

/// Geometrically link TWRR sub-period returns and return wire JSON.
///
/// Wire twin of :func:`twrr_linked`; same inputs, JSON-string output.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``LinkedReturn``.
#[pyfunction]
#[pyo3(text_signature = "(returns_json, horizon_years)")]
fn twrr_linked_json(py: Python<'_>, returns_json: &str, horizon_years: f64) -> PyResult<String> {
    let result = run_twrr_linked(py, returns_json, horizon_years)?;
    serde_json::to_string(&result).map_err(|err| serde_json_to_py(err, "serialize linked return"))
}

/// Compute money-weighted return via XIRR from dated cashflow JSON.
#[pyfunction]
#[pyo3(text_signature = "(cashflows_json)")]
fn mwr_xirr(py: Python<'_>, cashflows_json: &str) -> PyResult<f64> {
    let cashflows_json = cashflows_json.to_owned();
    py.detach(move || {
        let cashflows: Vec<finstack_quant_portfolio::DatedCashflow> =
            serde_json::from_str(&cashflows_json)
                .map_err(|err| serde_json_to_py(err, "invalid MWR cashflows JSON"))?;
        finstack_quant_portfolio::mwr_xirr_from_cashflows(&cashflows).map_err(core_to_py)
    })
}

/// Register performance measurement functions on the portfolio submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyLinkedReturn>()?;
    m.add_function(wrap_pyfunction!(twrr_modified_dietz, m)?)?;
    m.add_function(wrap_pyfunction!(twrr_linked, m)?)?;
    m.add_function(wrap_pyfunction!(twrr_linked_json, m)?)?;
    m.add_function(wrap_pyfunction!(mwr_xirr, m)?)?;
    Ok(())
}

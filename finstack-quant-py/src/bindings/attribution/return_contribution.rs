//! Typed wrapper for return-contribution attribution results.

use crate::bindings::pandas_utils::{
    labeled_values_to_series, serde_rows_to_dataframe, serde_to_py,
};
use crate::errors::display_to_py;
use pyo3::prelude::*;
use pyo3::types::PyAny;

/// Return-contribution attribution result.
///
/// Returned by :func:`attribute_return_contribution`. Decomposes a portfolio
/// return into per-instrument, per-group, and per-factor contributions, with
/// an optional Brinson-Fachler benchmark-relative block.
#[pyclass(
    name = "ReturnContributionResult",
    module = "finstack_quant.attribution",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyReturnContributionResult {
    pub(crate) inner: finstack_quant_attribution::ReturnContributionResult,
}

#[pymethods]
impl PyReturnContributionResult {
    /// Total portfolio return, equal to the summed instrument contributions.
    #[getter]
    fn portfolio_return(&self) -> f64 {
        self.inner.portfolio_return
    }

    /// Per-instrument contribution rows.
    #[getter]
    fn instrument_contribution<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_to_py(py, &self.inner.instrument_contribution)
    }

    /// Contributions keyed by group dimension.
    #[getter]
    fn group_contribution<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_to_py(py, &self.inner.group_contribution)
    }

    /// Factor contribution rows.
    #[getter]
    fn factor_contribution<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_to_py(py, &self.inner.factor_contribution)
    }

    /// Idiosyncratic residual when factor rows were supplied:
    /// ``portfolio_return - sum(factor contributions)``. ``None`` when the
    /// spec carried no factors.
    #[getter]
    fn specific_return(&self) -> Option<f64> {
        self.inner.specific_return
    }

    /// Brinson-Fachler benchmark-relative block, when benchmark inputs were
    /// supplied.
    #[getter]
    fn benchmark_relative<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        self.inner
            .benchmark_relative
            .as_ref()
            .map(|value| serde_to_py(py, value))
            .transpose()
    }

    /// Diagnostic warnings (for example leveraged weights from a near-flat
    /// net-market-value book).
    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.inner.warnings.clone()
    }

    /// Per-instrument contributions as a :class:`pandas.DataFrame`.
    ///
    /// One row per instrument with ``id``, ``weight``, ``return``,
    /// ``contribution``, and ``active_contribution`` columns.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_rows_to_dataframe(py, &self.inner.instrument_contribution)
    }

    /// Per-instrument contributions as a :class:`pandas.Series` indexed by
    /// instrument id.
    fn to_series<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let labels: Vec<String> = self
            .inner
            .instrument_contribution
            .iter()
            .map(|row| row.id.clone())
            .collect();
        let values: Vec<f64> = self
            .inner
            .instrument_contribution
            .iter()
            .map(|row| row.contribution)
            .collect();
        labeled_values_to_series(py, &labels, values, "contribution")
    }

    /// Serialize to a compact JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Deserialize from a JSON string.
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: finstack_quant_attribution::ReturnContributionResult =
            serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Identify this value in notebooks and logs.
    ///
    /// Rendered from the wire representation, so the fields shown are the
    /// fields `to_json()` names. Collections are summarised by length; use
    /// `to_json()` or a DataFrame exit when the contents matter.
    fn __repr__(&self) -> String {
        crate::bindings::repr_support::repr_from_serde("ReturnContributionResult", &self.inner)
    }
}

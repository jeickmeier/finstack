//! Typed wrapper for scenario profit-and-loss results.

use crate::bindings::pandas_utils::ColumnSchema;
use crate::bindings::pandas_utils::{
    labeled_values_to_series, serde_rows_to_dataframe_with_schema,
};
use crate::errors::display_to_py;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use serde::Serialize;

/// Column schema for [`PyScenarioPnl::to_dataframe`].
///
/// Pinned so an empty ladder keeps the same dtypes as a populated one.
const PNL_ROW_COLUMNS: &[ColumnSchema<'static>] = &[("position_id", "str"), ("pnl", "f64")];

#[derive(Serialize)]
struct PnlRow<'a> {
    position_id: &'a str,
    pnl: f64,
}

/// Scenario-attributable profit and loss, in the portfolio base currency.
///
/// Returned by :func:`scenario_pnl` alongside the scenario
/// :class:`ApplicationReport`.
#[pyclass(
    name = "ScenarioPnl",
    module = "finstack_quant.portfolio",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyScenarioPnl {
    pub(crate) inner: finstack_quant_portfolio::scenarios::ScenarioPnl,
}

impl PyScenarioPnl {
    fn rows(&self) -> Vec<PnlRow<'_>> {
        self.inner
            .by_position
            .iter()
            .map(|(id, money)| PnlRow {
                position_id: id.as_str(),
                pnl: money.amount(),
            })
            .collect()
    }
}

#[pymethods]
impl PyScenarioPnl {
    /// Total scenario P&L in the portfolio base currency.
    #[getter]
    fn total(&self) -> f64 {
        self.inner.total.amount()
    }

    /// Base currency the P&L is denominated in.
    #[getter]
    fn currency(&self) -> String {
        self.inner.total.currency().to_string()
    }

    /// Per-position scenario P&L, keyed by position id.
    #[getter]
    fn by_position(&self) -> std::collections::BTreeMap<String, f64> {
        self.inner
            .by_position
            .iter()
            .map(|(id, money)| (id.to_string(), money.amount()))
            .collect()
    }

    /// Per-position P&L ladder as a :class:`pandas.DataFrame`.
    ///
    /// Columns: ``position_id``, ``pnl``.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_rows_to_dataframe_with_schema(py, &self.rows(), PNL_ROW_COLUMNS)
    }

    /// Per-position P&L as a :class:`pandas.Series` indexed by position id.
    fn to_series<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let labels: Vec<String> = self
            .inner
            .by_position
            .keys()
            .map(ToString::to_string)
            .collect();
        let values: Vec<f64> = self
            .inner
            .by_position
            .values()
            .map(finstack_quant_core::money::Money::amount)
            .collect();
        labeled_values_to_series(py, &labels, values, "pnl")
    }

    /// Serialize to a compact JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Deserialize from a JSON string.
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: finstack_quant_portfolio::scenarios::ScenarioPnl =
            serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }
}

use pyo3::prelude::*;
use pyo3::types::PyDict;

use finstack_quant_portfolio::factor_model::{
    self as fm, CreditVolReport, LevelVolContribution, PositionVolContribution,
};

use crate::bindings::factor_model::credit::PyCreditFactorModel;

use super::super::json_bridge::serialize_json;
use super::contributions::PyRiskDecomposition;

/// Aggregated risk contribution for a single hierarchy level.
#[pyclass(
    name = "LevelVolContribution",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyLevelVolContribution {
    pub(crate) inner: LevelVolContribution,
}

impl PyLevelVolContribution {
    fn from_inner(inner: LevelVolContribution) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyLevelVolContribution {
    #[getter]
    fn level_name(&self) -> String {
        self.inner.level_name.clone()
    }

    #[getter]
    fn total(&self) -> f64 {
        self.inner.total
    }

    /// Per-bucket contributions keyed by the bucket path.
    ///
    /// Keys are returned in deterministic (sorted) bucket-path order.
    #[getter]
    fn by_bucket<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        for (k, v) in &self.inner.by_bucket {
            d.set_item(k, v)?;
        }
        Ok(d)
    }

    fn __repr__(&self) -> String {
        format!(
            "LevelVolContribution(level_name={:?}, total={}, buckets={})",
            self.inner.level_name,
            self.inner.total,
            self.inner.by_bucket.len(),
        )
    }
}

/// Per-position vol breakdown under :class:`CreditVolReport`.
#[pyclass(
    name = "PositionVolContribution",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyPositionVolContribution {
    pub(crate) inner: PositionVolContribution,
}

impl PyPositionVolContribution {
    fn from_inner(inner: PositionVolContribution) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPositionVolContribution {
    #[getter]
    fn position_id(&self) -> String {
        self.inner.position_id.as_str().to_owned()
    }

    #[getter]
    fn factor_total(&self) -> f64 {
        self.inner.factor_total
    }

    #[getter]
    fn idiosyncratic(&self) -> f64 {
        self.inner.idiosyncratic
    }

    #[getter]
    fn total(&self) -> f64 {
        self.inner.total
    }

    fn __repr__(&self) -> String {
        format!(
            "PositionVolContribution(position_id={:?}, factor_total={}, idiosyncratic={}, total={})",
            self.inner.position_id.as_str(),
            self.inner.factor_total,
            self.inner.idiosyncratic,
            self.inner.total,
        )
    }
}

/// Aggregated vol report grouped by hierarchy level.
#[pyclass(
    name = "CreditVolReport",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyCreditVolReport {
    pub(crate) inner: CreditVolReport,
}

impl PyCreditVolReport {
    fn from_inner(inner: CreditVolReport) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCreditVolReport {
    #[getter]
    fn total(&self) -> f64 {
        self.inner.total
    }

    /// Risk measure serialized as a JSON-compatible string (e.g. ``"variance"``).
    #[getter]
    fn measure_json(&self) -> PyResult<String> {
        serialize_json(&self.inner.measure)
    }

    #[getter]
    fn generic(&self) -> f64 {
        self.inner.generic
    }

    #[getter]
    fn idiosyncratic_total(&self) -> f64 {
        self.inner.idiosyncratic_total
    }

    #[getter]
    fn by_level(&self) -> Vec<PyLevelVolContribution> {
        self.inner
            .by_level
            .iter()
            .cloned()
            .map(PyLevelVolContribution::from_inner)
            .collect()
    }

    /// Optional per-position breakdown; ``None`` when not requested.
    #[getter]
    fn by_position(&self) -> Option<Vec<PyPositionVolContribution>> {
        self.inner.by_position_optional.as_ref().map(|rows| {
            rows.iter()
                .cloned()
                .map(PyPositionVolContribution::from_inner)
                .collect()
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "CreditVolReport(total={}, generic={}, idiosyncratic_total={}, by_level={})",
            self.inner.total,
            self.inner.generic,
            self.inner.idiosyncratic_total,
            self.inner.by_level.len(),
        )
    }
}

/// Build a credit volatility report from a risk decomposition and credit model.
#[pyfunction]
#[pyo3(signature = (decomposition, model, by_position = false))]
pub(super) fn build_credit_vol_report(
    py: Python<'_>,
    decomposition: &PyRiskDecomposition,
    model: &PyCreditFactorModel,
    by_position: bool,
) -> PyResult<PyCreditVolReport> {
    let decomposition = decomposition.inner.clone();
    let model = model.inner.clone();
    let report =
        py.detach(move || fm::build_credit_vol_report(&decomposition, &model, by_position));
    Ok(PyCreditVolReport::from_inner(report))
}

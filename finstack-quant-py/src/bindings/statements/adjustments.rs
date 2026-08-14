//! Python wrappers for EBITDA normalization and adjustments.

use super::evaluator::PyStatementResult;
use crate::errors::display_to_py;
use pyo3::prelude::*;

/// Configuration for normalizing a financial metric.
#[pyclass(
    name = "NormalizationConfig",
    module = "finstack_quant.statements",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyNormalizationConfig {
    pub(super) inner: finstack_quant_statements::adjustments::types::NormalizationConfig,
}

#[pymethods]
impl PyNormalizationConfig {
    /// Create a normalization configuration for a target node.
    ///
    /// Starts with no adjustments; add-backs and deductions are supplied via
    /// the JSON form (:meth:`from_json`).
    ///
    /// Parameters
    /// ----------
    /// target_node : str
    ///     Node identifier of the metric to normalize (e.g. ``"ebitda"``).
    #[new]
    #[pyo3(text_signature = "(target_node)")]
    fn new(target_node: &str) -> Self {
        Self {
            inner: finstack_quant_statements::adjustments::types::NormalizationConfig::new(
                target_node,
            ),
        }
    }

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Deserialize a normalization configuration from JSON.
    ///
    /// This is the way to supply adjustments: each carries an id, name,
    /// optional category, a value rule (a fixed per-period amount or a
    /// percentage of a reference node, where the percentage is a decimal
    /// fraction — 0.05 for 5%), and an optional cap. Unknown fields are
    /// rejected.
    #[staticmethod]
    #[pyo3(text_signature = "(json, /)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: finstack_quant_statements::adjustments::types::NormalizationConfig =
            serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize this configuration to compact JSON.
    #[pyo3(text_signature = "($self)")]
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Node identifier of the metric being normalized (e.g. ``"ebitda"``).
    #[getter]
    fn target_node(&self) -> &str {
        &self.inner.target_node
    }

    /// Number of add-back / deduction adjustments configured.
    #[getter]
    fn adjustment_count(&self) -> usize {
        self.inner.adjustments.len()
    }

    /// Return the debug representation with the target node and adjustment
    /// count.
    fn __repr__(&self) -> String {
        format!(
            "NormalizationConfig(target={:?}, adjustments={})",
            self.inner.target_node,
            self.inner.adjustments.len()
        )
    }
}

// normalize() function

/// Run normalization on statement results.
///
/// Parameters
/// ----------
/// results : StatementResult
///     Evaluated statement results.
/// config : NormalizationConfig
///     Normalization configuration (target node + adjustments).
///
/// Returns
/// -------
/// str
///     JSON-serialized list of ``NormalizationResult`` objects.
#[pyfunction]
#[pyo3(text_signature = "(results, config, /)")]
fn normalize(results: &PyStatementResult, config: &PyNormalizationConfig) -> PyResult<String> {
    let norm_results =
        finstack_quant_statements::adjustments::engine::NormalizationEngine::normalize(
            &results.inner,
            &config.inner,
        )
        .map_err(display_to_py)?;

    serde_json::to_string(&norm_results).map_err(display_to_py)
}

/// Register adjustment classes and functions.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyNormalizationConfig>()?;
    m.add_function(pyo3::wrap_pyfunction!(normalize, m)?)?;
    Ok(())
}

//! Python wrappers for EBITDA normalization and adjustments.

use super::evaluator::PyStatementResult;
use crate::errors::display_to_py;
use pyo3::prelude::*;

/// One adjustment applied while normalizing a statement metric.
#[pyclass(
    name = "AppliedAdjustment",
    module = "finstack_quant.statements",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyAppliedAdjustment {
    inner: finstack_quant_statements::adjustments::types::AppliedAdjustment,
}

#[pymethods]
impl PyAppliedAdjustment {
    /// Stable adjustment identifier.
    #[getter]
    fn adjustment_id(&self) -> &str {
        &self.inner.adjustment_id
    }

    /// Human-readable adjustment name.
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    /// Calculated amount before applying a cap.
    #[getter]
    fn raw_amount(&self) -> f64 {
        self.inner.raw_amount
    }

    /// Amount included after applying any cap.
    #[getter]
    fn capped_amount(&self) -> f64 {
        self.inner.capped_amount
    }

    /// Whether a cap changed the calculated amount.
    #[getter]
    fn is_capped(&self) -> bool {
        self.inner.is_capped
    }

    /// Return a concise diagnostic representation.
    fn __repr__(&self) -> String {
        format!(
            "AppliedAdjustment(id={:?}, raw_amount={}, capped_amount={}, is_capped={})",
            self.inner.adjustment_id,
            self.inner.raw_amount,
            self.inner.capped_amount,
            self.inner.is_capped
        )
    }
}

/// Normalized metric result for one reporting period.
#[pyclass(
    name = "NormalizationResult",
    module = "finstack_quant.statements",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyNormalizationResult {
    inner: finstack_quant_statements::adjustments::types::NormalizationResult,
}

#[pymethods]
impl PyNormalizationResult {
    /// Deserialize a normalization result from compact JSON.
    #[staticmethod]
    #[pyo3(text_signature = "(json, /)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize this result to compact JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Support pickle through the canonical JSON representation.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Reporting period identifier such as ``"2025Q1"``.
    #[getter]
    fn period(&self) -> String {
        self.inner.period.to_string()
    }

    /// Reported value before normalization adjustments.
    #[getter]
    fn base_value(&self) -> f64 {
        self.inner.base_value
    }

    /// Applied adjustments in declaration order.
    #[getter]
    fn adjustments(&self) -> Vec<PyAppliedAdjustment> {
        self.inner
            .adjustments
            .iter()
            .cloned()
            .map(|inner| PyAppliedAdjustment { inner })
            .collect()
    }

    /// Value after all capped adjustments.
    #[getter]
    fn final_value(&self) -> f64 {
        self.inner.final_value
    }

    /// Return a concise diagnostic representation.
    fn __repr__(&self) -> String {
        format!(
            "NormalizationResult(period={:?}, base_value={}, final_value={}, adjustments={})",
            self.inner.period.to_string(),
            self.inner.base_value,
            self.inner.final_value,
            self.inner.adjustments.len()
        )
    }
}

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
/// list[NormalizationResult]
///     Typed period results in chronological order.
#[pyfunction]
#[pyo3(text_signature = "(results, config, /)")]
fn normalize(
    results: &PyStatementResult,
    config: &PyNormalizationConfig,
) -> PyResult<Vec<PyNormalizationResult>> {
    let norm_results =
        finstack_quant_statements::adjustments::engine::NormalizationEngine::normalize(
            &results.inner,
            &config.inner,
        )
        .map_err(display_to_py)?;

    Ok(norm_results
        .into_iter()
        .map(|inner| PyNormalizationResult { inner })
        .collect())
}

/// Run normalization and return compact wire JSON.
#[pyfunction]
#[pyo3(text_signature = "(results, config, /)")]
fn normalize_json(results: &PyStatementResult, config: &PyNormalizationConfig) -> PyResult<String> {
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
    m.add_class::<PyAppliedAdjustment>()?;
    m.add_class::<PyNormalizationConfig>()?;
    m.add_class::<PyNormalizationResult>()?;
    m.add_function(pyo3::wrap_pyfunction!(normalize, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(normalize_json, m)?)?;
    Ok(())
}

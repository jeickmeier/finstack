use pyo3::prelude::*;
use pyo3::types::PyType;

use finstack_quant_portfolio::optimization::{OptimizationStatus, TradeDirection, TradeSpec};

use super::super::json_bridge::{deserialize_json, serialize_json};
use super::enums::{PyTradeDirection, PyTradeType};

/// Status of an optimization run (mirrors `OptimizationStatus`).
#[pyclass(
    name = "OptimizationStatus",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyOptimizationStatus {
    pub(crate) inner: OptimizationStatus,
}

impl PyOptimizationStatus {
    pub(crate) fn from_inner(inner: OptimizationStatus) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyOptimizationStatus {
    #[classmethod]
    fn optimal(_cls: &Bound<'_, PyType>) -> Self {
        Self::from_inner(OptimizationStatus::Optimal)
    }

    #[classmethod]
    fn feasible_but_suboptimal(_cls: &Bound<'_, PyType>) -> Self {
        Self::from_inner(OptimizationStatus::FeasibleButSuboptimal)
    }

    #[classmethod]
    fn unbounded(_cls: &Bound<'_, PyType>) -> Self {
        Self::from_inner(OptimizationStatus::Unbounded)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, conflicting_constraints)")]
    fn infeasible(_cls: &Bound<'_, PyType>, conflicting_constraints: Vec<String>) -> Self {
        Self::from_inner(OptimizationStatus::Infeasible {
            conflicting_constraints,
        })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, message)")]
    fn error(_cls: &Bound<'_, PyType>, message: String) -> Self {
        Self::from_inner(OptimizationStatus::Error { message })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: OptimizationStatus = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    /// Variant tag (``"optimal"``, ``"feasible_but_suboptimal"``, ...).
    #[getter]
    fn kind(&self) -> &'static str {
        match &self.inner {
            OptimizationStatus::Optimal => "optimal",
            OptimizationStatus::FeasibleButSuboptimal => "feasible_but_suboptimal",
            OptimizationStatus::Infeasible { .. } => "infeasible",
            OptimizationStatus::Unbounded => "unbounded",
            OptimizationStatus::Error { .. } => "error",
        }
    }

    /// Whether this status represents a usable (feasible) solution.
    #[getter]
    fn is_feasible(&self) -> bool {
        self.inner.is_feasible()
    }

    /// Conflicting constraint names when ``kind == "infeasible"``, otherwise
    /// an empty list.
    #[getter]
    fn conflicting_constraints(&self) -> Vec<String> {
        match &self.inner {
            OptimizationStatus::Infeasible {
                conflicting_constraints,
            } => conflicting_constraints.clone(),
            _ => Vec::new(),
        }
    }

    /// Error message when ``kind == "error"``, otherwise ``None``.
    #[getter]
    fn message(&self) -> Option<String> {
        match &self.inner {
            OptimizationStatus::Error { message } => Some(message.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("OptimizationStatus.{}(...)", self.kind())
    }
}

/// Trade specification for a single position.
#[pyclass(
    name = "TradeSpec",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyTradeSpec {
    pub(crate) inner: TradeSpec,
}

impl PyTradeSpec {
    pub(crate) fn from_inner(inner: TradeSpec) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyTradeSpec {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: TradeSpec = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    #[getter]
    fn position_id(&self) -> String {
        self.inner.position_id.as_str().to_owned()
    }

    #[getter]
    fn instrument_id(&self) -> String {
        self.inner.instrument_id.clone()
    }

    #[getter]
    fn trade_type(&self) -> PyTradeType {
        PyTradeType {
            inner: self.inner.trade_type,
        }
    }

    #[getter]
    fn direction(&self) -> PyTradeDirection {
        PyTradeDirection {
            inner: self.inner.direction,
        }
    }

    #[getter]
    fn current_quantity(&self) -> f64 {
        self.inner.current_quantity
    }

    #[getter]
    fn target_quantity(&self) -> f64 {
        self.inner.target_quantity
    }

    #[getter]
    fn delta_quantity(&self) -> f64 {
        self.inner.delta_quantity
    }

    #[getter]
    fn current_weight(&self) -> f64 {
        self.inner.current_weight
    }

    #[getter]
    fn target_weight(&self) -> f64 {
        self.inner.target_weight
    }

    fn __repr__(&self) -> String {
        format!(
            "TradeSpec(position_id={:?}, instrument_id={:?}, direction={:?}, delta_quantity={})",
            self.inner.position_id.as_str(),
            self.inner.instrument_id,
            match self.inner.direction {
                TradeDirection::Buy => "buy",
                TradeDirection::Sell => "sell",
                TradeDirection::Hold => "hold",
            },
            self.inner.delta_quantity,
        )
    }
}

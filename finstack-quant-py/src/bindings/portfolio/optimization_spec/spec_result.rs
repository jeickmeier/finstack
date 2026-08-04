use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::PyType;

use finstack_quant_portfolio::optimization::{
    CandidatePosition, MissingMetricPolicy, OptimizationStatus, PortfolioOptimizationResult,
    PortfolioOptimizationSpec, TradeUniverse, WeightingScheme,
};

use super::super::json_bridge::{deserialize_json, serialize_json};
use super::enums::{PyMissingMetricPolicy, PyWeightingScheme};
use super::expressions::{PyConstraint, PyObjective, PyPositionFilter};
use super::status_trade::{PyOptimizationStatus, PyTradeSpec};

/// Candidate instrument that could be added to the portfolio.
///
/// Construction from Python is not yet supported (requires the instrument
/// binding bridge). The wrapper is exposed so result types and getters can
/// return it.
#[pyclass(
    name = "CandidatePosition",
    module = "finstack_quant.portfolio",
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyCandidatePosition {
    pub(crate) inner: CandidatePosition,
}

impl PyCandidatePosition {
    pub(crate) fn from_inner(inner: CandidatePosition) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCandidatePosition {
    #[getter]
    fn id(&self) -> String {
        self.inner.id.as_str().to_owned()
    }

    #[getter]
    fn entity_id(&self) -> String {
        self.inner.entity_id.as_str().to_owned()
    }

    #[getter]
    fn max_weight(&self) -> f64 {
        self.inner.max_weight
    }

    #[getter]
    fn min_weight(&self) -> f64 {
        self.inner.min_weight
    }

    /// Instrument id, taken from the underlying ``Instrument::id()``.
    #[getter]
    fn instrument_id(&self) -> String {
        self.inner.instrument.id().to_owned()
    }

    fn __repr__(&self) -> String {
        format!(
            "CandidatePosition(id={:?}, entity_id={:?}, instrument_id={:?})",
            self.inner.id.as_str(),
            self.inner.entity_id.as_str(),
            self.inner.instrument.id(),
        )
    }
}

/// Universe of tradeable existing positions and candidate additions.
///
/// Construction from Python is not yet supported (candidate instruments
/// require the instrument binding bridge). The wrapper exists so callers
/// can hold an existing universe and inspect it.
#[pyclass(
    name = "TradeUniverse",
    module = "finstack_quant.portfolio",
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyTradeUniverse {
    pub(crate) inner: TradeUniverse,
}

impl PyTradeUniverse {
    pub(crate) fn from_inner(inner: TradeUniverse) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyTradeUniverse {
    /// Universe where all existing positions are tradeable and no candidates exist.
    #[classmethod]
    fn all_positions(_cls: &Bound<'_, PyType>) -> Self {
        Self::from_inner(TradeUniverse::all_positions())
    }

    #[getter]
    fn tradeable_filter(&self) -> PyPositionFilter {
        PyPositionFilter::from_inner(self.inner.tradeable_filter.clone())
    }

    #[getter]
    fn held_filter(&self) -> Option<PyPositionFilter> {
        self.inner
            .held_filter
            .clone()
            .map(PyPositionFilter::from_inner)
    }

    #[getter]
    fn candidates(&self) -> Vec<PyCandidatePosition> {
        self.inner
            .candidates
            .iter()
            .cloned()
            .map(PyCandidatePosition::from_inner)
            .collect()
    }

    #[getter]
    fn allow_short_candidates(&self) -> bool {
        self.inner.allow_short_candidates
    }

    fn __repr__(&self) -> String {
        format!(
            "TradeUniverse(candidates={}, allow_short_candidates={})",
            self.inner.candidates.len(),
            self.inner.allow_short_candidates,
        )
    }
}

/// JSON-serializable portfolio optimization specification, mirroring the
/// Rust builder pattern.
///
/// The portfolio body is held as a ``PortfolioSpec`` JSON payload so this
/// wrapper does not depend on the larger ``PortfolioSpec`` binding (which
/// remains JSON-first elsewhere in the portfolio bindings).
#[pyclass(
    name = "PortfolioOptimizationSpec",
    module = "finstack_quant.portfolio",
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyPortfolioOptimizationSpec {
    pub(crate) inner: PortfolioOptimizationSpec,
}

impl PyPortfolioOptimizationSpec {
    pub(crate) fn from_inner(inner: PortfolioOptimizationSpec) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPortfolioOptimizationSpec {
    /// Build a spec from a portfolio JSON spec + objective. Constraints,
    /// weighting, and policy default to the Rust defaults.
    #[classmethod]
    #[pyo3(text_signature = "(cls, portfolio_spec_json, objective)")]
    fn new(
        _cls: &Bound<'_, PyType>,
        portfolio_spec_json: &str,
        objective: PyObjective,
    ) -> PyResult<Self> {
        let portfolio: finstack_quant_portfolio::portfolio::PortfolioSpec =
            deserialize_json(portfolio_spec_json)?;
        Ok(Self::from_inner(PortfolioOptimizationSpec {
            portfolio,
            objective: objective.inner,
            constraints: Vec::new(),
            weighting: WeightingScheme::ValueWeight,
            missing_metric_policy: MissingMetricPolicy::Zero,
            label: None,
        }))
    }

    /// Append a constraint (returns a new spec).
    #[pyo3(text_signature = "(self, constraint)")]
    fn with_constraint(&self, constraint: PyConstraint) -> Self {
        let mut next = self.inner.clone();
        next.constraints.push(constraint.inner);
        Self::from_inner(next)
    }

    /// Replace the objective.
    #[pyo3(text_signature = "(self, objective)")]
    fn with_objective(&self, objective: PyObjective) -> Self {
        let mut next = self.inner.clone();
        next.objective = objective.inner;
        Self::from_inner(next)
    }

    /// Replace the weighting scheme.
    #[pyo3(text_signature = "(self, weighting)")]
    fn with_weighting(&self, weighting: PyWeightingScheme) -> Self {
        let mut next = self.inner.clone();
        next.weighting = weighting.inner;
        Self::from_inner(next)
    }

    /// Replace the missing-metric policy.
    #[pyo3(text_signature = "(self, policy)")]
    fn with_missing_metric_policy(&self, policy: PyMissingMetricPolicy) -> Self {
        let mut next = self.inner.clone();
        next.missing_metric_policy = policy.inner;
        Self::from_inner(next)
    }

    /// Replace the auditability label.
    #[pyo3(text_signature = "(self, label)")]
    fn with_label(&self, label: String) -> Self {
        let mut next = self.inner.clone();
        next.label = Some(label);
        Self::from_inner(next)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: PortfolioOptimizationSpec = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    #[getter]
    fn objective(&self) -> PyObjective {
        PyObjective::from_inner(self.inner.objective.clone())
    }

    #[getter]
    fn constraints(&self) -> Vec<PyConstraint> {
        self.inner
            .constraints
            .iter()
            .cloned()
            .map(PyConstraint::from_inner)
            .collect()
    }

    #[getter]
    fn weighting(&self) -> PyWeightingScheme {
        PyWeightingScheme {
            inner: self.inner.weighting,
        }
    }

    #[getter]
    fn missing_metric_policy(&self) -> PyMissingMetricPolicy {
        PyMissingMetricPolicy {
            inner: self.inner.missing_metric_policy,
        }
    }

    #[getter]
    fn label(&self) -> Option<String> {
        self.inner.label.clone()
    }

    /// Portfolio specification body (raw JSON).
    #[pyo3(text_signature = "(self)")]
    fn portfolio_spec_json(&self) -> PyResult<String> {
        serialize_json(&self.inner.portfolio)
    }

    fn __repr__(&self) -> String {
        format!(
            "PortfolioOptimizationSpec(constraints={}, label={:?})",
            self.inner.constraints.len(),
            self.inner.label,
        )
    }
}

/// Result of an optimization run.
///
/// `PortfolioOptimizationResult` implements `Serialize` but not
/// `Deserialize` in the Rust source, so this wrapper exposes ``to_json``
/// only — there is no ``from_json``.
#[pyclass(
    name = "PortfolioOptimizationResult",
    module = "finstack_quant.portfolio"
)]
pub(super) struct PyPortfolioOptimizationResult {
    pub(crate) inner: PortfolioOptimizationResult,
}

impl PyPortfolioOptimizationResult {
    pub(crate) fn from_inner(inner: PortfolioOptimizationResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPortfolioOptimizationResult {
    /// Serialize to the canonical JSON wire format.
    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    #[getter]
    fn status(&self) -> PyOptimizationStatus {
        PyOptimizationStatus::from_inner(self.inner.status.clone())
    }

    #[getter]
    fn is_feasible(&self) -> bool {
        self.inner.status.is_feasible()
    }

    #[getter]
    fn objective_value(&self) -> f64 {
        self.inner.objective_value
    }

    #[getter]
    fn current_weights(&self) -> HashMap<String, f64> {
        self.inner
            .current_weights
            .iter()
            .map(|(k, v)| (k.as_str().to_owned(), *v))
            .collect()
    }

    #[getter]
    fn optimal_weights(&self) -> HashMap<String, f64> {
        self.inner
            .optimal_weights
            .iter()
            .map(|(k, v)| (k.as_str().to_owned(), *v))
            .collect()
    }

    #[getter]
    fn weight_deltas(&self) -> HashMap<String, f64> {
        self.inner
            .weight_deltas
            .iter()
            .map(|(k, v)| (k.as_str().to_owned(), *v))
            .collect()
    }

    #[getter]
    fn implied_quantities(&self) -> HashMap<String, f64> {
        self.inner
            .implied_quantities
            .iter()
            .map(|(k, v)| (k.as_str().to_owned(), *v))
            .collect()
    }

    #[getter]
    fn metric_values(&self) -> HashMap<String, f64> {
        self.inner
            .metric_values
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    #[getter]
    fn constraint_slacks(&self) -> HashMap<String, f64> {
        self.inner
            .constraint_slacks
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    /// Total turnover (sum of absolute weight changes).
    #[getter]
    fn turnover(&self) -> f64 {
        self.inner.turnover()
    }

    /// Trade list sorted by absolute quantity delta (largest first).
    #[pyo3(text_signature = "(self)")]
    fn to_trade_list(&self) -> Vec<PyTradeSpec> {
        self.inner
            .to_trade_list()
            .into_iter()
            .map(PyTradeSpec::from_inner)
            .collect()
    }

    /// Subset of :meth:`to_trade_list` whose ``trade_type`` is ``NewPosition``.
    #[pyo3(text_signature = "(self)")]
    fn new_position_trades(&self) -> Vec<PyTradeSpec> {
        self.inner
            .new_position_trades()
            .into_iter()
            .map(PyTradeSpec::from_inner)
            .collect()
    }

    /// Binding constraint labels and their slack values.
    #[pyo3(text_signature = "(self)")]
    fn binding_constraints(&self) -> Vec<(String, f64)> {
        self.inner
            .binding_constraints()
            .into_iter()
            .map(|(name, slack)| (name.to_owned(), slack))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "PortfolioOptimizationResult(status={:?}, objective_value={}, turnover={})",
            match &self.inner.status {
                OptimizationStatus::Optimal => "optimal",
                OptimizationStatus::FeasibleButSuboptimal => "feasible_but_suboptimal",
                OptimizationStatus::Infeasible { .. } => "infeasible",
                OptimizationStatus::Unbounded => "unbounded",
                OptimizationStatus::Error { .. } => "error",
            },
            self.inner.objective_value,
            self.inner.turnover(),
        )
    }
}

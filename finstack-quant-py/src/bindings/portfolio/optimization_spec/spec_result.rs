use std::collections::HashMap;

use indexmap::IndexSet;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyType};

use finstack_quant_portfolio::optimization::{
    CandidatePosition, MissingMetricPolicy, OptimizationStatus, PortfolioOptimizationResult,
    PortfolioOptimizationSpec, TradeUniverse, WeightingScheme,
};
use finstack_quant_portfolio::types::PositionId;

use crate::bindings::pandas_utils::{
    dict_to_dataframe, serde_rows_to_dataframe_with_schema, ColumnSchema,
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

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Parse from a JSON string.
    #[staticmethod]
    #[pyo3(text_signature = "(json_str)")]
    fn from_json(json_str: &str) -> PyResult<Self> {
        let inner: PortfolioOptimizationSpec = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    /// Serialize to JSON.
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

/// Column schema for the trade-list DataFrame, in `TradeSpec` field order.
///
/// Pinned so a solution with no trades still yields a frame carrying every
/// documented column instead of a schema-less empty frame.
const TRADE_COLUMNS: [ColumnSchema<'static>; 9] = [
    ("position_id", "str"),
    ("instrument_id", "str"),
    ("trade_type", "str"),
    ("current_quantity", "float64"),
    ("target_quantity", "float64"),
    ("delta_quantity", "float64"),
    ("direction", "str"),
    ("current_weight", "float64"),
    ("target_weight", "float64"),
];

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

    /// Solver outcome (optimal, feasible-but-suboptimal, infeasible, ...).
    #[getter]
    fn status(&self) -> PyOptimizationStatus {
        PyOptimizationStatus::from_inner(self.inner.status.clone())
    }

    /// Whether :attr:`status` represents a solution that may be consumed.
    #[getter]
    fn is_feasible(&self) -> bool {
        self.inner.status.is_feasible()
    }

    /// Value of the objective function at the solution, in the objective's own
    /// units (which depend on the configured ``Objective``).
    #[getter]
    fn objective_value(&self) -> f64 {
        self.inner.objective_value
    }

    /// Pre-trade weights by position id.
    ///
    /// Returns
    /// -------
    /// dict[str, float]
    ///     Weights are **fractions** of the portfolio, not percentages.
    #[getter]
    fn current_weights(&self) -> HashMap<String, f64> {
        self.inner
            .current_weights
            .iter()
            .map(|(k, v)| (k.as_str().to_owned(), *v))
            .collect()
    }

    /// Post-trade target weights by position id.
    ///
    /// Returns
    /// -------
    /// dict[str, float]
    ///     Fractions, not percentages. Only covers positions in the trade
    ///     universe; positions outside it implicitly keep their current weight.
    #[getter]
    fn optimal_weights(&self) -> HashMap<String, f64> {
        self.inner
            .optimal_weights
            .iter()
            .map(|(k, v)| (k.as_str().to_owned(), *v))
            .collect()
    }

    /// Weight changes ``optimal - current`` by position id, as fractions.
    #[getter]
    fn weight_deltas(&self) -> HashMap<String, f64> {
        self.inner
            .weight_deltas
            .iter()
            .map(|(k, v)| (k.as_str().to_owned(), *v))
            .collect()
    }

    /// Implied target quantities by position id.
    ///
    /// Returns
    /// -------
    /// dict[str, float]
    ///     Units / face / notional, depending on the weighting scheme — these
    ///     are quantities, not weights.
    #[getter]
    fn implied_quantities(&self) -> HashMap<String, f64> {
        self.inner
            .implied_quantities
            .iter()
            .map(|(k, v)| (k.as_str().to_owned(), *v))
            .collect()
    }

    /// Evaluated portfolio-level metric values at the solution, keyed by metric id.
    #[getter]
    fn metric_values(&self) -> HashMap<String, f64> {
        self.inner
            .metric_values
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    /// Constraint slack by constraint label.
    ///
    /// Returns
    /// -------
    /// dict[str, float]
    ///     Positive means slack remains; approximately zero means the
    ///     constraint is binding.
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

    /// Export the per-position weight and quantity solution as a pandas
    /// ``DataFrame`` indexed by position id.
    ///
    /// ``current_weights``, ``optimal_weights``, ``weight_deltas`` and
    /// ``implied_quantities`` share one position key space, so they are joined
    /// into a single frame. The row axis is the union of their keys, ordered by
    /// first appearance (``current_weights`` first); a value missing from one
    /// map becomes ``None`` in that column — this is how candidate positions
    /// with no current weight show up.
    ///
    /// Columns: ``current_weight``, ``optimal_weight``, ``weight_delta`` (all
    /// fractions of the portfolio, not percentages), ``implied_quantity``
    /// (units / face / notional per the weighting scheme). The index holds the
    /// position ids.
    #[pyo3(text_signature = "(self)")]
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let mut positions: IndexSet<&PositionId> = IndexSet::new();
        for map in [
            &self.inner.current_weights,
            &self.inner.optimal_weights,
            &self.inner.weight_deltas,
            &self.inner.implied_quantities,
        ] {
            positions.extend(map.keys());
        }
        let column = |source: &indexmap::IndexMap<PositionId, f64>| -> Vec<Option<f64>> {
            positions
                .iter()
                .map(|id| source.get(*id).copied())
                .collect()
        };
        let data = PyDict::new(py);
        data.set_item("current_weight", column(&self.inner.current_weights))?;
        data.set_item("optimal_weight", column(&self.inner.optimal_weights))?;
        data.set_item("weight_delta", column(&self.inner.weight_deltas))?;
        data.set_item("implied_quantity", column(&self.inner.implied_quantities))?;
        let index: Vec<&str> = positions.iter().map(|id| id.as_str()).collect();
        let index = PyList::new(py, index)?;
        dict_to_dataframe(py, &data, Some(index.into_any()))
    }

    /// Export :meth:`to_trade_list` as a pandas ``DataFrame``.
    ///
    /// One row per trade, in the same order as :meth:`to_trade_list` (sorted by
    /// absolute quantity delta, largest first). A solution that requires no
    /// trades yields a zero-row frame that still carries the column schema.
    ///
    /// Columns: ``position_id``, ``instrument_id``, ``trade_type``
    /// (``"existing"``, ``"new_position"``, ``"close_out"``),
    /// ``current_quantity``, ``target_quantity``, ``delta_quantity``,
    /// ``direction`` (``"buy"``, ``"sell"``, ``"hold"``), ``current_weight``,
    /// ``target_weight`` (weights are fractions, not percentages).
    #[pyo3(text_signature = "(self)")]
    fn to_trade_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let trades = self.inner.to_trade_list();
        serde_rows_to_dataframe_with_schema(py, &trades, &TRADE_COLUMNS)
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

    /// Render as an HTML table in Jupyter notebooks.
    ///
    /// Delegates to the frame from `to_dataframe`, so pandas' own row/column
    /// truncation applies and a large result stays a small repr. Returns
    /// `None` if the frame cannot be built, which makes IPython fall back to
    /// `__repr__` instead of raising from the display hook.
    fn _repr_html_(&self, py: Python<'_>) -> Option<String> {
        let frame = self.to_dataframe(py).ok()?;
        frame.call_method0("_repr_html_").ok()?.extract().ok()
    }
}

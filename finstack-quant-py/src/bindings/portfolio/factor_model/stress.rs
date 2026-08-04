use pyo3::prelude::*;
use pyo3::types::PyType;

use finstack_quant_portfolio::factor_model::{
    self as fm, StressAttribution, StressPositionEntry, StressResult, TailScenarioBreakdown,
};

use crate::bindings::date_utils::parse_iso_date_py;
use crate::bindings::extract::{extract_market_ref, extract_portfolio_ref};
use crate::errors::{core_to_py, display_to_py, portfolio_to_py};

use super::super::json_bridge::{deserialize_json, serialize_json};
use super::super::matrix_input::extract_position_pnls;
use super::contributions::PyRiskDecomposition;
use super::to_position_ids;

/// Result of a factor-stress scenario.
#[pyclass(
    name = "StressResult",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyStressResult {
    pub(crate) inner: StressResult,
}

impl PyStressResult {
    fn from_inner(inner: StressResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyStressResult {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: StressResult = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    #[getter]
    fn total_pnl(&self) -> f64 {
        self.inner.total_pnl
    }

    /// Per-position ``(position_id, pnl)`` entries.
    #[getter]
    fn position_pnl(&self) -> Vec<(String, f64)> {
        self.inner
            .position_pnl
            .iter()
            .map(|(id, pnl)| (id.as_str().to_owned(), *pnl))
            .collect()
    }

    #[getter]
    fn stressed_decomposition(&self) -> PyRiskDecomposition {
        PyRiskDecomposition::from_inner(self.inner.stressed_decomposition.clone())
    }

    fn __repr__(&self) -> String {
        format!(
            "StressResult(total_pnl={}, positions={}, stressed_total_risk={})",
            self.inner.total_pnl,
            self.inner.position_pnl.len(),
            self.inner.stressed_decomposition.total_risk,
        )
    }
}

/// Single position's contribution to tail stress.
#[pyclass(
    name = "StressPositionEntry",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyStressPositionEntry {
    pub(crate) inner: StressPositionEntry,
}

impl PyStressPositionEntry {
    fn from_inner(inner: StressPositionEntry) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyStressPositionEntry {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: StressPositionEntry = deserialize_json(json_str)?;
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
    fn avg_tail_pnl(&self) -> f64 {
        self.inner.avg_tail_pnl
    }

    #[getter]
    fn pct_of_tail_loss(&self) -> f64 {
        self.inner.pct_of_tail_loss
    }

    #[getter]
    fn worst_scenario_pnl(&self) -> f64 {
        self.inner.worst_scenario_pnl
    }

    fn __repr__(&self) -> String {
        format!(
            "StressPositionEntry(position_id={:?}, avg_tail_pnl={}, pct_of_tail_loss={}, worst_scenario_pnl={})",
            self.inner.position_id.as_str(),
            self.inner.avg_tail_pnl,
            self.inner.pct_of_tail_loss,
            self.inner.worst_scenario_pnl,
        )
    }
}

/// Breakdown of a single tail scenario.
#[pyclass(
    name = "TailScenarioBreakdown",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyTailScenarioBreakdown {
    pub(crate) inner: TailScenarioBreakdown,
}

impl PyTailScenarioBreakdown {
    fn from_inner(inner: TailScenarioBreakdown) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyTailScenarioBreakdown {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: TailScenarioBreakdown = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    #[getter]
    fn scenario_index(&self) -> usize {
        self.inner.scenario_index
    }

    #[getter]
    fn portfolio_pnl(&self) -> f64 {
        self.inner.portfolio_pnl
    }

    /// Per-position P&L for this scenario, index-aligned to
    /// :attr:`StressAttribution.position_ids` (entry ``i`` is the P&L for
    /// ``position_ids[i]``).
    #[getter]
    fn position_pnls(&self) -> Vec<f64> {
        self.inner.position_pnls.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "TailScenarioBreakdown(scenario_index={}, portfolio_pnl={}, positions={})",
            self.inner.scenario_index,
            self.inner.portfolio_pnl,
            self.inner.position_pnls.len(),
        )
    }
}

/// Per-position attribution of portfolio losses in tail scenarios.
#[pyclass(
    name = "StressAttribution",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyStressAttribution {
    pub(crate) inner: StressAttribution,
}

impl PyStressAttribution {
    fn from_inner(inner: StressAttribution) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyStressAttribution {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: StressAttribution = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    #[getter]
    fn var_threshold(&self) -> f64 {
        self.inner.var_threshold
    }

    #[getter]
    fn n_tail_scenarios(&self) -> usize {
        self.inner.n_tail_scenarios
    }

    /// Canonical position ordering shared by every entry of
    /// :attr:`tail_scenarios`. ``tail_scenarios[k].position_pnls[i]`` is the
    /// P&L for ``position_ids[i]``.
    #[getter]
    fn position_ids(&self) -> Vec<String> {
        self.inner
            .position_ids
            .iter()
            .map(|id| id.as_str().to_owned())
            .collect()
    }

    #[getter]
    fn position_contributions(&self) -> Vec<PyStressPositionEntry> {
        self.inner
            .position_contributions
            .iter()
            .cloned()
            .map(PyStressPositionEntry::from_inner)
            .collect()
    }

    #[getter]
    fn tail_scenarios(&self) -> Vec<PyTailScenarioBreakdown> {
        self.inner
            .tail_scenarios
            .iter()
            .cloned()
            .map(PyTailScenarioBreakdown::from_inner)
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "StressAttribution(var_threshold={}, n_tail_scenarios={}, position_contributions={}, tail_scenarios={})",
            self.inner.var_threshold,
            self.inner.n_tail_scenarios,
            self.inner.position_contributions.len(),
            self.inner.tail_scenarios.len(),
        )
    }
}

/// Run a factor-stress scenario and revalue the portfolio under the stressed market.
#[pyfunction]
#[pyo3(signature = (portfolio, market, factor_model_config_json, as_of, stresses))]
pub(super) fn factor_stress(
    py: Python<'_>,
    portfolio: &Bound<'_, PyAny>,
    market: &Bound<'_, PyAny>,
    factor_model_config_json: &str,
    as_of: &str,
    stresses: Vec<(String, f64)>,
) -> PyResult<PyStressResult> {
    let portfolio = extract_portfolio_ref(py, portfolio)?;
    let market = extract_market_ref(py, market)?;
    let as_of = parse_iso_date_py(as_of)?;
    let config_json = factor_model_config_json.to_owned();
    let config: finstack_quant_factor_model::FactorModelConfig = py
        .detach(move || serde_json::from_str(&config_json))
        .map_err(display_to_py)?;

    let portfolio_ref: &finstack_quant_portfolio::Portfolio = &portfolio;
    let market_ref: &finstack_quant_core::market_data::context::MarketContext = &market;
    let result = py
        .detach(move || {
            let stresses = stresses
                .into_iter()
                .map(|(factor_id, shift)| {
                    (finstack_quant_factor_model::FactorId::new(factor_id), shift)
                })
                .collect::<Vec<_>>();
            let model = fm::FactorModelBuilder::new().config(config).build()?;
            model.factor_stress(portfolio_ref, market_ref, as_of, &stresses)
        })
        .map_err(portfolio_to_py)?;

    Ok(PyStressResult::from_inner(result))
}

/// Build tail-scenario stress attribution from position x scenario P&Ls.
///
/// Python input is one row per position, where every row contains that
/// position's P&L across all scenarios. The binding transposes that ergonomic
/// shape into Rust's row-major scenario x position buffer.
#[pyfunction]
#[pyo3(signature = (position_ids, position_pnls, confidence = 0.95))]
pub(super) fn build_stress_attribution(
    py: Python<'_>,
    position_ids: Vec<String>,
    position_pnls: &Bound<'_, PyAny>,
    confidence: f64,
) -> PyResult<PyStressAttribution> {
    let n_positions = position_ids.len();
    let position_pnls = extract_position_pnls(py, position_pnls, n_positions)?;
    let n_scenarios = position_pnls.n_scenarios();
    let result = py
        .detach(move || {
            let ids = to_position_ids(position_ids);
            let flat = position_pnls.into_scenario_major(n_positions);
            fm::build_stress_attribution(&ids, &flat, n_scenarios, confidence)
        })
        .map_err(core_to_py)?;
    Ok(PyStressAttribution::from_inner(result))
}
